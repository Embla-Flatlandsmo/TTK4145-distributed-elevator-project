#[derive(Clone,Copy,Debug)]
pub struct Timer {
    start_time: std::time::Instant,
    timeout_time: std::time::Duration,
    enabled: bool  
}
#[derive(Clone,Copy,Debug, PartialEq)]
pub enum TimerCommand {
    Start,
    Cancel,
}


impl Timer {
    pub fn new(timeout_sec: u64) -> Timer {
        Timer {
            start_time: std::time::Instant::now(),
            timeout_time: std::time::Duration::new(timeout_sec,0),
            enabled: false
        }
    }

    pub fn on_command(&mut self, command: TimerCommand) {
        match command {
            TimerCommand::Start => self.start(),
            TimerCommand::Cancel => self.cancel(),
        }
    } 

    fn start(&mut self) {
        self.start_time = std::time::Instant::now();
        self.enabled = true;
    }

    fn cancel(&mut self) {
        self.enabled = false;
    }

    pub fn did_expire(&mut self) -> bool {
        if (self.start_time.elapsed() > self.timeout_time) && self.enabled {
            self.enabled = false;
            return true;
        } else {
            return false;
        }
    }
}