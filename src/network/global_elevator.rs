use crate::network::remote_elevator::RemoteElevatorUpdate;
use crate::elevio::poll::CallButton;

pub struct GlobalElevatorInfo {
    is_online: bool,
    local_elevator: ElevatorInfo,
    remote_elevators: HashMap<String, ElevatorInfo>
}

impl GlobalElevatorInfo {
    pub fn new(ref mut local_elev: ElevatorInfo) -> GlobalElevatorInfo {
        GlobalElevatorInfo {
            is_online: true,
            local_elevator: local_elev,
            remote_elevators: Vec::new(),
        }
        }
    pub fn find_lowest_cost_id(&self, btn: CallButton) -> String {
        if self.is_online {
            return self.local_elevator.clone().id.clone();
        }
        let mut lowest_cost_id: String = self.local_elevator.id.clone();
        let mut lowest_cost: usize = cost_function::time_to_idle(self.local_elevator, btn);
        for elev in remote_elevators.iter() {
            let elev_cost = cost_function::time_to_idle(elev);
            if elev_cost < lowest_cost {
                lowest_cost_id = elev.clone().id.clone();
                lowest_cost = elev_cost;
            }
        }
        return lowest_cost_id;
    }

    pub fn update_remote_elevator_info(&mut self, remote_update: RemoteElevatorUpdate) {
        for update in remote_update.clone().peers.iter() {
            
        }

        for lost_elev in remote_update.clone().lost.iter() {
            remote_elevators.remove(lost_elev.id)
        }


        if remote_update.clone().new != None {

        }

    }
}