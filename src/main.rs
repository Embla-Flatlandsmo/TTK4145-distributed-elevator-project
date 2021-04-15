use std::env;
use std::thread::*;
use std::time;
use elevator::*;
use crossbeam_channel as cbc;

use util::constants as setting;

use elevio::elev as e;
use fsm::door_timer;
use fsm::elevatorfsm::Event;
use network::global_elevator::GlobalElevatorInfo;
use elevio::poll::CallButton;

use order_assigner::order_assigner;
use crate::fsm::elevatorfsm::ElevatorInfo;

fn main() -> std::io::Result<()> {
    // To run on a simulator port, call "cargo run PORT_TO_RUN_ON"
    let args: Vec<String> = env::args().collect();
    let mut server_port: String = "15657".to_string();
    if args.len() > 1 {
        server_port = args[1].clone();
    }
    let elev_hw_server: String = format!("{}:{}", "localhost", server_port);


    /*--------------------SINGLE ELEVATOR---------------------*/
    let elevator = e::ElevatorHW::init(&elev_hw_server[..], setting::ELEV_NUM_FLOORS)?;
    println!("Elevator started:\n{:#?}", elevator);

    /* Initialization of hardware polling */
    let poll_period = time::Duration::from_millis(25);
    let (call_button_tx, call_button_rx) = cbc::unbounded::<elevio::poll::CallButton>();
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::call_buttons(elevator, call_button_tx, poll_period));
    }

    let (floor_sensor_tx, floor_sensor_rx) = cbc::unbounded::<u8>();
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::floor_sensor(elevator, floor_sensor_tx, poll_period));
    }

    let (stop_button_tx, stop_button_rx) = cbc::unbounded::<bool>();
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::stop_button(elevator, stop_button_tx, poll_period));
    }

    let (obstruction_tx, obstruction_rx) = cbc::unbounded::<bool>();
    {
        let elevator = elevator.clone();
        spawn(move || elevio::poll::obstruction(elevator, obstruction_tx, poll_period));
    }    

    /* Thread that keeps track of the local elevator door timer */
    let (door_timer_start_tx, door_timer_start_rx) = cbc::unbounded::<door_timer::TimerCommand>();
    let (door_timeout_tx, door_timeout_rx) = cbc::unbounded::<()>();
    spawn(move || {
        door_timer::run(door_timer_start_rx, door_timeout_tx);
    });

    /* Initialization of the local elevator fsm */
    let (hardware_command_tx, hardware_command_rx) = cbc::unbounded::<elevio::elev::HardwareCommand>();
    let mut fsm =
        fsm::elevatorfsm::Elevator::new(setting::ELEV_NUM_FLOORS, setting::ID, hardware_command_tx.clone(), door_timer_start_tx);
    let (local_elev_info_tx, local_elev_info_rx) = cbc::unbounded::<ElevatorInfo>();
    let (assign_orders_locally_tx, assign_orders_locally_rx) = cbc::unbounded::<CallButton>();

    /* Execute elevator commands sent from fsm */
    {
        let elevator = elevator.clone();
        spawn(move || loop {
            let r = hardware_command_rx.recv();
            match r {
                Ok(c) => elevator.execute_command(c),
                Err(_e) => {}
            }
        });
    }

    // Global elevator info manager
    let (local_info_for_global_tx, local_info_for_global_rx) = cbc::unbounded::<fsm::elevatorfsm::ElevatorInfo>();
    let (remote_update_tx, remote_update_rx) = cbc::unbounded::<Vec<fsm::elevatorfsm::ElevatorInfo>>();
    let (global_info_tx, global_info_rx) = cbc::unbounded::<GlobalElevatorInfo>();
    let (global_info_for_assigner_tx,global_info_for_assigner_rx) = cbc::unbounded::<GlobalElevatorInfo>();
    let (global_info_for_lights_tx, global_info_for_lights_rx) = cbc::unbounded::<GlobalElevatorInfo>();
    let (set_pending_tx, set_pending_rx) = cbc::unbounded::<(bool,usize,CallButton)>();
    {
        let alc_tx = assign_orders_locally_tx.clone();
        spawn(move || network::global_elevator::global_elevator_info(local_info_for_global_rx, remote_update_rx, set_pending_rx, global_info_tx, alc_tx));
    }
    local_info_for_global_tx.send(fsm.get_info()).unwrap();
    
    {
        let order_lights_tx = hardware_command_tx.clone();
        spawn(move || network::global_elevator::set_order_lights(global_info_for_lights_rx, order_lights_tx));
    }
    

    /*--------------------NETWORK MESSAGE HANDLERS--------------------*/
    /* The sender for peer discovery */
    let (peer_tx_enable_tx, peer_tx_enable_rx) = cbc::unbounded::<bool>();
    /* Transmit local elevator info on network */
    let (local_elev_info_to_transmit_tx, local_elev_info_to_transmit_rx) = cbc::unbounded::<ElevatorInfo>();
    spawn(move || 
            network::remote_elevator::local_elev_info_tx::<ElevatorInfo>(local_elev_info_to_transmit_rx, peer_tx_enable_rx)
        );
    local_elev_info_to_transmit_tx.send(fsm.get_info()).unwrap();

    // (periodically disable/enable the peer broadcast, to provoke new peer / peer loss messages)
    /*spawn(move || loop {
        sleep(time::Duration::new(6, 0));
        peer_tx_enable_tx.send(false).unwrap();
        sleep(time::Duration::new(3, 0));
        peer_tx_enable_tx.send(true).unwrap();
    });*/

    /* Receive elevator info from remote elevators */
    spawn(move || 
        network::remote_elevator::remote_elev_info_rx::<Vec<ElevatorInfo>>(setting::PEER_PORT, remote_update_tx, cab_order_transmitter_tx)
    );


    /* Transmit and receive orders from other elevators */
    {
        let set_pending_transmitter = set_pending_tx.clone();
        let local_order_assign_tx = assign_orders_locally_tx.clone();
        spawn(move || order_assigner::order_transmitter(global_info_for_assigner_rx, call_button_rx, set_pending_transmitter, local_order_assign_tx));
    }

    {
        spawn(move || order_assigner::order_receiver(assign_orders_locally_tx, set_pending_tx));
    }



    /*------------------CAB ORDER BACKUP---------------*/

    /* Receive cab_order backup from remote elevators */
    let (cab_order_receiver_tx, cab_order_receiver_rx) = cbc::unbounded::<ElevatorInfo>();
    spawn(move || 
        network::remote_elevator::cab_order_backup_rx::<Vec<ElevatorInfo>>(setting::CAB_BACKUP_PORT, cab_order_receiver_tx)
    );

    /*Transmit cab_order_backup to network*/
    let (cab_order_transmitter_tx, cab_order_transmitter_rx) = cbc::unbounded::<ElevatorInfo>();
    spawn(move || 
        network::remote_elevator::cab_order_backup_tx::<ElevatorInfo>(cab_order_transmitter_rx)
    );




    /*--------------------UTILITY---------------------*/
    // Forwarding messages to the appropriate channels (they need same info, but shouldn't steal messages from one another)
    spawn(move || {
        loop {
            cbc::select!{
                recv(global_info_rx) -> a => {
                    let glob_info = a.unwrap();
                    global_info_for_assigner_tx.send(glob_info.clone()).unwrap();
                    global_info_for_lights_tx.send(glob_info.clone()).unwrap();

                },
                recv(local_elev_info_rx) -> a => {
                    let local_info = a.unwrap();
                    local_info_for_global_tx.send(local_info.clone()).unwrap();
                    local_elev_info_to_transmit_tx.send(local_info.clone()).unwrap();
                }
            }
        }
        });

    /*----------------LOOP FOR LOCAL ELEVATOR INPUT---------------------*/
    loop {
        cbc::select! {
            recv(assign_orders_locally_rx) -> a => {
                let call_button = a.unwrap();
                println!("{:#?}", call_button);
                fsm.on_event(Event::OnNewOrder{btn: call_button});
                local_elev_info_tx.send(fsm.get_info()).unwrap();         
            },
            recv(floor_sensor_rx) -> a => {
                let floor = a.unwrap();
                fsm.on_event(Event::OnFloorArrival{floor: floor});
                println!("Floor: {:#?}", floor);
                local_elev_info_tx.send(fsm.get_info()).unwrap();
            },
            recv(stop_button_rx) -> a => {
                let _stop = a.unwrap();
                local_elev_info_tx.send(fsm.get_info()).unwrap();
            },
            recv(obstruction_rx) -> a => {
                let obstr = a.unwrap();
                fsm.on_event(Event::OnObstructionSignal{active: obstr});
                local_elev_info_tx.send(fsm.get_info()).unwrap();
            },
            recv(door_timeout_rx) -> a => {
                a.unwrap();
                fsm.on_event(Event::OnDoorTimeOut);
                local_elev_info_tx.send(fsm.get_info()).unwrap();
            },
        }
    }
}

