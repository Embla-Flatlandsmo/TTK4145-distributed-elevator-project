#![allow(dead_code, unused_mut, unused_variables)]
use crate::local_elevator::elevio::elev as elevio;
use crate::local_elevator::elevio::poll;
use crate::local_elevator::fsm::order_list;
use crate::local_elevator::fsm::order_list::OrderType;
use crate::util::constants as setting;
use serde;
use std::time;

#[path = "./direction_decider.rs"]
mod direction_decider;

use crate::local_elevator::fsm::door_timer::TimerCommand;

use crossbeam_channel as cbc;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Hash, PartialEq)]
pub struct ElevatorInfo {
    pub id: usize,
    pub state: State,
    pub dirn: u8,
    pub floor: u8,
    pub responsible_orders: order_list::OrderList,
}

impl ElevatorInfo {
    pub fn get_id(&self) -> usize {
        return self.clone().id;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, Hash)]
pub enum State {
    Initializing,
    DoorOpen,
    Idle,
    Obstructed,
    ObstrTimedOut,
    Moving,
    MovTimedOut,
}

#[derive(Debug)]
pub enum Event {
    OnDoorTimeOut,
    OnFloorArrival { floor: u8 },
    OnNewOrder { btn: poll::CallButton },
    OnObstructionSignal { active: bool },
    OnStateTimeOut,
}

/// Contains all we need to know about our elevator.
/// * `hw_tx` the transmitter for sending hardware commands
/// * `timer_start_tx` the transmitter for starting the door timer
/// * `state_update_tx` the transmitter for door 
/// * `info` information about the elevator: `state`, `dirn`, `floor` and `responsible_orders`
#[derive(Clone, Debug)]
pub struct Elevator {
    hw_tx: crossbeam_channel::Sender<elevio::HardwareCommand>,
    timer_start_tx: cbc::Sender<TimerCommand>,
    state_update_tx: cbc::Sender<State>,
    info: ElevatorInfo,
}

impl Elevator {
    pub fn new(
        hw_commander: cbc::Sender<elevio::HardwareCommand>,
        timer_start_tx: cbc::Sender<TimerCommand>,
        state_updater_tx: cbc::Sender<State>,
    ) -> Elevator {
        hw_commander
            .send(elevio::HardwareCommand::MotorDirection {
                dirn: elevio::DIRN_DOWN,
            })
            .unwrap();
        // Disable all lights when we first start
        for f in 0..setting::ELEV_NUM_FLOORS {
            for c in 0..3 {
                hw_commander.send(elevio::HardwareCommand::CallButtonLight{floor: f, call: c, on: false}).unwrap();
            }
        }
        hw_commander.send(elevio::HardwareCommand::StopLight{on: false}).unwrap();
        hw_commander.send(elevio::HardwareCommand::DoorLight{on: false}).unwrap();   

        return Elevator {
            hw_tx: hw_commander,
            timer_start_tx: timer_start_tx,
            state_update_tx: state_updater_tx,
            info: ElevatorInfo {
                id: setting::ID,
                state: State::Initializing,
                dirn: elevio::DIRN_DOWN,
                floor: u8::MAX,
                responsible_orders: order_list::OrderList::new(setting::ELEV_NUM_FLOORS),
            },
        };
    }
    /// Takes the elevator fsm from one state to the next and sends the appropriate hardware commands on the hardware channel
    #[allow(unreachable_patterns)]
    pub fn on_event(&mut self, event: Event) {
        match event {
            Event::OnDoorTimeOut => self.on_door_time_out(),
            Event::OnFloorArrival { floor } => self.on_floor_arrival(floor),
            Event::OnNewOrder { btn } => self.on_new_order(btn),
            Event::OnObstructionSignal { active } => self.on_obstruction_signal(active),
            Event::OnStateTimeOut => self.on_state_timeout(),
            _ => panic!("Invalid event: {:#?}", event),
        }
    }

    pub fn create_simulation_elevator(
        info: ElevatorInfo,
        hw_commander: cbc::Sender<elevio::HardwareCommand>,
        timer_start_tx: cbc::Sender<TimerCommand>,
        state_updater_tx: cbc::Sender<State>,
    ) -> Elevator {
        return Elevator {
            hw_tx: hw_commander,
            timer_start_tx: timer_start_tx,
            state_update_tx: state_updater_tx,
            info: info.clone(),
        };
    }

