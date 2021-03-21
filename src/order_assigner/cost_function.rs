use crate::fsm::elevatorfsm::*;
use crate::order_manager::local_order_manager;
use crate::elevio::{poll, elev};

pub const TRAVEL_TIME: u64 = 1.5;

/// Calculates the time it takes for an elevator to reach the idle
/// state after we have added the new order.
/// 
/// `fsm` - elevator to simulate
/// 
/// `button` - Button corresponding to order we want to add.
pub fn time_to_idle(mut fsm: &Elevator, button: elevio::poll::CallButton) -> usize {
    let mut elev = fsm.get_simulation_elevator();
    let duration: usize = 0;

    elev.on_event(Event::OnNewOrder{btn: button});

    while elev.get_state() != State::Idle {
        duration += time_for_next_step(elev);
    }
}

/// Estimates the time it takes for the elevator to reach the next event.
/// It is to be used in the loop of the cost function.
/// 
/// `fsm` - the fsm to simulate
fn time_for_next_step(fsm: &mut Elevator) -> usize {
    match fsm.get_state() {
        State::Moving => {
            if fsm.get_dirn() == elevio::elev::DIRN_DOWN {
                fsm.on_event(Event::OnFloorArrival{fsm.get_floor()-1});
            } else {
                fsm.on_event(Event::OnFloorArrival{fsm.get_floor()+1});
            }
            return TRAVEL_TIME;
        },
        State::DoorOpen => {
            fsm.on_event(Event::OnDoorTimeOut);
            return DOOR_OPEN_TIME;
        }
        State::Idle => 0,
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crossbeam_channel as cbc;
    fn initialize_elevator(
        num_floors: u8,
        arriving_floor: u8,
        hardware_command_tx: cbc::Sender<elevio::HardwareCommand>,
        door_timer_start_tx: cbc::Sender<TimerCommand>,
    ) -> Elevator {
        let mut elevator = Elevator::new(num_floors, hardware_command_tx, door_timer_start_tx);
        elevator.on_event(Event::OnFloorArrival {
            floor: arriving_floor,
        });
        return elevator;
    }
    
    #[test]
    fn it_does_not_alter_elevator() {
        let (hw_tx, hw_rx) = cbc::unbounded::<elevio::HardwareCommand>();
        let (timer_tx, _timer_rx) = cbc::unbounded::<TimerCommand>();
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
        let elevator_backup = elevator.clone();

        let new_button = poll::CallButton{floor: 2, call: 1};
        let cost = time_to_idle(elevator, new_button);
        assert_eq!(elevator, elevator_backup);
    }

    #[test]
    fn idle_elevator_cheaper_than_moving() {
        let (hw_tx, hw_rx) = cbc::unbounded::<elevio::HardwareCommand>();
        let (timer_tx, _timer_rx) = cbc::unbounded::<TimerCommand>();
        let mut moving_elevator = initialize_elevator(5, 2, hw_tx, timer_tx);
        while !hw_rx.is_empty() {
            hw_rx.recv().unwrap();
        }
        moving_elevator.on_event(Event::OnNewOrder {
            btn: poll::CallButton { floor: 0, call: 1 },
        });
        moving_elevator.on_event(Event::OnNewOrder {
            btn: poll::CallButton { floor: 4, call: 2 },
        });

        let mut idle_elevator = initialize_elevator(5, 2, hw_tx, timer_tx);

        let new_order = poll::CallButton{floor:5, call: 1};

        let cost_idle = time_to_idle(&idle_elevator, new_order);
        let cost_moving = time_to_idle(&moving_elevator,new_order);

        assert(cost_idle<cost_moving);
    }
}
