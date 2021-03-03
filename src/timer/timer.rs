use std::Time::{Duration,Instant};

/**
 * Here: Maybe we need to introduce some way to spawn a thread in timer_start?
 * */

pub struct Timer {
    start_time: Instant,
    duration: Duration
}

fn timer_start() -> Timer {/*...*/}
fn timer_isTimeout(timer: Timer) {/* Send message on the channel */}