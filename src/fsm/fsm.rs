enum ElevatorBehaviour {
    Idle,
    Moving,
    DoorOpen
}

struct Elevator {
    behaviour: ElevatorBehaviour,
    hw: ElevatorHW,
    floor: i32,
    //orders : [num_floors][call_buttons],
}

fn chooseDirection(Elevator e) {/* ... */}

fn shouldStop(Elevator e) {/* ... */}

fn clearRequests(Elevator e) {/* ... */}