    pub fn get_info(&self) -> ElevatorInfo {
        return self.info.clone();
    }
    pub fn get_floor(&self) -> u8 {
        return self.get_info().floor;
    }
    pub fn get_state(&self) -> State {
        return self.get_info().state;
    }
    pub fn get_dirn(&self) -> u8 {
        return self.get_info().dirn;
    }
    pub fn get_hw_tx_handle(&self) -> cbc::Sender<elevio::HardwareCommand> {
        return self.hw_tx.clone();
    }
    pub fn get_orders(&self) -> order_list::OrderList {
        return self.get_info().responsible_orders;
    }

    fn on_door_time_out(&mut self) {
        let state = self.get_state();
        match state {
            State::DoorOpen => {
                self.hw_tx
                    .send(elevio::HardwareCommand::DoorLight { on: false })
                    .unwrap();
                self.info
                    .responsible_orders
                    .clear_orders_on_floor(self.get_floor());
                let new_dirn: u8 = direction_decider::choose_direction(self);
                self.hw_tx
                    .send(elevio::HardwareCommand::MotorDirection { dirn: new_dirn })
                    .unwrap();
                if new_dirn == elevio::DIRN_STOP {
                    self.info.state = State::Idle;
                    self.state_update_tx.send(State::Idle).unwrap();
                } else {
                    self.info.dirn = new_dirn;
                    self.info.state = State::Moving;
                    self.state_update_tx.send(State::Moving).unwrap();
                }
            }
            _ => {}
        }
    }

    fn on_floor_arrival(&mut self, new_floor: u8) {
        let state = self.get_state();
        self.info.floor = new_floor;
        self.hw_tx
            .send(elevio::HardwareCommand::FloorLight { floor: new_floor })
            .unwrap();
        match state {
            State::Moving => {
                if direction_decider::should_stop(self) {
                    self.hw_tx
                        .send(elevio::HardwareCommand::MotorDirection {
                            dirn: elevio::DIRN_STOP,
                        })
                        .unwrap();
                    self.hw_tx
                        .send(elevio::HardwareCommand::DoorLight { on: true })
                        .unwrap();
                    self.info.state = State::DoorOpen;
                    self.state_update_tx.send(State::DoorOpen).unwrap();
                    //Start timer
                    self.timer_start_tx.send(TimerCommand::Start).unwrap();
                } else {
                    self.state_update_tx.send(State::Moving).unwrap();
                }
            }
            State::Initializing => {
                self.hw_tx
                .send(elevio::HardwareCommand::DoorLight { on: true })
                .unwrap();
                self.hw_tx
                .send(elevio::HardwareCommand::MotorDirection {
                    dirn: elevio::DIRN_STOP,
                }).unwrap();
                self.info.state = State::DoorOpen;
                self.state_update_tx.send(State::DoorOpen).unwrap();
                self.timer_start_tx.send(TimerCommand::Start).unwrap();
            }
            State::MovTimedOut => {
                self.info.responsible_orders.change_all_assigned_hall_order_status(OrderType::Active);
                self.hw_tx
                    .send(elevio::HardwareCommand::MotorDirection {
                        dirn: elevio::DIRN_STOP,
                    })
                    .unwrap();
                self.hw_tx
                    .send(elevio::HardwareCommand::DoorLight { on: true })
                    .unwrap();
                self.info.state = State::DoorOpen;
                self.state_update_tx.send(State::DoorOpen).unwrap();
                //Start timer
                self.timer_start_tx.send(TimerCommand::Start).unwrap();
            }
            _ => {}
        }
    }

    fn on_new_order(&mut self, btn: poll::CallButton) {
        let state = self.get_state();

        match state {
            State::DoorOpen => {
                if self.get_floor() == btn.floor {
                    //start timer
                    self.timer_start_tx.send(TimerCommand::Start).unwrap();
                }
                self.info.responsible_orders.set_active(btn);
            }
            State::Obstructed | State::Moving | State::ObstrTimedOut => {
                self.info.responsible_orders.set_active(btn);
            }

            State::Idle => {
                self.info.responsible_orders.set_active(btn);
                if self.get_floor() == btn.floor {
                    self.hw_tx
                        .send(elevio::HardwareCommand::DoorLight { on: true })
                        .unwrap();
                    self.timer_start_tx.send(TimerCommand::Start).unwrap();
                    self.info.state = State::DoorOpen;
                    self.state_update_tx.send(State::DoorOpen).unwrap();
                } else {
                    let new_dirn: u8 = direction_decider::choose_direction(self);
                    self.hw_tx
                        .send(elevio::HardwareCommand::MotorDirection { dirn: new_dirn })
                        .unwrap();
                    self.info.state = State::Moving;
                    self.state_update_tx.send(State::Moving).unwrap();
                    self.info.dirn = new_dirn;
                }
            }

            State::Initializing | State::MovTimedOut => {}
        }
    }

