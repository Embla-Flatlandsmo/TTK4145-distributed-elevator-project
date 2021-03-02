enum ElevatorBehaviour {
    Idle,
    Moving,
    DoorOpen
}

struct Elevator {
    hw: ElevatorHW,
    pub floor: u8,
    behaviour: ElevatorBehaviour,
    orders : [num_floors][call_buttons],
}

impl Elevator {
    init 
}
fn setAllLights(Elevator e) {/*...*/}
fn fsm_onInitBetweenFloors() {/*...*/}
fn fsm_onRequestButtonPress(btn_floor: u8, button_type: CallButton) {/*...*/}
fn fsm_onFloorArrival(new_floor: u8) {/*...*/}
fn fsm_onDoorTimeout() {/*...*/}
