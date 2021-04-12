use std::env;
use std::net;
use std::process;
use std::thread::*;
use std::time;
use elevator::*;

use crossbeam_channel as cbc;
use serde;

use elevio::elev as e;
use fsm::door_timer;
use fsm::elevatorfsm::Event;
use network::global_elevator::GlobalElevatorInfo;

// Data types to be sent on the network must derive traits for serialization

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct order_t {
    node: String,
    floor: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct CustomDataType {
    message: String,
    order: order_t,
    iteration: u64,
}

pub const DOOR_OPEN_TIME: u64 = 3;

fn main() -> std::io::Result<()> {
    /* ------------------NETWORK----------------- */
    // Genreate id: either from command line, or a default rust@ip#pid
    let args: Vec<String> = env::args().collect();
    let id = if args.len() > 1 {
        args[1].clone()
    } else {
        let local_ip = net::TcpStream::connect("8.8.8.8:53")
            .unwrap()
            .local_addr()
            .unwrap()
            .ip();
        format!("rust@{}#{}", local_ip, process::id())
    };

    let msg_port = 19747;
    let peer_port = 19738;

    // The sender for peer discovery
    let (peer_tx_enable_tx, peer_tx_enable_rx) = cbc::unbounded::<bool>();
    {
        let id = id.clone();
        spawn(move || {
            network::peers::tx(peer_port, id, peer_tx_enable_rx);
        });
    }
    // (periodically disable/enable the peer broadcast, to provoke new peer / peer loss messages)
    spawn(move || loop {
        sleep(time::Duration::new(6, 0));
        peer_tx_enable_tx.send(false).unwrap();
        sleep(time::Duration::new(3, 0));
        peer_tx_enable_tx.send(true).unwrap();
    });
    // The receiver for peer discovery updates
    let (peer_update_tx, peer_update_rx) = cbc::unbounded::<network::peers::PeerUpdate>();
    spawn(move || {
        network::peers::rx(peer_port, peer_update_tx);
    });
    
    // Periodically produce a custom data message
    let (custom_data_send_tx, custom_data_send_rx) = cbc::unbounded::<CustomDataType>();
    {
        let id = id.clone();
        spawn(move || {
            let new_order = order_t {
                node: "192.168.1.1".to_string(),
                floor: 2,
            };
            let mut cd = CustomDataType {
                message: format!("Hello from node {}", id),
                order: new_order,
                iteration: 0,
            };
            loop {
                custom_data_send_tx.send(cd.clone()).unwrap();
                cd.iteration += 1;
                sleep(time::Duration::new(1, 0));
            }
        });
    }
    // The sender for our custom data
    spawn(move || {
        network::bcast::tx(msg_port, custom_data_send_rx);
    });
    // The receiver for our custom data
    let (custom_data_recv_tx, custom_data_recv_rx) = cbc::unbounded::<CustomDataType>();
    spawn(move || {
        network::bcast::rx(msg_port, custom_data_recv_tx);
    });

    /*----------------SINGLE ELEVATOR---------------------*/
    let elev_num_floors = 4;
    let elevator = e::ElevatorHW::init("localhost:15657", elev_num_floors)?;
    println!("Elevator started:\n{:#?}", elevator);

    let (hardware_command_tx, hardware_command_rx) =
        cbc::unbounded::<elevio::elev::HardwareCommand>();
    let (door_timer_start_tx, door_timer_start_rx) = cbc::unbounded::<door_timer::TimerCommand>();
    let (door_timeout_tx, door_timeout_rx) = cbc::unbounded::<()>();
    /* Thread that keeps track of the door timer */
    spawn(move || {
        let mut door_timer: door_timer::Timer = door_timer::Timer::new(DOOR_OPEN_TIME);
        loop {
            let r = door_timer_start_rx.try_recv();
            match r {
                Ok(r) => door_timer.on_command(r),
                _ => {}
            }
            if door_timer.did_expire() {
                door_timeout_tx.send(()).unwrap();
            }
        }
    });

    /* Spawn a thread that executes elevator commands sent from fsm */
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

    let mut fsm =
        fsm::elevatorfsm::Elevator::new(elev_num_floors, 0, hardware_command_tx.clone(), door_timer_start_tx);
    
    // Global elevator info manager
    let (elevator_info_tx, elevator_info_rx) = cbc::unbounded::<fsm::elevatorfsm::ElevatorInfo>();
    let (remote_update_tx, remote_update_rx) = cbc::unbounded::<Vec<fsm::elevatorfsm::ElevatorInfo>>();
    let (global_info_tx, global_info_rx) = cbc::unbounded::<GlobalElevatorInfo>();
    {
        let elev_info_init = fsm.get_info();
        spawn(move || network::global_elevator::global_elevator_info(elev_info_init, 10, elevator_info_rx, remote_update_rx, global_info_tx));
    }

    // Thread that 'does something' with the global elevator info received
    {
        spawn(move || {
            let mut old_lights: order_manager::order_list::OrderList = order_manager::order_list::OrderList::new(elev_num_floors);
            loop {
                cbc::select! {
                    recv(global_info_rx) -> a => {
                        let global_info = a.unwrap();
                        let set_lights = global_info.get_orders_for_lights();
                        for f in 0..elev_num_floors {
                            for c in 0..3 {
                                let btn = elevio::poll::CallButton{floor: f, call: c};
                                if old_lights.is_active(btn) != set_lights.is_active(btn) {
                                    hardware_command_tx.send(
                                        elevio::elev::HardwareCommand::CallButtonLight{floor:btn.floor, call: btn.call, on: set_lights.is_active(btn)}).unwrap();
                                }
                            }
                        }
                        old_lights = set_lights;
                    },
                }
            }
    });
    }

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

    loop {
        cbc::select! {
            recv(peer_update_rx) -> a => {
                let update = a.unwrap();
                println!("{:#?}", update);
            }
            recv(custom_data_recv_rx) -> a => {
                let cd = a.unwrap();
                println!("{:#?}", cd);
            },
            recv(call_button_rx) -> a => {
                let call_button = a.unwrap();
                println!("{:#?}", call_button);
                fsm.on_event(Event::OnNewOrder{btn: call_button});
                elevator_info_tx.send(fsm.get_info()).unwrap();
            },
            recv(floor_sensor_rx) -> a => {
                let floor = a.unwrap();
                fsm.on_event(Event::OnFloorArrival{floor: floor});
                println!("Floor: {:#?}", floor);
                elevator_info_tx.send(fsm.get_info()).unwrap();

            },
            recv(stop_button_rx) -> a => {
                let _stop = a.unwrap();
                // This elevator doesn't care about stopping
                elevator_info_tx.send(fsm.get_info()).unwrap();
            },
            recv(obstruction_rx) -> a => {
                let obstr = a.unwrap();
                fsm.on_event(Event::OnObstructionSignal{active: obstr});
                elevator_info_tx.send(fsm.get_info()).unwrap();
            },
            recv(door_timeout_rx) -> a => {
                a.unwrap();
                fsm.on_event(Event::OnDoorTimeOut);
                elevator_info_tx.send(fsm.get_info()).unwrap();
            },
        }
    }
}

