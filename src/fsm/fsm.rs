
#![allow(dead_code)]
/*enum ElevatorBehaviour {
    Idle,
    Moving,
    DoorOpen,
    Initializing
}
*/

/**
 * Based on the FSM proposed by Ana Hobden: https://hoverbear.org/blog/rust-state-machine-pattern/
 * This design should only allow for catching as many errors as possible during compile-time, easy-to-understand
 * error messages and ensures we can only be in one state at a time.
 */

#[path = "../elevio/elev.rs"] mod elevio;
#[path = "../timer/timer.rs"] pub mod timer;

/* Structs */
struct FSM<ElevatorBehaviour> {
    hw: elevio::ElevatorHW,
    pub floor: u8,
    pub behaviour: ElevatorBehaviour,
}

struct Initializing {}
struct Idle {}
struct Moving {
    drivingDirection: u8
}
struct DoorOpen {
    door_timer: timer::Timer
}
pub const DOOR_OPEN_TIME:    u64 = 3;
/**
 * -----------------------------------------------------TRANSITION IMPLEMENTATIONS-----------------------------------------------------
 * 
 */

/**
 * -----------------------------------------------------Initialization-----------------------------------------------------
 */
impl FSM<Initializing> {
    /* Creates a new instance of a FSM */
    fn new(e: elevio::ElevatorHW) -> Self {
        FSM {
            hw: e,
            floor: u8::MAX,
            behaviour: Initializing{},
        }
    }
}

// Maybe this state is needed because the floor would be undefined? OR we could just go straight into downward move.
impl Initializing {
    fn new() -> Self {
        Initializing{}
    }
}

// We should move straight from initializing to downward moving
impl From<FSM<Initializing>> for FSM<Moving> {
    fn from(val: FSM<Initializing>) -> FSM<Moving> {
        let mut dirn = elevio::DIRN_DOWN;
        val.hw.motor_direction(dirn);
        FSM {
            hw: val.hw,
            floor: val.floor,
            behaviour: Moving {drivingDirection: dirn}
        }
    }
}

/**
 * -----------------------------------------------------Transitions from Idle-----------------------------------------------------
 */
// Idle->Moving
impl From<FSM<Idle>> for FSM<Moving> {
    fn from(val: FSM<Idle>) -> FSM<Moving> {
        // Todo: Disallow any value but DIRN_DOWN/STOP/UP
        // Figure out motor direction
        let dirn = elevio::DIRN_DOWN;
        val.hw.motor_direction(dirn);
        FSM {
            hw: val.hw,
            floor: val.floor,
            behaviour: Moving {drivingDirection: dirn}
        }
    }
}

// Idle->DoorOpen
impl From<FSM<Idle>> for FSM<DoorOpen> {
    fn from(val: FSM<Idle>) -> FSM<DoorOpen> {
        val.hw.door_light(true);
        //val.hw.call_button_light(floor: u8, call: u8, on: bool); //Some function to clear light
        FSM {
            hw: val.hw,
            floor: val.floor,
            behaviour: DoorOpen { door_timer: timer::timer_start(DOOR_OPEN_TIME) }
        }
    }
}

/**
 * -----------------------------------------------------Transitions from DoorOpen-----------------------------------------------------
 * 
 */
// DoorOpen->Moving
impl From<FSM<DoorOpen>> for FSM<Moving> {
    fn from(val: FSM<DoorOpen>) -> FSM<Moving> {
        //Figure out movement direction
        let dirn = elevio::DIRN_DOWN;
        val.hw.motor_direction(dirn);
        FSM {
            hw: val.hw,
            floor: val.floor,
            behaviour: Moving {drivingDirection: dirn}
        }
    }
}

/*
//DoorOpen->DoorOpen (obstruction signal OR currentfloor order button is pressed)
impl From<FSM<DoorOpen>> for FSM<DoorOpen> {
    fn from(val: FSM<DoorOpen>) -> FSM<DoorOpen> {
        let new_timer: timer::Timer = timer::timer_start(DOOR_OPEN_TIME);
        FSM {
            hw: val.hw,
            floor: val.floor,
            behaviour: DoorOpen { new_timer },
        }

    /* Restart timer */}
}
*/
impl FSM<DoorOpen> {
    fn restart_timer(val: FSM<DoorOpen>) -> FSM<DoorOpen> {
        FSM {
            hw: val.hw,
            floor: val.floor,
            behaviour: DoorOpen{door_timer: timer::timer_start(DOOR_OPEN_TIME)}
        }
    }
}

// DoorOpen->Idle
impl From<FSM<DoorOpen>> for FSM<Idle> {
    fn from(val: FSM<DoorOpen>) -> FSM<Idle> {
        val.hw.door_light(false);
        FSM {
            hw: val.hw,
            floor: val.floor,
            behaviour: Idle{},
        }
    }
}

/**
 * -----------------------------------------------------Transitions from Moving-----------------------------------------------------
 * Note: The elevator does not need to poll as the crossbeam channel will 
 * trigger the transition... maybe?
 */

//Moving->Idle (is this actually necessary?)
impl From<FSM<Moving>> for FSM<Idle> {
    fn from(val: FSM<Moving>) -> FSM<Idle> {
        let mut dirn = elevio::DIRN_STOP;
        val.hw.motor_direction(dirn);
        FSM {
            hw: val.hw,
            floor: val.floor,
            behaviour: Idle{},
        }
    }
}

// Moving-> DoorOpen
impl From<FSM<Moving>> for FSM<DoorOpen> {
    fn from(val: FSM<Moving>) -> FSM<DoorOpen> {
        /* Start timer, stop motor, open door */
        val.hw.motor_direction(e::DIRN_STOP);
        val.hw.door_light(true);
        let door_open_timer: timer::Timer = timer::timer_start(3);
        FSM {
            hw: val.hw,
            floor: val.floor,
            behaviour: DoorOpen { door_timer: door_open_timer }
        }
    }
}

/**
 * -----------------------------------------------------Helper functions-----------------------------------------------------
 */

//To be used inside transition from dooropen to moving/idle
fn setAllOrderLights(floor: u8) {/*...*/} 

fn notifyPeerInfo() {/* Some logic for sending a message that PeerInfo uses to update its info on the local elevator? */}


/* I think these might be unnecessary */
//fn fsm_onInitBetweenFloors() {/*...*/}
//fn fsm_onRequestButtonPress(btn_floor: u8, button_type: CallButton) {/*...*/}
//fn fsm_onFloorArrival(new_floor: u8) {/*...*/}
//fn fsm_onDoorTimeout() {/*...*/}
