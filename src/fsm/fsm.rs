/*enum ElevatorBehaviour {
    Idle,
    Moving,
    DoorOpen,
    Initializing
}
*/

#[path = "../elevio/elev.rs"] mod elevio;
#[path = "../timer/timer.rs"] mod timer;

struct FSM<ElevatorBehaviour> {
    hw: elevio::ElevatorHW,
    pub floor: u8,
    pub behaviour: ElevatorBehaviour,
}

impl FSM<Initializing> {
    fn initialize(e: elevio::ElevatorHW) -> Self {
        FSM {
            hw: e,
            floor: 0,
            behaviour: Initializing::new(),
        }
    }
}

struct Initializing {}

// Some weird initialization stuff, idk...
impl Initializing {
    fn new() -> Self {



        Initializing{}
    }
}
struct Moving {
    drivingDirection: u8
}

// Transition from initialize to moving
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

// Transition from idle to moving
impl From<FSM<Idle>> for FSM<Moving> {
    fn from(val: FSM<Idle>, dirn: u8) -> FSM<Moving> {
        // Todo: Disallow any value but DIRN_DOWN/STOP/UP
        val.hw.motor_direction(dirn);
        FSM {
            hw: val.hw,
            floor: val.floor,
            behaviour: Moving {drivingDirection: dirn}
        }
    }
}

// Transition from idle to DoorOpen
impl From<FSM<Idle>> for FSM<DoorOpen> {
    fn from(val: FSM<Idle>) -> FSM<DoorOpen> {
        timer: timer::timer_start();
        val.hw.door_light(true);
        //val.hw.call_button_light(floor: u8, call: u8, on: bool); //Some function to clear light
        FSM {
            hw: val.hw,
            floor: val.floor,
            behaviour: DoorOpen { timer }
        }
    }
}

// Transition from door open to moving

impl From<FSM<DoorOpen>> for FSM<Moving> {
    fn from(val: FSM<DoorOpen>, dirn: u8) -> FSM<Moving> {
        val.hw.motor_direction(dirn);
        FSM {
            hw: val.hw,
            floor: val.floor,
            behaviour: Moving {drivingDirection: dirn}
        }
    }
}

struct Idle {}

/**
 * Transition from Moving to Idle
 * Note: The elevator does not need to poll as the crossbeam channel will trigger the transition
 */
impl From<FSM<Moving>> for FSM<Idle> {
    fn from(val: FSM<Moving>) -> FSM<Idle> {
        let mut dirn = elevio::DIRN_STOP;
        val.hw.motor_direction(dirn);
        FSM {
            hw: val.hw,
            floor: val.floor,
            behaviour: Idle,
        }
    }
}

struct DoorOpen {
    timer: timer::Timer
}

// Transition from DoorOpen to Idle 
impl From<FSM<DoorOpen>> for FSM<Idle> {
    fn from(val: FSM<Idle>) -> FSM<Moving> {
        timer: timer::timer_start();
        val.hw.door_light(true);
        FSM {
            hw: val.hw,
            floor: val.floor,
            behaviour: DoorOpen { timer }
        }
    }
}

// Transition from DoorOpen to DoorOpen (obstruction signal OR button is pressed)
impl From<FSM<DoorOpen>> for FSM<DoorOpen> {
    fn from(val: FSM<DoorOpen>) -> FSM<DoorOpen> {

    }
}

// Transition from DoorOpen to Moving
impl From<FSM<DoorOpen>> for FSM<Moving> {
    fn from(val: FSM<DoorOpen>)
}


fn setAllLights(e: elevio::ElevatorHW) {/*...*/}

// I think these might be unnecessary
//fn fsm_onInitBetweenFloors() {/*...*/}
//fn fsm_onRequestButtonPress(btn_floor: u8, button_type: CallButton) {/*...*/}
//fn fsm_onFloorArrival(new_floor: u8) {/*...*/}
//fn fsm_onDoorTimeout() {/*...*/}
