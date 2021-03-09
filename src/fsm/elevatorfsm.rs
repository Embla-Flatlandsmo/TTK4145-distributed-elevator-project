#![allow(dead_code)]
//#[path = "../elevio/elev.rs"]
use crate::elevio::elev as elevio;
//#[path = "../timer/timer.rs"]

/* Structs */
#[derive(Clone, Debug)]
pub struct Elevator {
    hw: elevio::ElevatorHW,
    state: State,
    dirn: u8,
    floor: u8,
}

#[derive(Clone, Debug, PartialEq)]
enum State {
    Initializing,
    DoorOpen,
    Idle,
    Stopped,
    Moving,
    Failure(String),
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    ShouldOpenDoor,
    ShouldMoveDown,
    ShouldMoveUp,
    ArriveAtDestination,
    DoorTimedOut,
    ObstructionSignal,
    StopSignal,
}

pub const DOOR_OPEN_TIME: u8 = 3;

impl Elevator {
    pub fn new(elevhw: elevio::ElevatorHW) -> Elevator {
        elevhw.clone().motor_direction(elevio::DIRN_DOWN);
        Elevator {
            hw: elevhw,
            state: State::Moving,
            dirn: elevio::DIRN_DOWN,
            floor: u8::MAX,
        }
    }

    pub fn transition(self, event: Event) -> Elevator {
        let from_state = self.clone().state;
        match (from_state, event) {
            (State::Idle, Event::ShouldOpenDoor) => on_should_open_door(self),
            (State::Idle, Event::ShouldMoveDown) => on_should_move(self, elevio::DIRN_DOWN),
            (State::Idle, Event::ShouldMoveUp) => on_should_move(self, elevio::DIRN_UP),
            (State::DoorOpen, Event::DoorTimedOut) => on_door_timed_out(self),
            (State::Moving, Event::ArriveAtDestination) => on_arrive_at_destination(self),
            (s,e) => Elevator{
                hw: self.hw,
                state: State::Failure(format!("Wrong state, event combination: {:#?} {:#?}", s, e)
                .to_string()),
                dirn: self.dirn,
                floor: self.floor
            }
        }
    }


    pub fn set_floor(self, new_floor: u8) -> Elevator {
        Elevator {
            hw: self.hw,
            state: self.state,
            dirn: self.dirn,
            floor: new_floor,
        }
    }

    pub fn get_floor(self) -> u8 { return self.clone().floor;}
    pub fn get_state(self) -> State { return self.clone().state;}
    pub fn get_dirn(self) -> u8 { return self.clone().dirn; }
}

pub fn clear_all_order_lights(elevhw: elevio::ElevatorHW, floor: u8) { 
    for c in 0..3 {
        elevhw.clone().call_button_light(floor, c, false)
    }
}

pub fn on_should_move(elev: Elevator, dir: u8) -> Elevator {
    elev.hw.clone().motor_direction(dir);
    if dir == elevio::DIRN_DOWN {
        println!("OnShouldMove, DIRN = DOWN");
    } else if dir == elevio::DIRN_UP {
        println!("OnShouldMove, DIRN = UP");
    } else {
        println!("OnShouldMove, Unrecognized Direction");
    }

    Elevator {
        hw: elev.hw,
        state: State::Moving,
        dirn: dir,
        floor: elev.floor,
    }
}

fn on_door_timed_out(elev: Elevator) -> Elevator {
    elev.hw.clone().door_light(false);
    clear_all_order_lights(elev.hw.clone(), elev.floor);
    Elevator {
        hw: elev.hw,
        state: State::Idle,
        dirn: elev.dirn,
        floor: elev.floor,
    }
}

fn on_should_open_door(elev: Elevator) -> Elevator {
    elev.hw.clone().door_light(true);
    Elevator {
        hw: elev.hw,
        state: State::DoorOpen,
        dirn: elev.dirn,
        floor: elev.floor,
    }
}

fn on_arrive_at_destination(elev: Elevator) -> Elevator {
    elev.hw.clone().motor_direction(elevio::DIRN_STOP);
    Elevator {
        hw: elev.hw,
        state: State::Idle,
        dirn: elev.dirn,
        floor: elev.floor,
    }
}
//To be used inside transition from dooropen to moving/idle


fn notifyPeerInfo() { /* Some logic for sending a message that PeerInfo uses to update its info on the local elevator? */
}

/* I think these might be unnecessary */
//fn fsm_onInitBetweenFloors() {/*...*/}
//fn fsm_onRequestButtonPress(btn_floor: u8, button_type: CallButton) {/*...*/}
//fn fsm_onFloorArrival(new_floor: u8) {/*...*/}
//fn fsm_onDoorTimeout() {/*...*/}