    fn on_obstruction_signal(&mut self, active: bool) {
        let state = self.get_state();
        if state == State::DoorOpen || state == State::Obstructed || state == State::ObstrTimedOut {
            match active {
                true => {
                    self.timer_start_tx.send(TimerCommand::Cancel).unwrap();
                    self.info.state = State::Obstructed;
                    self.state_update_tx.send(State::Obstructed).unwrap();
                }
                false => {
                    self.timer_start_tx.send(TimerCommand::Start).unwrap();
                    self.info.state = State::DoorOpen;
                    self.state_update_tx.send(State::DoorOpen).unwrap();
                }
            }
        }
    }

    fn on_state_timeout(&mut self) {
        let state = self.get_state();
        match state {
            State::Obstructed => {
                self.info.state = State::ObstrTimedOut;
                self.state_update_tx.send(State::ObstrTimedOut).unwrap();
            }
            State::Moving | State::Initializing => {
                self.info.state = State::MovTimedOut;
                self.info.responsible_orders.change_all_assigned_hall_order_status(OrderType::Pending);
                self.state_update_tx.send(State::MovTimedOut).unwrap();
            }
            _ => {}
        }
    }
}

pub fn create_simulation_elevator(
    elev_info: ElevatorInfo,
    dummy_hw_tx: cbc::Sender<elevio::HardwareCommand>,
    dummy_timer_start_tx: cbc::Sender<TimerCommand>,
    dummy_state_updater_tx: cbc::Sender<State>,
) -> Elevator {
    return Elevator {
        hw_tx: dummy_hw_tx,
        timer_start_tx: dummy_timer_start_tx,
        state_update_tx: dummy_state_updater_tx,
        info: elev_info.clone(),
    };
}


pub fn state_timeout_checker(state_updater_rx: cbc::Receiver<State>, elev_timeout_tx: cbc::Sender<()>) {
    let mut when_state_updated = time::Instant::now();
    let mut timeout_duration = time::Duration::from_secs(setting::MOTOR_TIMEOUT_DURATION_SEC);
    loop {
        match state_updater_rx.try_recv() {
            Ok(val) => {
                when_state_updated = time::Instant::now();
                let state = val;
                match state {
                    State::Obstructed => {
                        timeout_duration = time::Duration::from_secs(setting::OBSTRUCTED_TIME_BEFORE_REASSIGN_SEC);
                    },
                    State::Moving | State::Initializing => {
                        timeout_duration = time::Duration::from_secs(setting::MOTOR_TIMEOUT_DURATION_SEC);
                    },
                    _ => {
                        timeout_duration = time::Duration::new(u64::MAX,0);
                    },
                }
            },
            _ => {}
        }
        let now = time::Instant::now();
        if now.duration_since(when_state_updated) > timeout_duration {
            elev_timeout_tx.send(()).unwrap();
        }
    }
}



#[cfg(test)]
mod test {
    use super::*;
    fn initialize_elevator(
        arriving_floor: u8,
        hardware_command_tx: cbc::Sender<elevio::HardwareCommand>,
        door_timer_start_tx: cbc::Sender<TimerCommand>,
        state_updater_tx: cbc::Sender<State>,
    ) -> Elevator {
        let mut elevator =
            Elevator::new(hardware_command_tx, door_timer_start_tx, state_updater_tx);
        elevator.on_event(Event::OnFloorArrival {
            floor: arriving_floor,
        });
        return elevator;
    }
    #[test]
    fn it_initializes_correctly() {
        let (hw_tx, _hw_rx) = cbc::unbounded::<elevio::HardwareCommand>();
        let (timer_tx, _timer_rx) = cbc::unbounded::<TimerCommand>();
        let (state_updater_tx, _state_updater_rx) = cbc::unbounded::<State>();

        let elevator = initialize_elevator(2, hw_tx, timer_tx, state_updater_tx);
        let elevator_state = elevator.get_state();
        assert!((elevator.get_floor() == 2) && (elevator_state == State::Idle));
    }

