#![allow(dead_code)]
use crate::elevio::elev as elevio;
use crate::elevio::poll;
use crate::order_manager::local_order_manager;
use crate::order_manager::order_list;
use std::time::Duration;
use crate::fsm::door_timer;

use crossbeam_channel as cbc;
extern crate timer;

/// Contains all we need to know about our elevator.
/// * `hw_tx` the transmitter for sending hardware commands
/// * `state` what the elevator is currently doing
/// * `dirn` the direction that the elevator last moved in (for direction conservation when picking where to go next)
/// * `floor` the last floor the elevator was at
/// * `orders` list of orders that the elevator will service
#[derive(Clone, Debug)]
pub struct Elevator {
    hw_tx: crossbeam_channel::Sender<elevio::HardwareCommand>,
    timer_start_tx: cbc::Sender<()>,
    state: State,
    dirn: u8,
    floor: u8,
    orders: order_list::OrderList,
}

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Initializing,
    DoorOpen,
    Idle,
    Stopped, //Todo
    Moving,
    Failure(String),
}

#[derive(Debug)]
pub enum Event {
    OnDoorTimeOut,
    OnFloorArrival { floor: u8 },
    OnNewOrder { btn: poll::CallButton },
    OnObstructionSignal,
}

pub const DOOR_OPEN_TIME: u64 = 3;

impl Elevator {
    pub fn new(n_floors: u8, hw_commander: cbc::Sender<elevio::HardwareCommand>, timer_start_tx: cbc::Sender<()>) -> Elevator {
        hw_commander
            .send(elevio::HardwareCommand::MotorDirection {
                dirn: elevio::DIRN_DOWN,
            })
            .unwrap();
        return Elevator {
            hw_tx: hw_commander,
            timer_start_tx: timer_start_tx,
            state: State::Initializing,
            dirn: elevio::DIRN_DOWN,
            floor: u8::MAX,
            orders: order_list::OrderList::new(n_floors),
        };
    }
    /// Takes the elevator fsm from one state to the next and sends the appropriate hardware commands on the hardware channel
    pub fn on_event(mut self, event: Event) {
        let from_state = self.get_state();
        match (from_state, event) {
            /* Todo: Find out how to make this a bit more concise. The functions themselves have some redundancies here... */
            (State::Idle, Event::OnNewOrder { btn }) => on_new_order(self, btn),
            (State::DoorOpen, Event::OnNewOrder { btn }) => on_new_order(self, btn),
            (State::Moving, Event::OnNewOrder { btn }) => on_new_order(self, btn),
            (State::DoorOpen, Event::OnDoorTimeOut) => on_door_time_out(self),
            (State::DoorOpen, Event::OnObstructionSignal) => on_obstruction_signal(self),
            (State::Moving, Event::OnFloorArrival { floor }) => on_floor_arrival(self, floor),
            (State::Initializing, Event::OnFloorArrival { floor }) => on_floor_arrival(self, floor),
            (State::Initializing, Event::OnNewOrder { btn }) => {},
            (s, e) => self.state = State::Failure(
                format!("Wrong state, event combination: {:#?} {:#?}", s, e).to_string(),
            )
            }
        }

    pub fn get_floor(&self) -> u8 {
        return self.floor;
    }
    pub fn get_state(&self) -> State {
        return self.state.clone();
    }
    pub fn get_dirn(&self) -> u8 {
        return self.dirn;
    }
    pub fn get_hw_tx_handle(&self) -> cbc::Sender<elevio::HardwareCommand> {
        return self.hw_tx.clone();
    }
    pub fn get_orders(&self) -> order_list::OrderList {
        return self.orders.clone();
    }
}

pub fn clear_all_order_lights(elevhw: elevio::ElevatorHW, floor: u8) {
    for c in 0..3 {
        elevhw.clone().call_button_light(floor, c, false)
    }
}

fn on_door_time_out(mut elev: Elevator) {
    let state = elev.get_state();
    let hw_tx = elev.get_hw_tx_handle();
    match state {
        State::DoorOpen => {
            let new_dirn: u8 = local_order_manager::order_chooseDirection(&mut elev);

            hw_tx
                .send(elevio::HardwareCommand::DoorLight { on: false })
                .unwrap();
            hw_tx
                .send(elevio::HardwareCommand::MotorDirection { dirn: new_dirn })
                .unwrap();
            if new_dirn == elevio::DIRN_STOP {
                elev.state = State::Idle;
            } else {
                elev.state = State::Moving;
            }
        }
        _ => {},
    }
}

