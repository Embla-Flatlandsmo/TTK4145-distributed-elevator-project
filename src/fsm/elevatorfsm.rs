#![allow(dead_code)]
use crate::elevio::elev as elevio;
use crate::elevio::poll;
use crate::order_manager::local_order_manager;
use crate::order_manager::order_list;

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
    pub fn new(
        n_floors: u8,
        hw_commander: cbc::Sender<elevio::HardwareCommand>,
        timer_start_tx: cbc::Sender<()>,
    ) -> Elevator {
        hw_commander
            .send(elevio::HardwareCommand::MotorDirection {
                dirn: elevio::DIRN_DOWN,
            })
            .unwrap();
        for floor in 0..n_floors {
            clear_all_order_lights_on_floor(&hw_commander, u8::from(floor));
        }
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
    pub fn on_event(&mut self, event: Event) {
        match event {
            Event::OnDoorTimeOut => self.on_door_time_out(),
            Event::OnFloorArrival { floor } => self.on_floor_arrival(floor),
            Event::OnNewOrder { btn } => self.on_new_order(btn),
            Event::OnObstructionSignal => self.on_obstruction_signal(),
            _ => panic!("Invalid event: {:#?}", event),
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

    fn on_door_time_out(&mut self) {
        let state = self.get_state();
        let hw_tx = self.get_hw_tx_handle();
        match state {
            State::DoorOpen => {    
                hw_tx
                    .send(elevio::HardwareCommand::DoorLight { on: false })
                    .unwrap();
                clear_all_order_lights_on_floor(&hw_tx, self.get_floor());
                self.orders.clear_orders_on_floor(self.get_floor());
                let new_dirn: u8 = local_order_manager::order_chooseDirection(self);
                hw_tx
                    .send(elevio::HardwareCommand::MotorDirection { dirn: new_dirn })
                    .unwrap();
                if new_dirn == elevio::DIRN_STOP {
                    self.state = State::Idle;
                } else {
                    self.state = State::Moving;
                }
            }
            _ => panic!("Door timed out in state {:#?}", state)
        }
    }

    fn on_floor_arrival(&mut self, new_floor: u8) {
        let state = self.get_state();
        self.floor = new_floor;
        //let hw_tx = self.get_hw_tx_handle();
        self.hw_tx
            .send(elevio::HardwareCommand::FloorLight { floor: new_floor })
            .unwrap();
        match state {
            State::Moving => {
                if local_order_manager::order_shouldStop(self) {
                    self.hw_tx
                        .send(elevio::HardwareCommand::MotorDirection {
                            dirn: elevio::DIRN_STOP,
                        })
                        .unwrap();
                    self.hw_tx
                        .send(elevio::HardwareCommand::DoorLight { on: true })
                        .unwrap();
                    self.state = State::DoorOpen;
                    //Start timer
                    self.timer_start_tx.send(()).unwrap();

                } else {
                    self.floor = new_floor;
                }
            }
            State::Initializing => {
                self.hw_tx
                    .send(elevio::HardwareCommand::MotorDirection {
                        dirn: elevio::DIRN_STOP,
                    })
                    .unwrap();
                self.state = State::Idle;
            }
            _ => {},
        }
    }

    fn on_new_order(&mut self, btn: poll::CallButton) {
        let state = self.get_state();

        match state {
            State::DoorOpen => {
                if self.get_floor() == btn.floor {
                    //start timer
                    self.timer_start_tx.send(()).unwrap();
                } else {
                    self.orders.add_order(btn);
                    self.hw_tx
                        .send(elevio::HardwareCommand::CallButtonLight {
                            floor: btn.floor,
                            call: btn.call,
                            on: true,
                        })
                        .unwrap();
                }
            }
            State::Moving => {
                self.orders.add_order(btn);
                self.hw_tx
                    .send(elevio::HardwareCommand::CallButtonLight {
                        floor: btn.floor,
                        call: btn.call,
                        on: true,
                    })
                    .unwrap();
            }
            State::Idle => {
                if self.get_floor() == btn.floor {
                    self.hw_tx
                        .send(elevio::HardwareCommand::DoorLight { on: true })
                        .unwrap();
                    //timer start
                    self.state = State::DoorOpen;
                } else {
                    self.orders.add_order(btn);
                    let new_dirn: u8 = local_order_manager::order_chooseDirection(self);
                    self.hw_tx
                        .send(elevio::HardwareCommand::MotorDirection { dirn: new_dirn })
                        .unwrap();
                    self.state = State::Moving;
                    self.dirn = new_dirn;
                }
            }
            _ => panic!("Tried to add new order in invalid state: {:#?}", state)
        }
    }

    fn on_obstruction_signal(&mut self) {
        if self.get_state() == State::DoorOpen {
            self.timer_start_tx.send(()).unwrap();
        }
    }
}

fn clear_all_order_lights_on_floor(hw_tx: &crossbeam_channel::Sender<elevio::HardwareCommand>, floor: u8) {
    for c in 0..3 {
        hw_tx.send(elevio::HardwareCommand::CallButtonLight{floor: floor, call: c, on: false}).unwrap();
    }
}

fn notify_peer_info() { /* Some logic for sending a message that PeerInfo uses to update its info on the local elevator? */
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread;

    fn initialize_elevator(
        num_floors: u8,
        arriving_floor: u8,
        hardware_command_tx: cbc::Sender<elevio::HardwareCommand>,
        door_timer_start_tx: cbc::Sender<()>,
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

        let mut elevator = Elevator::new(elev_num_floors, hw_tx, timer_tx);
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
            btn: poll::CallButton { floor: 0, call: 0 },
        });
        elevator.on_event(Event::OnFloorArrival { floor: 1 });
        elevator.on_event(Event::OnFloorArrival { floor: 0 });
        assert_eq!(elevator.get_state(), State::DoorOpen);
    }

    #[test]
    fn it_sends_timer_signal_when_door_opened() {
        let (hw_tx, hw_rx) = cbc::unbounded::<elevio::HardwareCommand>();
        let (timer_tx, timer_rx) = cbc::unbounded::<()>();
        let mut elevator = initialize_elevator(5, 2, hw_tx, timer_tx);
        while !hw_rx.is_empty() {
            hw_rx.recv().unwrap();
        }
        elevator.on_event(Event::OnNewOrder {
            btn: poll::CallButton { floor: 0, call: 1 },
        });
        elevator.on_event(Event::OnFloorArrival { floor: 1 });
        elevator.on_event(Event::OnFloorArrival { floor: 0 });
        assert_eq!(timer_rx.recv(), Ok(()));
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
        elevator.on_event(Event::OnFloorArrival { floor: 1 });
        elevator.on_event(Event::OnFloorArrival { floor: 0 });
        elevator.on_event(Event::OnDoorTimeOut);
        assert_eq!(elevator.get_state(), State::Idle);
    }

    #[test]
    fn it_clears_orders_after_servicing_floor() {
        let (hw_tx, hw_rx) = cbc::unbounded::<elevio::HardwareCommand>();
        let (timer_tx, timer_rx) = cbc::unbounded::<()>();
        let mut elevator = initialize_elevator(5, 2, hw_tx, timer_tx);
        while !hw_rx.is_empty() {
            hw_rx.recv().unwrap();
        }
        elevator.on_event(Event::OnNewOrder {
            btn: poll::CallButton { floor: 0, call: 1 },
        });
        elevator.on_event(Event::OnFloorArrival { floor: 1 });
        elevator.on_event(Event::OnFloorArrival { floor: 0 });
        elevator.on_event(Event::OnDoorTimeOut);
        let ref_orders = order_list::OrderList::new(5);
        assert_eq!(elevator.get_orders(), ref_orders);
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
        elevator.on_event(Event::OnFloorArrival { floor: 1 });
        elevator.on_event(Event::OnFloorArrival { floor: 0 });
        elevator.on_event(Event::OnDoorTimeOut);
        assert_eq!(elevator.get_state(), State::Moving);
    }
}
