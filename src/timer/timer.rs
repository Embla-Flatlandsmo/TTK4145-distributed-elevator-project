
use super::std::time::Duration;
use super::std::time::Instant;
use std::thread;
use crossbeam_channel as cbc;


/**
 * Here: Maybe we need to introduce some way to spawn a thread in timer_start?
 * */
pub struct Timer {
    start_time: time::Instant,
    pub duration: time::Duration,
}

/// Determines if the given timer is timed out, then sends a message on the given channel.
/// * `timer` - the timer you want to see if has timed out
/// * `ch` - The channel on which you want to send the message
/// * `period` - How often the function should be checked.
pub fn timer_isTimeout(timer: Timer, ch: cbc::Sender<bool>, period: time::Duration) {
    let mut prev = false;
    loop {
        let v = prev;
        if time::Instant::now() > timer.start_time+timer.duration {
            v = true; 
        }
        if prev != v {
            ch.send(v).unwrap();
            prev = v;
        }
        thread::sleep(period);
    }
}

pub fn timer_start(timer_duration: u64) -> Timer {
    let start = time::Instant::now();
    let dur = Time::Duration::new(timer_duration,0);
    Timer {
        start_time: Instant::now(),
        duration: dur,
    }
}

//fn timer_isTimeout(timer: Timer) {/* Send message on the channel */}