    #[test]
    fn it_opens_the_door_when_order_on_current_floor() {
        let (hw_tx, hw_rx) = cbc::unbounded::<elevio::HardwareCommand>();
        let (timer_tx, _timer_rx) = cbc::unbounded::<TimerCommand>();
        let (state_updater_tx, _state_updater_rx) = cbc::unbounded::<State>();

        let mut elevator = initialize_elevator(2, hw_tx, timer_tx, state_updater_tx);
        while !hw_rx.is_empty() {
            hw_rx.recv().unwrap();
        }
        elevator.on_event(Event::OnNewOrder {
            btn: poll::CallButton { floor: 3, call: 2 },
        });
        hw_rx.recv().unwrap();
        assert_eq!(
            hw_rx.recv(),
            Ok(elevio::HardwareCommand::DoorLight { on: true })
        );
    }

    #[test]
    fn it_goes_up_when_order_is_above() {
        let (hw_tx, hw_rx) = cbc::unbounded::<elevio::HardwareCommand>();
        let (timer_tx, _timer_rx) = cbc::unbounded::<TimerCommand>();
        let (state_updater_tx, _state_updater_rx) = cbc::unbounded::<State>();

        let mut elevator = initialize_elevator(2, hw_tx, timer_tx, state_updater_tx);
        while !hw_rx.is_empty() {
            hw_rx.recv().unwrap();
        }
        elevator.on_event(Event::OnNewOrder {
            btn: poll::CallButton { floor: 4, call: 1 },
        });

        hw_rx.recv().unwrap();
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
        let (timer_tx, _timer_rx) = cbc::unbounded::<TimerCommand>();
        let (state_updater_tx, _state_updater_rx) = cbc::unbounded::<State>();

        let mut elevator = initialize_elevator(2, hw_tx, timer_tx, state_updater_tx);
        while !hw_rx.is_empty() {
            hw_rx.recv().unwrap();
        }
        elevator.on_event(Event::OnNewOrder {
            btn: poll::CallButton { floor: 0, call: 1 },
        });
        hw_rx.recv().unwrap();
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
        let (timer_tx, _timer_rx) = cbc::unbounded::<TimerCommand>();
        let (state_updater_tx, _state_updater_rx) = cbc::unbounded::<State>();

        let mut elevator = initialize_elevator(2, hw_tx, timer_tx, state_updater_tx);
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
        let (timer_tx, timer_rx) = cbc::unbounded::<TimerCommand>();
        let (state_updater_tx, _state_updater_rx) = cbc::unbounded::<State>();

        let mut elevator = initialize_elevator(2, hw_tx, timer_tx, state_updater_tx);
        while !hw_rx.is_empty() {
            hw_rx.recv().unwrap();
        }
        elevator.on_event(Event::OnNewOrder {
            btn: poll::CallButton { floor: 0, call: 1 },
        });
        elevator.on_event(Event::OnFloorArrival { floor: 1 });
        elevator.on_event(Event::OnFloorArrival { floor: 0 });
        assert_eq!(timer_rx.recv(), Ok(TimerCommand::Start));
    }

    #[test]
    fn it_goes_to_idle_when_no_orders_found() {
        let (hw_tx, hw_rx) = cbc::unbounded::<elevio::HardwareCommand>();
        let (timer_tx, _timer_rx) = cbc::unbounded::<TimerCommand>();
        let (state_updater_tx, _state_updater_rx) = cbc::unbounded::<State>();

        let mut elevator = initialize_elevator(2, hw_tx, timer_tx, state_updater_tx);
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
        let (timer_tx, _timer_rx) = cbc::unbounded::<TimerCommand>();
        let (state_updater_tx, _state_updater_rx) = cbc::unbounded::<State>();

        let mut elevator = initialize_elevator(2, hw_tx, timer_tx, state_updater_tx);
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
        let (timer_tx, _timer_rx) = cbc::unbounded::<TimerCommand>();
        let (state_updater_tx, _state_updater_rx) = cbc::unbounded::<State>();

        let mut elevator = initialize_elevator(2, hw_tx, timer_tx, state_updater_tx);
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
