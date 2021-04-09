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
    pub fn find_lowest_cost_id(&self, btn: CallButton) -> usize {
        if self.is_online {
            return self.local_elevator.clone().get_id();
        }
        let mut lowest_cost_id: usize = self.local_elevator.clone().get_id();
        let mut lowest_cost: usize = cost_function::time_to_idle(self.local_elevator.clone(), btn);
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
        let mut latest_elevator_update: HashMap<usize, ElevatorInfo> = HashMap::new();
        for (id, remote_info) in remote_update {
            let mut current_info: ElevatorInfo;
            if self.remote_elevators.contains_key(&id) {
                match self.remote_elevators.clone().get_mut(&id) {
                    Some(v) => current_info = v.clone(),
                    None => return
                }
                current_info.responsible_orders.merge_remote_orders(remote_info.responsible_orders);
            } else {
                current_info = remote_info;
            }
            latest_elevator_update.insert(id, current_info);
        }
        self.remote_elevators = latest_elevator_update;
    }

    pub fn update_local_elevator_info(&mut self, local_update: ElevatorInfo) {
        self.local_elevator = local_update;
    }

    pub fn set_to_pending(&mut self, id: &String, button: CallButton) {
        let mut elev_info: ElevatorInfo;
        match self.remote_elevators.get_mut(id) {
            Some(v) => elev_info = v.clone(),
            None => return
        }
        elev_info.responsible_orders.set_pending(button);
        self.remote_elevators.insert((*id).clone(), elev_info);
    }

    pub fn is_pending(&self, id: &String, button: CallButton) -> bool{
        let elev_info: ElevatorInfo;
        match self.remote_elevators.clone().get_mut(id) {
            Some(v) => elev_info = v.clone(),
            None => return false
        }
        return elev_info.responsible_orders.is_pending(button)
    }
    pub fn get_remote_elevators(&self) -> HashMap<String, ElevatorInfo> {
        return self.clone().remote_elevators.clone();
    }

    pub fn get_local_elevator_info(&self) -> ElevatorInfo {
        return self.clone().local_elevator.clone();
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::elevio::poll::CallButton;
    use crate::fsm::elevatorfsm::State;
    use crate::order_manager::order_list::OrderList;
    use crate::order_manager::order_list::OrderType;
    use crate::elevio::elev::*;
    extern crate rand;
    fn create_random_remote_order_list() -> OrderList {
        let n_floors=5;
        let mut order_list = OrderList::new(n_floors);
        for i in 0..n_floors-1 {
            if rand::random() {
                order_list.add_order(CallButton {floor: i as u8, call: 0})
            }

        }
        for i in 0..n_floors-1 {
            if rand::random() {
                order_list.add_order(CallButton {floor: i as u8, call: 1})
            }

        }
        return order_list;
    }

    fn create_random_remote_update() -> HashMap<String, ElevatorInfo> {
        let elev_id_0: ElevatorInfo = ElevatorInfo{
            id: "0".to_string(),
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: create_random_remote_order_list()
        };
        let elev_id_1: ElevatorInfo = ElevatorInfo{
            id: "1".to_string(),
            state: State::Moving,
            dirn: DIRN_DOWN,
            floor: 1,
            responsible_orders: create_random_remote_order_list()
        };
        let mut remote_update: HashMap<String, ElevatorInfo> = HashMap::new();
        remote_update.insert("0".to_string(), elev_id_0);
        remote_update.insert("1".to_string(), elev_id_1);
        return remote_update;
    }

    fn create_empty_remote_update() -> HashMap<String, ElevatorInfo> {
        let elev_id_0: ElevatorInfo = ElevatorInfo{
            id: "0".to_string(),
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5)
        };
        let elev_id_1: ElevatorInfo = ElevatorInfo{
            id: "1".to_string(),
            state: State::Moving,
            dirn: DIRN_DOWN,
            floor: 1,
            responsible_orders: OrderList::new(5)
        };
        let mut remote_update: HashMap<String, ElevatorInfo> = HashMap::new();
        remote_update.insert("0".to_string(), elev_id_0);
        remote_update.insert("1".to_string(), elev_id_1);
        return remote_update;
    }

    #[test]
    fn it_updates_from_remote(){
        let remote_update: HashMap<String, ElevatorInfo> = create_random_remote_update();
        let local_elev_info: ElevatorInfo = ElevatorInfo{
            id: "2".to_string(),
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5)
        };
        let mut global_elevator_info: GlobalElevatorInfo = GlobalElevatorInfo::new(local_elev_info);
        global_elevator_info.update_remote_elevator_info(remote_update.clone());
        assert_eq!(remote_update, global_elevator_info.get_remote_elevators());
    }

    #[test]
    fn it_correctly_sets_pending_in_remote_elevators() {
        let local_elev_info: ElevatorInfo = ElevatorInfo{
            id: "2".to_string(),
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5)
        };
        let mut global_elevator_info: GlobalElevatorInfo = GlobalElevatorInfo::new(local_elev_info);
        global_elevator_info.update_remote_elevator_info(create_empty_remote_update());
        global_elevator_info.set_to_pending(&"0".to_string(), CallButton{floor: 2, call: 1});
        assert!(global_elevator_info.is_pending(&"0".to_string(), CallButton{floor: 2, call: 1}));
    }

    #[test]
    fn it_correctly_updates_remote_pending_to_active() {
        let local_elev_info: ElevatorInfo = ElevatorInfo{
            id: "2".to_string(),
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5)
        };

        let mut global_elevator_info: GlobalElevatorInfo = GlobalElevatorInfo::new(local_elev_info);
        let mut remote_update = create_empty_remote_update();
        global_elevator_info.update_remote_elevator_info(remote_update.clone());
        global_elevator_info.set_to_pending(&"0".to_string(), CallButton{floor: 2, call: 1});
        let mut elev_info: ElevatorInfo;
        match remote_update.clone().get_mut(&"0".to_string()) {
            Some(v) => elev_info = v.clone(),
            None => return
        }
        elev_info.responsible_orders.add_order(CallButton{floor: 2, call: 1});
        remote_update.insert("0".to_string(), elev_info);
        global_elevator_info.update_remote_elevator_info(remote_update.clone());
        assert!(!global_elevator_info.is_pending(&"0".to_string(), CallButton{floor: 2, call: 1}));
    }
    
    #[test]
    fn it_correctly_updates_local_elevator() {
        let local_elev_info: ElevatorInfo = ElevatorInfo{
            id: "2".to_string(),
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5)
        };

        let mut global_elevator_info: GlobalElevatorInfo = GlobalElevatorInfo::new(local_elev_info);
        global_elevator_info.update_remote_elevator_info(create_random_remote_update());

        let mut new_local_elev_info: ElevatorInfo =ElevatorInfo{
            id: "2".to_string(),
            state: State::Moving,
            dirn: DIRN_DOWN,
            floor: 1,
            responsible_orders: OrderList::new(5)
        };
        new_local_elev_info.responsible_orders.add_order(CallButton{floor: 2, call: 1});

        global_elevator_info.update_local_elevator_info(new_local_elev_info.clone());
        assert_eq!(global_elevator_info.get_local_elevator_info(), new_local_elev_info);
    }
}