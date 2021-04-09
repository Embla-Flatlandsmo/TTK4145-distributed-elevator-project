use crate::elevio::poll::CallButton;
use crate::fsm::elevatorfsm::ElevatorInfo;
use crate::order_assigner::cost_function;

pub struct GlobalElevatorInfo {
    local_id: usize,
    global_elevators: Vec<Option<ElevatorInfo>>
}

impl GlobalElevatorInfo {
    pub fn new(mut local_elev: ElevatorInfo, max_number_of_elevators: usize) -> GlobalElevatorInfo {
        let mut global_elevs: Vec<Option<ElevatorInfo>> = Vec::new();
        global_elevs.resize_with(max_number_of_elevators, || None);
        global_elevs[local_elev.clone().get_id()] = Some(local_elev.clone());
        GlobalElevatorInfo {
            local_id: local_elev.get_id(),
            global_elevators: global_elevs,
            }
        }

    pub fn find_lowest_cost_id(&self, btn: CallButton) -> usize {
        let mut lowest_cost: usize = cost_function::time_to_idle(self.get_local_elevator_info(), btn);
        let mut lowest_cost_id: usize = self.local_id;

        for i in self.global_elevators.iter() {
            let mut elev_cost: usize = usize::MAX;
            match (*i).clone() {
                Some(val) => {
                    elev_cost = cost_function::time_to_idle((val).clone(), btn);
                    if elev_cost < lowest_cost {
                        lowest_cost_id = (val).id;
                        lowest_cost = elev_cost;
                    }},
                None => {}
            }
        }
        return lowest_cost_id;
    }

    /// Updates global info with the newest info received from remote elevators.
    pub fn update_remote_elevator_info(&mut self, remote_update: Vec<ElevatorInfo>) {
        let mut latest_elevator_update: Vec<Option<ElevatorInfo>> = Vec::new();
        latest_elevator_update.resize_with(self.global_elevators.len(), || None);
        latest_elevator_update[self.local_id] = self.global_elevators[self.local_id].clone();

        for elev in remote_update.iter() {
            let remote_info =  (*elev).clone();
            let remote_id = remote_info.clone().id;
            if remote_id != self.local_id {
                let mut updated_info: ElevatorInfo;
                match self.global_elevators[remote_id].as_ref() {
                    Some(v) => {
                        updated_info = v.clone();
                        updated_info.responsible_orders.merge_remote_orders(remote_info.responsible_orders);
                        latest_elevator_update[remote_id] = Some(updated_info);},
                    None => {latest_elevator_update[remote_id] = Some(remote_info)}
                }
            }
        }
        self.global_elevators = latest_elevator_update;
    }

    pub fn update_local_elevator_info(&mut self, local_update: ElevatorInfo) {
        self.global_elevators[self.local_id] = Some(local_update);
    }

    pub fn set_to_pending(&mut self, id: usize, button: CallButton) {
        let mut elev_info: ElevatorInfo;
        match self.global_elevators[id].as_ref() {
            Some(v) => elev_info = v.clone(),
            None => return
        }
        elev_info.responsible_orders.set_pending(button);
        self.global_elevators[id] = Some(elev_info);
    }

    pub fn is_pending(&self, id: usize, button: CallButton) -> bool{
        let elev_info: ElevatorInfo;
        match self.global_elevators[id].as_ref() {
            Some(v) => elev_info = v.clone(),
            None => return false,
        }
        return elev_info.responsible_orders.is_pending(button)
    }
    pub fn get_global_elevators(&self) -> Vec<Option<ElevatorInfo>> {
        return self.clone().global_elevators.clone();
    }

    pub fn get_local_elevator_info(&self) -> ElevatorInfo {
        return self.clone().global_elevators[self.local_id].clone().unwrap();
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
    fn create_random_remote_order_list(max_number_of_elevators: usize) -> OrderList {
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

    fn create_random_remote_update(max_number_of_elevators: usize) -> Vec<ElevatorInfo> {
        let elev_id_0: ElevatorInfo = ElevatorInfo{
            id: 0,
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: create_random_remote_order_list(max_number_of_elevators)
        };
        let elev_id_1: ElevatorInfo = ElevatorInfo{
            id: 1,
            state: State::Moving,
            dirn: DIRN_DOWN,
            floor: 1,
            responsible_orders: create_random_remote_order_list(max_number_of_elevators)
        };
        let mut remote_update: Vec<ElevatorInfo> = Vec::new();
        remote_update.push(elev_id_0);
        remote_update.push(elev_id_1);
        return remote_update;
    }

    fn create_empty_remote_update(max_number_of_elevators: usize) -> Vec<ElevatorInfo> {
        let elev_id_0: ElevatorInfo = ElevatorInfo{
            id: 0,
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5)
        };
        let elev_id_1: ElevatorInfo = ElevatorInfo{
            id: 1,
            state: State::Moving,
            dirn: DIRN_DOWN,
            floor: 1,
            responsible_orders: OrderList::new(5)
        };
        let mut remote_update: Vec<ElevatorInfo> = Vec::new();
        remote_update.push(elev_id_0);
        remote_update.push(elev_id_1);
        return remote_update;
    }

    #[test]
    fn it_updates_from_remote(){
        let max_num_elev = 10;
        let remote_update: Vec<ElevatorInfo> = create_random_remote_update(max_num_elev);
        let local_elev_info: ElevatorInfo = ElevatorInfo{
            id: 2,
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5)
        };
        let mut global_elevator_info: GlobalElevatorInfo = GlobalElevatorInfo::new(local_elev_info, 10);
        global_elevator_info.update_remote_elevator_info(remote_update.clone());
        let is_identical = true;
        for elev in remote_update.iter() {
            if *elev != global_elevator_info.get_global_elevators()[elev.id].unwrap() {
                is_identical = false;
            }
        }
        assert!(is_identical);
    }

    #[test]
    fn it_correctly_sets_pending_in_remote_elevators() {
        let max_num_elev: usize = 10;
        let local_elev_info: ElevatorInfo = ElevatorInfo{
            id: 2,
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5)
        };
        let mut global_elevator_info: GlobalElevatorInfo = GlobalElevatorInfo::new(local_elev_info, max_num_elev);
        global_elevator_info.update_remote_elevator_info(create_empty_remote_update(max_num_elev));
        global_elevator_info.set_to_pending(0, CallButton{floor: 2, call: 1});
        assert!(global_elevator_info.is_pending(0, CallButton{floor: 2, call: 1}));
    }
    
    #[test]
    fn it_correctly_updates_remote_pending_to_active() {
        let max_num_elev = 10;
        let local_elev_info: ElevatorInfo = ElevatorInfo{
            id: 2,
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5)
        };

        let mut global_elevator_info: GlobalElevatorInfo = GlobalElevatorInfo::new(local_elev_info, max_num_elev);
        let mut remote_update = create_empty_remote_update(max_num_elev);
        global_elevator_info.update_remote_elevator_info(remote_update.clone());
        global_elevator_info.set_to_pending(0, CallButton{floor: 2, call: 1});
        remote_update[0].responsible_orders.add_order(CallButton{floor: 2, call: 1});
        global_elevator_info.update_remote_elevator_info(remote_update.clone());
        assert!(!global_elevator_info.is_pending(0, CallButton{floor: 2, call: 1}));
    }
}