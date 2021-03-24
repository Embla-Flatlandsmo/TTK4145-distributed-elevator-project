
#[derive(Clone,Copy,Debug)]
struct TimedElevatorInfo {
    elev_info: ElevatorInfo,
    last_seen: Time::Instant
}

impl From<ElevatorInfo> for TimedElevatorInfo {
    fn from(elev: ElevatorInfo) -> Self {
        TimedElevatorInfo {
            elev_info: ElevatorInfo,
            last_seen: Time::Instant::now()
        }
    }
}

#[derive(Clone,Copy,Debug, serde::Serialize, serde::Deserialize))]
struct ElevatorInfo {
    id: u8
    state: State,
    dirn: u8,
    floor: u8,
    responsible_orders: order_list::OrderList,
}

#[derive(Clone,Copy,Debug)]
struct GlobalElevatorInfo {
    local_id: u8,
    global_elevator_info: Vec<TimedElevatorInfo>
}

impl GlobalElevatorInfo {
    pub fn new(local_info: ElevatorInfo) GlobalElevatorInfo {
        GlobalElevatorInfo {
            local_id: local_info.id,
            global_elevator_info: Vec::new(local_info),
        }
    }

    pub fn find_lowest_cost_id(&self) {
        for current_elevator_info in self.global_elevator_info.iter() {
            cost_function::time_to_idle() //todo: fix this
        }

    }

    pub fn on_remote_elevator_timed_out(&mut self, id: u8) {
        for elevator_info in self.global_elevator_info.iter() {
            if id = *elevator_info.id {

            }
        }
    }

    fn add_elevator(&mut self, elev_info: ElevatorInfo) {
        self.global_elevator_info.append(TimedElevatorInfo::from(elev_info));
    }


    pub fn update_elevator_info(&mut self, elev_info: ElevatorInfo) {
        let mut did_already_exist: bool = false;
        for current_elevator_info in self.global_elevator_info.iter_mut() {
            if *current_elevator_info.id == elev_info.id {
                *current_elevator_info = TimedElevatorInfo::from(elev_info);
                did_already_exist = true;
            }
        }
        if !did_already_exist {
            self.add_elevator(elev_info);
        }
    }

}

let mut glob_elev = GlobalElevatorInfo
glob_elev.add_elevator()
gloab_elev.send_own_info();