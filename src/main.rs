use std::env;
use std::net;
use std::process;
use std::thread::*;
use std::time;
use elevator::*;

use crossbeam_channel as cbc;
use serde;

use util::constants as setting;

use elevio::elev as e;
use fsm::door_timer;
use fsm::elevatorfsm::Event;
use network::global_elevator::GlobalElevatorInfo;
use elevio::poll::CallButton;

use order_assigner::order_assigner;
use crate::fsm::elevatorfsm::ElevatorInfo;

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



fn main() -> std::io::Result<()> {



    /*----------------SINGLE ELEVATOR---------------------*/
    let elevator = e::ElevatorHW::init("localhost:15657", setting::ELEV_NUM_FLOORS)?;
    println!("Elevator started:\n{:#?}", elevator);

    let (hardware_command_tx, hardware_command_rx) =
        cbc::unbounded::<elevio::elev::HardwareCommand>();
    let (door_timer_start_tx, door_timer_start_rx) = cbc::unbounded::<door_timer::TimerCommand>();
    let (door_timeout_tx, door_timeout_rx) = cbc::unbounded::<()>();
    /* Thread that keeps track of the door timer */
    spawn(move || {
        let mut door_timer: door_timer::Timer = door_timer::Timer::new(setting::DOOR_OPEN_TIME);
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
        fsm::elevatorfsm::Elevator::new(setting::ELEV_NUM_FLOORS, setting::ID, hardware_command_tx.clone(), door_timer_start_tx);
    
    // Global elevator info manager
    let (local_info_for_global_tx, local_info_for_global_rx) = cbc::unbounded::<fsm::elevatorfsm::ElevatorInfo>();
    let (remote_update_tx, remote_update_rx) = cbc::unbounded::<Vec<fsm::elevatorfsm::ElevatorInfo>>();
    let (global_info_tx, global_info_rx) = cbc::unbounded::<GlobalElevatorInfo>();
    let (global_info_for_assigner_tx,global_info_for_assigner_rx) = cbc::unbounded::<GlobalElevatorInfo>();
    let (global_info_for_lights_tx, global_info_for_lights_rx) = cbc::unbounded::<GlobalElevatorInfo>();
    let (set_pending_tx, set_pending_rx) = cbc::unbounded::<(usize,CallButton)>();
    {
        spawn(move || network::global_elevator::global_elevator_info(local_info_for_global_rx, remote_update_rx, set_pending_rx, global_info_tx));
    }
    local_info_for_global_tx.send(fsm.get_info());
    /*
    {
        let order_lights_tx = hardware_command_tx.clone();
        spawn(move || network::global_elevator::set_order_lights(global_info_for_lights_rx, order_lights_tx));
    }
    */
    let (assign_orders_locally_tx, assign_orders_locally_rx) = cbc::unbounded::<CallButton>();
    {
        spawn(move || {
            let mut old_lights: order_manager::order_list::OrderList = order_manager::order_list::OrderList::new(setting::ELEV_NUM_FLOORS);
            loop {
                cbc::select! {
                    recv(global_info_rx) -> a => {
                        //forward global info...
                        let global_info = a.unwrap();
                        let set_lights = global_info.get_orders_for_lights();
                        for f in 0..setting::ELEV_NUM_FLOORS {
                            for c in 0..3 {
                                let btn = elevio::poll::CallButton{floor: f, call: c};
                                if old_lights.is_active(btn) != set_lights.is_active(btn) {
                                    hardware_command_tx.send(
                                        elevio::elev::HardwareCommand::CallButtonLight{floor:btn.floor, call: btn.call, on: set_lights.is_active(btn)}).unwrap();
                                }
                            }
                        }
                        old_lights = set_lights;
                        //global_info_for_lights_tx.send(global_info.clone()).unwrap();
                        global_info_for_assigner_tx.send(global_info.clone()).unwrap();
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

    /*----------------METWWORK---------------------*/

    // The sender for orders
    let (order_send_tx, order_send_rx) = cbc::unbounded::<(usize, CallButton)>();
    spawn(move || {
        network::bcast::tx(setting::ORDER_PORT, order_send_rx,3);
    });
    // The reciever for orders
    let (order_recv_tx, order_recv_rx) = cbc::unbounded::<(usize, CallButton)>();
    spawn(move || {
        network::bcast::rx(setting::ORDER_PORT, order_recv_tx);
    });

    // The sender for peer discovery
    let (peer_tx_enable_tx, peer_tx_enable_rx) = cbc::unbounded::<bool>();
    let (elevator_info_tx, elevator_info_rx) = cbc::unbounded::<ElevatorInfo>();
    {
        //let id = id.clone();
        spawn(move || {
            network::remote_elevator::local_elev_info_tx::<ElevatorInfo>(setting::PEER_PORT, elevator_info_rx, peer_tx_enable_rx);
        });
    }
    elevator_info_tx.send(fsm.get_info());
    // (periodically disable/enable the peer broadcast, to provoke new peer / peer loss messages)
    /*spawn(move || loop {
        sleep(time::Duration::new(6, 0));
        peer_tx_enable_tx.send(false).unwrap();
        sleep(time::Duration::new(3, 0));
        peer_tx_enable_tx.send(true).unwrap();
    });*/
    // The receiver for peer discovery updates
    let (peer_update_tx, peer_update_rx) = cbc::unbounded::<Vec<ElevatorInfo>>();
    spawn(move || {
        network::remote_elevator::remote_elev_info_rx::<Vec<ElevatorInfo>>(setting::PEER_PORT, remote_update_tx);
    });

    {
        let set_pending_transmitter = set_pending_tx.clone();
        spawn(move || order_assigner::order_assigner(global_info_for_assigner_rx, call_button_rx, set_pending_transmitter, order_send_tx, assign_orders_locally_tx));
    }

    loop {
        cbc::select! {
            recv(order_recv_rx) -> a => {
                let order = a.unwrap();
                let id = order.0;
                let call_button = order.1;
                if id == setting::ID {
                    fsm.on_event(Event::OnNewOrder{btn: call_button});
                    elevator_info_tx.send(fsm.get_info()).unwrap();
                    local_info_for_global_tx.send(fsm.get_info()).unwrap();
                } else {
                    set_pending_tx.send((id, call_button));
                }
            },
            recv(assign_orders_locally_rx) -> a => {
                let call_button = a.unwrap();
                println!("{:#?}", call_button);
                fsm.on_event(Event::OnNewOrder{btn: call_button});
                elevator_info_tx.send(fsm.get_info()).unwrap();
                local_info_for_global_tx.send(fsm.get_info()).unwrap();           
            },
            recv(floor_sensor_rx) -> a => {
                let floor = a.unwrap();
                fsm.on_event(Event::OnFloorArrival{floor: floor});
                println!("Floor: {:#?}", floor);
                elevator_info_tx.send(fsm.get_info()).unwrap();
                local_info_for_global_tx.send(fsm.get_info()).unwrap();
            },
            recv(stop_button_rx) -> a => {
                let _stop = a.unwrap();
                elevator_info_tx.send(fsm.get_info()).unwrap();
                local_info_for_global_tx.send(fsm.get_info()).unwrap();
                // This elevator doesn't care about stopping
            },
            recv(obstruction_rx) -> a => {
                let obstr = a.unwrap();
                fsm.on_event(Event::OnObstructionSignal{active: obstr});
                elevator_info_tx.send(fsm.get_info()).unwrap();
                local_info_for_global_tx.send(fsm.get_info()).unwrap();
            },
            recv(door_timeout_rx) -> a => {
                a.unwrap();
                fsm.on_event(Event::OnDoorTimeOut);
                elevator_info_tx.send(fsm.get_info()).unwrap();
                local_info_for_global_tx.send(fsm.get_info()).unwrap();
            },
        }
    }
}

