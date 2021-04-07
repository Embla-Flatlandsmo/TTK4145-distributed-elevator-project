use crate::network::remote_elevator::RemoteElevatorUpdate;
use crate::elevio::poll::CallButton;
use crate::fsm::elevatorfsm::ElevatorInfo;
use crate::order_assigner::cost_function;
use std::collections::HashMap;

pub struct GlobalElevatorInfo {
    is_online: bool,
    local_elevator: ElevatorInfo,
    remote_elevators: HashMap<String, ElevatorInfo>
}

impl GlobalElevatorInfo {
    pub fn new(ref mut local_elev: ElevatorInfo) -> GlobalElevatorInfo {
        GlobalElevatorInfo {
            is_online: true,
            local_elevator: local_elev.clone(),
            remote_elevators: HashMap::new(),
        }
        }
    pub fn find_lowest_cost_id(&self, btn: CallButton) -> String {
        if self.is_online {
            return self.local_elevator.clone().get_id();
        }
        let mut lowest_cost_id: String = self.local_elevator.clone().get_id();
        let mut lowest_cost: usize = cost_function::time_to_idle(self.local_elevator, btn);
        for (id, elevinfo) in self.remote_elevators.iter() {
            let elev_cost = cost_function::time_to_idle(elevinfo.clone(), btn);
            if elev_cost < lowest_cost {
                lowest_cost_id = id.clone();
                lowest_cost = elev_cost;
            }
        }
        return lowest_cost_id;
    }

    pub fn update_remote_elevator_info(&mut self, remote_update: HashMap<String, ElevatorInfo>) {
        //TODO: Don't overwrite here...
        self.remote_elevators = remote_update;
    }

    pub fn update_local_elevator_info(&mut self, local_update: ElevatorInfo) {
        self.local_elevator = local_update;
    }

    pub fn set_to_pending(&mut self, id: &String, button: CallButton) {
        let mut elev_info: ElevatorInfo = *(self.remote_elevators.get(id).unwrap());
        elev_info.responsible_orders.set_pending(button);
        self.remote_elevators.insert(*id, elev_info);
    }

    pub fn is_pending(mut self, id: &str, button: CallButton) -> bool{
        let elev_info: ElevatorInfo = *(self.remote_elevators.get(id).unwrap());
        return elev_info.responsible_orders.is_pending(button)
    }
}