fn on_new_order(mut elev: Elevator, btn: poll::CallButton) {
    let state = elev.get_state();
    
    match state {
        State::DoorOpen => {
            if elev.get_floor() == btn.floor {
                //start timer
            } else {
                elev.orders.add_order(btn);
                elev.hw_tx
                .send(elevio::HardwareCommand::CallButtonLight {
                    floor: btn.floor,
                    call: btn.call,
                    on: true,
                })
                .unwrap();
            }
        }
        State::Moving => {
            elev.orders.add_order(btn);
            elev.hw_tx
                .send(elevio::HardwareCommand::CallButtonLight {
                    floor: btn.floor,
                    call: btn.call,
                    on: true,
                })
                .unwrap();
        }
        State::Idle => {
            if elev.get_floor() == btn.floor {
                elev.hw_tx
                    .send(elevio::HardwareCommand::DoorLight { on: true })
                    .unwrap();
                //timer start
                elev.state = State::DoorOpen;
            } else {
                elev.orders.add_order(btn);
                let new_dirn: u8 = local_order_manager::order_chooseDirection(&mut elev);
                elev.hw_tx
                    .send(elevio::HardwareCommand::MotorDirection { dirn: new_dirn })
                    .unwrap();
                elev.state = State::Moving;
                elev.dirn = new_dirn;
            }
        }
        _ => {},
    }
}

fn on_floor_arrival(mut elev: Elevator, new_floor: u8) {
    let state = elev.get_state();
    elev.floor = new_floor;
    //let hw_tx = elev.get_hw_tx_handle();
    elev.hw_tx
        .send(elevio::HardwareCommand::FloorLight { floor: new_floor })
        .unwrap();
    match state {
        State::Moving => {
            if local_order_manager::order_shouldStop(&mut elev) {
                elev.hw_tx
                    .send(elevio::HardwareCommand::MotorDirection {
                        dirn: elevio::DIRN_STOP,
                    })
                    .unwrap();
                elev.hw_tx
                    .send(elevio::HardwareCommand::DoorLight { on: true })
                    .unwrap();
                //Start timer
                elev.timer_start_tx.send(()).unwrap();
                elev.orders.clear_orders_on_floor(new_floor);
                elev.state = State::DoorOpen;
            } else {
                elev.floor = new_floor;
            }
        }
        State::Initializing => {
            elev.hw_tx
                .send(elevio::HardwareCommand::MotorDirection {
                    dirn: elevio::DIRN_STOP,
                })
                .unwrap();
                elev.state = State::Idle;
        }
        _ => {},
    }
}

fn on_obstruction_signal(mut elev: Elevator){
    elev.timer_start_tx.send(()).unwrap();
}

fn notifyPeerInfo() { /* Some logic for sending a message that PeerInfo uses to update its info on the local elevator? */
}




#[cfg(test)]
mod test {
    use super::*;
    use crate::elevio::poll::CallButton;

    fn initialize_elevator(
        num_floors: u8,
        arriving_floor: u8,
        hardware_command_tx: cbc::Sender<elevio::HardwareCommand>,
        door_timer_start_tx: cbc::Sender<()>
    ) -> Elevator {
        let mut elevator = Elevator::new(num_floors, hardware_command_tx, door_timer_start_tx);
        elevator.on_event(Event::OnFloorArrival {
            floor: arriving_floor,
        });
        return elevator;
    }
    #[test]
    fn it_initializes_correctly() {
        let (hw_tx, hw_rx) = cbc::unbounded::<elevio::HardwareCommand>();
        let (timer_tx, timer_rx) = cbc::unbounded::<()>();
        let elev_num_floors = 5;

        let mut elevator = Elevator::new(5, hw_tx, timer_tx);
        elevator.on_event(Event::OnFloorArrival { floor: 2 });
        let elevator_state = elevator.get_state();
        assert!((elevator.get_floor() == 2) && (elevator_state == State::Idle));
    }

