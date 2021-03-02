use std::Time::{Duration,Instant};


pub struct Timer {
    start_time: Instant,
    duration: Duration
}

fn timer_start() -> Timer {/*...*/}
fn timer_isTimeout(timer: Timer) {/*...*/}