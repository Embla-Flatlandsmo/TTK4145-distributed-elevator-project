
/*How to import module?*/
mod driver {
    pub mod elev;
    pub mod poll;
}

pub fn requests_chooseDirection(Elevator e) -> u8 {/* ... */}

pub fn requests_shouldStop(Elevator e) -> bool {/* ... */}

fn requests_clearAtCurrentFloor(Elevator e) -> Elevator {/* ... */}

fn requests_above(Elevator e) { /*...*/}

fn requests_below(Elevator e) {/*...*/}
