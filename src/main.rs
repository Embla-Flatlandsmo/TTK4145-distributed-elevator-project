use std::env;
use std::thread::*;
use std::time;
use elevator::*;
use crossbeam_channel as cbc;

use util::constants as setting;

use local_elevator::elevio::elev as e;
use local_elevator::fsm::door_timer;
use local_elevator::fsm::elevatorfsm::{Elevator, Event, ElevatorInfo, State};
use global_elevator_info::connected_elevators::ConnectedElevatorInfo;
use local_elevator::elevio::poll::CallButton;

fn main() -> std::io::Result<()> {
    if setting::ID > setting::MAX_NUM_ELEV {
        panic!("Trying to start an elevator with an ID that is too high. Consider increasing MAX_NUM_ELEV in util/constants.rs");
    }

    println!("Elevator started with local ID: {}", setting::ID);
    // To run on a simulator port, call "cargo run PORT_TO_RUN_ON"
    let args: Vec<String> = env::args().collect();
    let mut server_port: String = "15657".to_string();
    if args.len() > 1 {
        server_port = args[1].clone();
    }
    let elev_hw_server: String = format!("{}:{}", "localhost", server_port);

    // We must wait to assure that, in the case of software restart, the system is registered as lost:
    std::thread::sleep(std::time::Duration::from_millis(setting::TIME_UNTIL_PEER_LOST_MILLISEC+500));

    /*--------------------SINGLE ELEVATOR---------------------*/
    let elevator = e::ElevatorHW::init(&elev_hw_server[..], setting::ELEV_NUM_FLOORS)?;
    println!("Elevator started:\n{:#?}", elevator);

    /* Initialization of hardware polling */
    let poll_period = time::Duration::from_millis(25);
    let (call_button_tx, call_button_rx) = cbc::unbounded::<CallButton>();
    {
        let elevator = elevator.clone();
        spawn(move || local_elevator::elevio::poll::call_buttons(elevator, call_button_tx, poll_period));
    }

    let (floor_sensor_tx, floor_sensor_rx) = cbc::unbounded::<u8>();
    {
        let elevator = elevator.clone();
        spawn(move || local_elevator::elevio::poll::floor_sensor(elevator, floor_sensor_tx, poll_period));
    }

    let (stop_button_tx, stop_button_rx) = cbc::unbounded::<bool>();
    {
        let elevator = elevator.clone();
        spawn(move || local_elevator::elevio::poll::stop_button(elevator, stop_button_tx, poll_period));
    }

    let (obstruction_tx, obstruction_rx) = cbc::unbounded::<bool>();
    {
        let elevator = elevator.clone();
        spawn(move || local_elevator::elevio::poll::obstruction(elevator, obstruction_tx, poll_period));
    }    

    /* Thread that keeps track of the local elevator door timer */
    let (door_timer_start_tx, door_timer_start_rx) = cbc::unbounded::<door_timer::TimerCommand>();
    let (door_timeout_tx, door_timeout_rx) = cbc::unbounded::<()>();
    spawn(move || {
        door_timer::run(door_timer_start_rx, door_timeout_tx);
    });

    /* Initialization of the local elevator fsm */
    let (hardware_command_tx, hardware_command_rx) = cbc::unbounded::<e::HardwareCommand>();
    let (state_updater_tx, state_updater_rx) = cbc::unbounded::<State>();
    let mut fsm = Elevator::new(hardware_command_tx.clone(), door_timer_start_tx, state_updater_tx);
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
    let (local_info_for_global_tx, local_info_for_global_rx) = cbc::unbounded::<local_elevator::fsm::elevatorfsm::ElevatorInfo>();
    let (remote_update_tx, remote_update_rx) = cbc::unbounded::<Vec<local_elevator::fsm::elevatorfsm::ElevatorInfo>>();
    let (connected_info_tx, connected_info_rx) = cbc::unbounded::<ConnectedElevatorInfo>();
    let (connected_info_for_assigner_tx,connected_info_for_assigner_rx) = cbc::unbounded::<ConnectedElevatorInfo>();
    let (connected_info_for_lights_tx, connected_info_for_lights_rx) = cbc::unbounded::<ConnectedElevatorInfo>();
    let (set_pending_tx, set_pending_rx) = cbc::unbounded::<(bool,usize,CallButton)>();
    {
        let alc_tx = assign_orders_locally_tx.clone();
        spawn(move || global_elevator_info::connected_elevators::connected_elevator_info(local_info_for_global_rx, remote_update_rx, set_pending_rx, connected_info_tx, alc_tx));
    }
    local_info_for_global_tx.send(fsm.get_info()).unwrap();
    
    {
        let order_lights_tx = hardware_command_tx.clone();
        spawn(move || global_elevator_info::connected_elevators::set_order_lights(connected_info_for_lights_rx, order_lights_tx));
    }
    

    /*--------------------NETWORK MESSAGE HANDLERS--------------------*/
    /* The sender for peer discovery */
    let (_peer_tx_enable_tx, peer_tx_enable_rx) = cbc::unbounded::<bool>();
    /* Transmit local elevator info on network */
    let (local_elev_info_to_transmit_tx, local_elev_info_to_transmit_rx) = cbc::unbounded::<ElevatorInfo>();
    spawn(move || 
            global_elevator_info::elev_info_updater::local_elev_info_tx::<ElevatorInfo>(local_elev_info_to_transmit_rx, peer_tx_enable_rx)
        );
    local_elev_info_to_transmit_tx.send(fsm.get_info()).unwrap();

    /* Receive elevator info from remote elevators */
    let (backup_cab_order_transmitter_tx, backup_cab_order_transmitter_rx) = cbc::unbounded::<ElevatorInfo>();
    spawn(move || 
        global_elevator_info::elev_info_updater::remote_elev_info_rx::<Vec<ElevatorInfo>>(remote_update_tx, backup_cab_order_transmitter_tx)
    );


    /* Transmit and receive orders from other elevators */
    {
        let set_pending_transmitter = set_pending_tx.clone();
        let local_order_assign_tx = assign_orders_locally_tx.clone();
        spawn(move || order_assigner::order_transmitter::hall_order_transmitter(connected_info_for_assigner_rx, call_button_rx, set_pending_transmitter, local_order_assign_tx));
    }

    {
        let local_order_assign_tx = assign_orders_locally_tx.clone();
        spawn(move || order_assigner::order_receiver::hall_order_receiver(local_order_assign_tx));
    }


    /*------------------CAB ORDER BACKUP---------------*/

    /* Receive cab_order backup from remote elevators */

    spawn(move || 
        order_assigner::order_receiver::cab_order_backup_rx::<Vec<ElevatorInfo>>(assign_orders_locally_tx)
    );

    /*Transmit cab_order_backup to network*/
    spawn(move || 
        order_assigner::order_transmitter::cab_order_backup_tx::<ElevatorInfo>(backup_cab_order_transmitter_rx)
    );




    /*--------------------UTILITY---------------------*/
    // Forwarding messages to the appropriate channels (they need same info, but shouldn't steal messages from one another)
    spawn(move || {
        loop {
            cbc::select!{
                recv(connected_info_rx) -> a => {
                    let glob_info = a.unwrap();
                    connected_info_for_assigner_tx.send(glob_info.clone()).unwrap();
                    connected_info_for_lights_tx.send(glob_info.clone()).unwrap();

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
    
    let (elev_timeout_tx, elev_timeout_rx) = cbc::unbounded::<()>();
    spawn(move || local_elevator::fsm::elevatorfsm::state_timeout_checker(state_updater_rx, elev_timeout_tx));
    
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
            recv(door_timeout_rx) -> _a => {
                fsm.on_event(Event::OnDoorTimeOut);
                local_elev_info_tx.send(fsm.get_info()).unwrap();
            },
            recv(elev_timeout_rx) -> _a => {
                fsm.on_event(Event::OnStateTimeOut);
                local_elev_info_tx.send(fsm.get_info()).unwrap();
            }
        }
    }
}

