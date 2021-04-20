use crossbeam_channel as cbc;

use crate::local_elevator::elevio::{elev, poll};
use crate::local_elevator::fsm::door_timer::TimerCommand;
use crate::local_elevator::fsm::elevatorfsm::*;
use crate::local_elevator::elevio::poll::CallButton;
use crate::global_elevator_info::connected_elevators::ConnectedElevatorInfo;
use crate::util::constants::ID as LOCAL_ID;
use crate::util::constants as setting;

const TRAVEL_TIME: u64 = 2;


pub fn find_lowest_cost_id(
    connected_elevator_info: ConnectedElevatorInfo, 
    button_to_add: CallButton) -> usize {
    
    let local_elev_info;
    match connected_elevator_info.get_local_elevator_info() {
        Some(v) => local_elev_info = v,
        None => {
            println!("Info not found at ID {}, assigning order to local elevator.", LOCAL_ID);
            return LOCAL_ID;
    }
}

    let mut lowest_cost: usize =
        time_to_idle(local_elev_info, button_to_add);
    let mut lowest_cost_id: usize = LOCAL_ID;

    for i in connected_elevator_info.get_connected_elevators().iter().cloned() {
        let elev_cost: usize;
        match i.clone() {
            Some(val) => {
                elev_cost = time_to_idle((val).clone(), button_to_add);
                if elev_cost < lowest_cost {
                    lowest_cost_id = (val).id;
                    lowest_cost = elev_cost;
                }
            }
            None => {}
        }
    }
    return lowest_cost_id;
}

/// Calculates the time it takes for an elevator to reach the idle
/// state after we have added the new order.
///
/// `fsm` - elevator to simulate
///
/// `button` - Button corresponding to order we want to add.
fn time_to_idle(ref mut elev_info: ElevatorInfo, ref button: poll::CallButton) -> usize {
    // Dummy timers needed to "disconnect" the elevator from its current channel
    let (dummy_hw_tx, _dummy_hw_rx) = cbc::unbounded::<elev::HardwareCommand>();
    let (dummy_timer_tx, _dummy_timer_rx) = cbc::unbounded::<TimerCommand>();
    let (dummy_state_updater_tx, __dummy_state_updater_rx) = cbc::unbounded::<State>();

    let mut elev = Elevator::create_simulation_elevator(elev_info.clone(), dummy_hw_tx, dummy_timer_tx, dummy_state_updater_tx);
    elev.on_event(Event::OnNewOrder{btn: *button});
    let mut duration: usize = 0;
    let state = elev.get_state();
    if state == State::Obstructed || state == State::ObstrTimedOut || 
    state == State::MovTimedOut || state == State::Initializing {
        return usize::MAX;
    }
    while elev.get_state() != State::Idle {
        duration += simulate_next_step(&mut elev);
    }
    return duration;
}

/// Estimates the time it takes for the elevator to reach the next event.
/// It is to be used in the loop of the cost function.
///
/// `fsm` - the fsm to simulate
fn simulate_next_step(fsm: &mut Elevator) -> usize {
    match fsm.get_state() {
        State::Moving => {
            if fsm.get_dirn() == elev::DIRN_DOWN {
                fsm.on_event(Event::OnFloorArrival {
                    floor: (fsm.get_floor() - 1),
                });
            } else {
                fsm.on_event(Event::OnFloorArrival {
                    floor: (fsm.get_floor() + 1),
                });
            }
            return TRAVEL_TIME as usize;
        }
        State::DoorOpen => {
            fsm.on_event(Event::OnDoorTimeOut);
            return setting::DOOR_OPEN_TIME as usize;
        }
        State::Idle => 0,
        _ => 0,
    }
}