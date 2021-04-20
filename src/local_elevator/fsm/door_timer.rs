use crate::util::constants as setting;
use crossbeam_channel as cbc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimerCommand {
    Start,
    Cancel,
}

#[derive(Clone, Copy, Debug)]
struct Timer {
    start_time: std::time::Instant,
    timeout_time: std::time::Duration,
    enabled: bool,
}


impl Timer {
    fn new(timeout_sec: u64) -> Timer {
        Timer {
            start_time: std::time::Instant::now(),
            timeout_time: std::time::Duration::new(timeout_sec, 0),
            enabled: false,
        }
    }

    fn on_command(&mut self, command: TimerCommand) {
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

pub fn run(door_timer_start_rx: cbc::Receiver<TimerCommand>, door_timeout_tx: cbc::Sender<()>) {
    let mut door_timer: Timer = Timer::new(setting::DOOR_OPEN_TIME);
    loop {
        let r = door_timer_start_rx.try_recv();
        match r {
            Ok(r) => door_timer.on_command(r),
            _ => {}
        }
        if door_timer.did_expire() {
            door_timeout_tx.send(()).unwrap();
        }
    }
}
