use std::time::Duration;
use std::time::Instant;
use std::thread;
use crossbeam_channel as cbc;

#[derive(Clone,Copy,Debug)]
pub struct Timer {
    start_time: std::time::Instant,
    timeout_time: std::time::Duration,
    did_notify: bool  
}

impl Timer {
    pub fn new(timeout_sec: u64) -> Timer {
        Timer {
            start_time: std::time::Instant::now(),
            timeout_time: std::time::Duration::new(timeout_sec,0),
            did_notify: false
        }
    }

    pub fn start(mut self) {
        self.start_time = std::time::Instant::now();
        self.did_notify = false;
    }

    pub fn did_expire(self) -> bool {
        if (self.start_time.elapsed() > self.timeout_time) && !self.did_notify {
            return true;
        } else {
            return false;
        }
    }
}