    #[test]
    fn it_opens_the_door_when_order_on_current_floor() {
        let (hw_tx, hw_rx) = cbc::unbounded::<elevio::HardwareCommand>();
        let (timer_tx, timer_rx) = cbc::unbounded::<()>();
        let mut elevator = initialize_elevator(5, 3, hw_tx, timer_tx);
        while !hw_rx.is_empty() {
            hw_rx.recv().unwrap();
        }
        elevator.on_event(Event::OnNewOrder {
            btn: poll::CallButton { floor: 3, call: 2 },
        });
        assert_eq!(
            hw_rx.recv(),
            Ok(elevio::HardwareCommand::DoorLight { on: true })
        );
    }

    #[test]
    fn it_goes_up_when_order_is_above() {
        let (hw_tx, hw_rx) = cbc::unbounded::<elevio::HardwareCommand>();
        let (timer_tx, timer_rx) = cbc::unbounded::<()>();
        let mut elevator = initialize_elevator(5, 2, hw_tx, timer_tx);
        while !hw_rx.is_empty() {
            hw_rx.recv().unwrap();
        }
        elevator.on_event(Event::OnNewOrder {
            btn: poll::CallButton { floor: 4, call: 1 },
        });
        assert_eq!(
            hw_rx.recv(),
            Ok(elevio::HardwareCommand::MotorDirection {
                dirn: elevio::DIRN_UP
            })
        );
    }

    #[test]
    fn it_goes_down_when_order_is_below() {
        let (hw_tx, hw_rx) = cbc::unbounded::<elevio::HardwareCommand>();
        let (timer_tx, timer_rx) = cbc::unbounded::<()>();
        let mut elevator = initialize_elevator(5, 2, hw_tx, timer_tx);
        while !hw_rx.is_empty() {
            hw_rx.recv().unwrap();
        }
        elevator.on_event(Event::OnNewOrder {
            btn: poll::CallButton { floor: 0, call: 1 },
        });
        assert_eq!(
            hw_rx.recv(),
            Ok(elevio::HardwareCommand::MotorDirection {
                dirn: elevio::DIRN_DOWN
            })
        );
    }
    
    #[test]
    fn it_opens_door_at_ordered_floor() {
        let (hw_tx, hw_rx) = cbc::unbounded::<elevio::HardwareCommand>();
        let (timer_tx, timer_rx) = cbc::unbounded::<()>();
        let mut elevator = initialize_elevator(5, 2, hw_tx, timer_tx);
        while !hw_rx.is_empty() {
            hw_rx.recv().unwrap();
        }
        elevator.on_event(Event::OnNewOrder {
            btn: poll::CallButton { floor: 0, call: 1 },
        });
        elevator.on_event(Event::OnFloorArrival{floor: 1});
        elevator.on_event(Event::OnFloorArrival{floor: 0});
        assert_eq!(elevator.get_state(), State::DoorOpen);
    }

    #[test]
    fn it_goes_to_idle_when_no_orders_found() {
        let (hw_tx, hw_rx) = cbc::unbounded::<elevio::HardwareCommand>();
        let (timer_tx, timer_rx) = cbc::unbounded::<()>();
        let mut elevator = initialize_elevator(5, 2, hw_tx, timer_tx);
        while !hw_rx.is_empty() {
            hw_rx.recv().unwrap();
        }
        elevator.on_event(Event::OnNewOrder {
            btn: poll::CallButton { floor: 0, call: 1 },
        });
        elevator.on_event(Event::OnFloorArrival{floor: 1});
        elevator.on_event(Event::OnFloorArrival{floor: 0});
        elevator.on_event(Event::OnDoorTimeOut);
        assert_eq!(elevator.get_state(), State::Idle);
    }

    #[test]
    fn it_services_next_order_after_door_closed() {
        let (hw_tx, hw_rx) = cbc::unbounded::<elevio::HardwareCommand>();
        let (timer_tx, timer_rx) = cbc::unbounded::<()>();
        let mut elevator = initialize_elevator(5, 2, hw_tx, timer_tx);
        while !hw_rx.is_empty() {
            hw_rx.recv().unwrap();
        }
        elevator.on_event(Event::OnNewOrder {
            btn: poll::CallButton { floor: 0, call: 1 },
        });
        elevator.on_event(Event::OnNewOrder {
            btn: poll::CallButton { floor: 4, call: 2 },
        });
        elevator.on_event(Event::OnFloorArrival{floor: 1});
        elevator.on_event(Event::OnFloorArrival{floor: 0});
        elevator.on_event(Event::OnDoorTimeOut);
        assert_eq!(elevator.get_state(), State::Moving);
    }
    
}
