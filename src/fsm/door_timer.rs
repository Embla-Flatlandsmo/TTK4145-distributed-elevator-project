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
            did_notify: true
        }
    }

    pub fn start(&mut self) {
        self.start_time = std::time::Instant::now();
        self.did_notify = false;
    }

    pub fn did_expire(&mut self) -> bool {
        if (self.start_time.elapsed() > self.timeout_time) && !self.did_notify {
            self.did_notify = true;
            return true;
        } else {
            return false;
        }
    }
}