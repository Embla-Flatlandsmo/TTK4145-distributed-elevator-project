use crate::elevio::poll::CallButton;
use crate::elevio::elev::HardwareCommand;
use crate::fsm::elevatorfsm::ElevatorInfo;
use crate::order_assigner::cost_function;
use crate::order_manager::order_list::{OrderList, OrderType};
use crossbeam_channel as cbc;
use std::time;

#[derive(Clone, Debug)]
pub struct GlobalElevatorInfo {
    local_id: usize,
    global_elevators: Vec<Option<ElevatorInfo>>,
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
        let mut lowest_cost: usize =
            cost_function::time_to_idle(self.get_local_elevator_info(), btn);
        let mut lowest_cost_id: usize = self.local_id;

        for i in self.global_elevators.iter().cloned() {
            let mut elev_cost: usize = usize::MAX;
            match i.clone() {
                Some(val) => {
                    elev_cost = cost_function::time_to_idle((val).clone(), btn);
                    if elev_cost < lowest_cost {
                        lowest_cost_id = (val).id;
                        lowest_cost = elev_cost;
                    }
                }
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
            let remote_info = (*elev).clone();
            let remote_id = remote_info.clone().id;
            if remote_id != self.local_id {
                let mut updated_info: ElevatorInfo;
                match self.global_elevators[remote_id].as_ref() {
                    Some(v) => {
                        updated_info = v.clone();
                        updated_info.responsible_orders = merge_remote_orders(
                            v.clone().responsible_orders,
                            remote_info.responsible_orders,
                        );
                        latest_elevator_update[remote_id] = Some(updated_info);
                    }
                    None => latest_elevator_update[remote_id] = Some(remote_info),
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
            None => return,
        }
        elev_info.responsible_orders.set_pending(button);
        self.global_elevators[id] = Some(elev_info);
    }

    pub fn is_pending(&self, id: usize, button: CallButton) -> bool {
        let elev_info: ElevatorInfo;
        match self.global_elevators[id].as_ref() {
            Some(v) => elev_info = v.clone(),
            None => return false,
        }
        return elev_info.responsible_orders.is_pending(button);
    }

    pub fn get_orders_for_lights(&self) -> OrderList {
        let num_floors = self.get_local_elevator_info().responsible_orders.inside_queue.len();
        let mut order_lights: OrderList = OrderList::new(num_floors as u8);
        for entry in self.global_elevators.iter().cloned() {
            match entry {
                Some(elev) => {
                    order_lights =
                        merge_remote_active(order_lights.clone(), elev.responsible_orders)
                }
                None => {}
            }
        }
        order_lights.inside_queue = self.global_elevators[self.local_id].clone().unwrap().responsible_orders.inside_queue;
        return order_lights;
    }

    pub fn get_global_elevators(&self) -> Vec<Option<ElevatorInfo>> {
        return self.clone().global_elevators.clone();
    }

    pub fn get_local_elevator_info(&self) -> ElevatorInfo {
        return self.clone().global_elevators[self.local_id]
            .clone()
            .unwrap();
    }
}

pub fn global_elevator_info(
    local_update: cbc::Receiver<ElevatorInfo>, 
    remote_update: cbc::Receiver<Vec<ElevatorInfo>>,
    set_pending: cbc::Receiver<(usize, CallButton)>, 
    global_info_update: cbc::Sender<GlobalElevatorInfo>) {

    let mut global_info: GlobalElevatorInfo;

    match local_update.recv() {
        Ok(res) => global_info = GlobalElevatorInfo::new(res, 10),
        _ => {panic!("Cannot initialize global info")}
    }

    let ticker = cbc::tick(time::Duration::from_millis(15));

    loop {
        cbc::select! {
            recv(local_update) -> a => {
                let local_info = a.unwrap();
                global_info.update_local_elevator_info(local_info);
                global_info_update.send(global_info.clone()).unwrap();
            },
            recv(remote_update) -> a => {
                let remote_info = a.unwrap();
                global_info.update_remote_elevator_info(remote_info);
                global_info_update.send(global_info.clone()).unwrap()
            },
            recv(ticker) -> _ => {
                //global_info_update.send(global_info.clone()).unwrap();
            },
            recv(set_pending) -> (a) => {
                let (id, btn) = a.unwrap();
                global_info.set_to_pending(id, btn);
                global_info_update.send(global_info.clone()).unwrap();
            }
        }
    }
}

pub fn set_order_lights(global_info_rx: cbc::Receiver<GlobalElevatorInfo>, set_lights_tx: cbc::Sender<HardwareCommand>) {
    let elev_num_floors = 4;
    let mut old_lights: OrderList = OrderList::new(elev_num_floors);
    loop {
        cbc::select! {
            recv(global_info_rx) -> a => {
                let global_info = a.unwrap();
                let set_lights = global_info.get_orders_for_lights();
                for f in 0..elev_num_floors {
                    for c in 0..3 {
                        let btn = CallButton{floor: f, call: c};
                        if old_lights.is_active(btn) != set_lights.is_active(btn) {
                            set_lights_tx.send(
                                HardwareCommand::CallButtonLight{floor:btn.floor, call: btn.call, on: set_lights.is_active(btn)}).unwrap();
                        }
                    }
                }
                old_lights = set_lights;
            },
        }
    }
}




fn merge_remote_active(local_order_info: OrderList, remote_orders: OrderList) -> OrderList {
    let n_floors: usize = local_order_info.up_queue.len();
    let mut new_order_list: OrderList = OrderList::new(n_floors as u8);
    if n_floors != remote_orders.up_queue.len() {
        panic!("Tried to merge elevator orders of different lengths :(");
    }

    for i in 0..=n_floors - 1 {
        if local_order_info.up_queue[i] == OrderType::Active
            || remote_orders.up_queue[i] == OrderType::Active
        {
            new_order_list.up_queue[i] = OrderType::Active
        }
        if local_order_info.down_queue[i] == OrderType::Active
            || remote_orders.down_queue[i] == OrderType::Active
        {
            new_order_list.down_queue[i] = OrderType::Active
        }
    }
    return new_order_list;
}

/// Merges remote order into current order. It prioritizes remote order.
/// A pending order can only be upgraded to active by `remote_order`.
fn merge_remote_order(current_order: OrderType, remote_order: OrderType) -> OrderType {
    match current_order {
        OrderType::Pending => {
            if remote_order == OrderType::Active {
                return remote_order;
            } else {
                return OrderType::Pending;
            }
        }
        _ => remote_order,
    }
}

/// Creates new list with values of a remote order list, but preserves pending orders in the local list
///
/// *`local_order_info` - The local knowledge of the orderlist of a remote elevator
/// *`remote_orders` - The update received from the remote elevator
fn merge_remote_orders(local_order_info: OrderList, remote_orders: OrderList) -> OrderList {
    let n_floors: usize = local_order_info.up_queue.len();
    let mut new_order_list: OrderList = OrderList::new(n_floors as u8);
    new_order_list.inside_queue = local_order_info.inside_queue.clone();
    if n_floors != remote_orders.up_queue.len() {
        panic!("Tried to merge elevator orders of different lengths :(");
    }

    for i in 0..=n_floors - 1 {
        new_order_list.up_queue[i] =
            merge_remote_order(local_order_info.up_queue[i], remote_orders.up_queue[i]);
        new_order_list.down_queue[i] = merge_remote_order(
            local_order_info.down_queue[i],
            remote_orders.down_queue[i],
        );
        new_order_list.inside_queue[i] = merge_remote_order(
            local_order_info.inside_queue[i],
            remote_orders.inside_queue[i],
        );
    }
    return new_order_list;
}

/// Sets both Active and Pending hall orders of `orders` to Active in its own list.
/// Creates an OrderList that contains
///
/// * `local_orders` - OrderList to service
fn service_hall_orders(local_elev_orders: &OrderList, remote_orders: &OrderList) -> OrderList {
    let n_floors: usize = local_elev_orders.up_queue.len();
    let mut new_order_list: OrderList = OrderList::new(n_floors as u8);
    new_order_list.inside_queue = local_elev_orders.inside_queue.clone();
    if n_floors != remote_orders.up_queue.len() {
        panic!("Tried to merge elevator orders of different lengths :(");
    }

    for i in 0..=n_floors - 1 {
        if local_elev_orders.up_queue[i] == OrderType::Active
            || remote_orders.up_queue[i] == OrderType::Active
            || remote_orders.up_queue[i] == OrderType::Pending
        {
            new_order_list.up_queue[i] = OrderType::Active;
        }
        if local_elev_orders.down_queue[i] == OrderType::Active
            || remote_orders.down_queue[i] == OrderType::Active
            || remote_orders.down_queue[i] == OrderType::Pending
        {
            new_order_list.down_queue[i] = OrderType::Active;
        }
    }

    return new_order_list;
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::elevio::elev::*;
    use crate::elevio::poll::CallButton;
    use crate::fsm::elevatorfsm::State;
    use crate::order_manager::order_list::OrderList;
    use crate::order_manager::order_list::OrderType;
    extern crate rand;
    fn create_random_remote_order_list(max_number_of_elevators: usize) -> OrderList {
        let n_floors = 5;
        let mut order_list = OrderList::new(n_floors);
        for i in 0..n_floors - 1 {
            if rand::random() {
                order_list.set_active(CallButton {
                    floor: i as u8,
                    call: 0,
                })
            }
        }
        for i in 0..n_floors - 1 {
            if rand::random() {
                order_list.set_active(CallButton {
                    floor: i as u8,
                    call: 1,
                })
            }
        }
        return order_list;
    }

    fn create_random_remote_update(max_number_of_elevators: usize) -> Vec<ElevatorInfo> {
        let elev_id_0: ElevatorInfo = ElevatorInfo {
            id: 0,
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: create_random_remote_order_list(max_number_of_elevators),
        };
        let elev_id_1: ElevatorInfo = ElevatorInfo {
            id: 1,
            state: State::Moving,
            dirn: DIRN_DOWN,
            floor: 1,
            responsible_orders: create_random_remote_order_list(max_number_of_elevators),
        };
        let mut remote_update: Vec<ElevatorInfo> = Vec::new();
        remote_update.push(elev_id_0);
        remote_update.push(elev_id_1);
        return remote_update;
    }

    fn create_empty_remote_update(max_number_of_elevators: usize) -> Vec<ElevatorInfo> {
        let elev_id_0: ElevatorInfo = ElevatorInfo {
            id: 0,
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5),
        };
        let elev_id_1: ElevatorInfo = ElevatorInfo {
            id: 1,
            state: State::Moving,
            dirn: DIRN_DOWN,
            floor: 1,
            responsible_orders: OrderList::new(5),
        };
        let mut remote_update: Vec<ElevatorInfo> = Vec::new();
        remote_update.push(elev_id_0);
        remote_update.push(elev_id_1);
        return remote_update;
    }

    #[test]
    fn it_updates_from_remote() {
        let max_num_elev = 10;
        let remote_update: Vec<ElevatorInfo> = create_random_remote_update(max_num_elev);
        let local_elev_info: ElevatorInfo = ElevatorInfo {
            id: 2,
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5),
        };
        let mut global_elevator_info: GlobalElevatorInfo =
            GlobalElevatorInfo::new(local_elev_info, 10);
        global_elevator_info.update_remote_elevator_info(remote_update.clone());
        let mut is_identical = true;
        for elev in remote_update.iter().cloned() {
            let elev_id: usize = elev.id;
            if elev
                != global_elevator_info.get_global_elevators()[elev_id]
                    .clone()
                    .unwrap()
            {
                is_identical = false;
            }
        }
        assert!(is_identical);
    }

    #[test]
    fn it_correctly_sets_pending_in_remote_elevators() {
        let max_num_elev: usize = 10;
        let local_elev_info: ElevatorInfo = ElevatorInfo {
            id: 2,
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5),
        };
        let mut global_elevator_info: GlobalElevatorInfo =
            GlobalElevatorInfo::new(local_elev_info, max_num_elev);
        global_elevator_info.update_remote_elevator_info(create_empty_remote_update(max_num_elev));
        global_elevator_info.set_to_pending(0, CallButton { floor: 2, call: 1 });
        assert!(global_elevator_info.is_pending(0, CallButton { floor: 2, call: 1 }));
    }
    #[test]
    fn it_correctly_updates_remote_pending_to_active() {
        let max_num_elev = 10;
        let local_elev_info: ElevatorInfo = ElevatorInfo {
            id: 2,
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5),
        };

        let mut global_elevator_info: GlobalElevatorInfo =
            GlobalElevatorInfo::new(local_elev_info, max_num_elev);
        let mut remote_update = create_empty_remote_update(max_num_elev);
        global_elevator_info.update_remote_elevator_info(remote_update.clone());
        global_elevator_info.set_to_pending(0, CallButton { floor: 2, call: 1 });
        remote_update[0]
            .responsible_orders
            .set_active(CallButton { floor: 2, call: 1 });
        global_elevator_info.update_remote_elevator_info(remote_update.clone());
        assert!(!global_elevator_info.is_pending(0, CallButton { floor: 2, call: 1 }));
    }

    #[test]
    fn it_correctly_updates_local() {
        let max_num_elev = 10;
        let local_elev_info: ElevatorInfo = ElevatorInfo {
            id: 2,
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5),
        };

        let mut global_elevator_info: GlobalElevatorInfo =
            GlobalElevatorInfo::new(local_elev_info, max_num_elev);

        let local_elev_info_2: ElevatorInfo = ElevatorInfo {
            id: 2,
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 4,
            responsible_orders: OrderList::new(5),
        };

        global_elevator_info.update_local_elevator_info(local_elev_info_2.clone());
        assert_eq!(
            global_elevator_info.get_local_elevator_info(),
            local_elev_info_2
        );
    }

    #[test]
    fn it_does_not_overwrite_local() {
        let max_num_elev = 10;
        let local_elev_info: ElevatorInfo = ElevatorInfo {
            id: 2,
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5),
        };
        // Attempt to overwrite local with 'bad' remote info
        let bad_remote_elev_info: ElevatorInfo = ElevatorInfo {
            id: 2,
            state: State::Idle,
            dirn: DIRN_UP,
            floor: 1,
            responsible_orders: OrderList::new(5),
        };
        let mut remote_update: Vec<ElevatorInfo> = Vec::new();
        remote_update.push(bad_remote_elev_info);
        let mut global_elevator_info: GlobalElevatorInfo =
            GlobalElevatorInfo::new(local_elev_info.clone(), 10);
        global_elevator_info.update_remote_elevator_info(remote_update);
        assert_eq! {global_elevator_info.get_local_elevator_info(), local_elev_info}
    }

    #[test]
    fn it_correctly_merges_remote_order_list() {
        let mut order_list = OrderList::new(5);
        order_list.set_pending(CallButton { floor: 4, call: 0 });
        order_list.set_pending(CallButton { floor: 3, call: 2 });
        order_list.set_active(CallButton { floor: 0, call: 2 });

        // Check: Is merging the lists the same as adding orders?
        let mut order_list_to_compare = order_list.clone();
        order_list_to_compare.set_active(CallButton { floor: 1, call: 0 });
        order_list_to_compare.set_active(CallButton { floor: 0, call: 2 });

        let mut order_list_update = OrderList::new(5);
        order_list_update.set_active(CallButton { floor: 1, call: 0 });
        order_list_update.set_active(CallButton { floor: 0, call: 2 });

        order_list = merge_remote_orders(&order_list, &order_list_update);
        assert_eq!(order_list, order_list_to_compare);
    }

    #[test]
    fn it_correctly_services_hall_orders() {
        let mut local_order_list = OrderList::new(5);
        local_order_list.set_active(CallButton { floor: 4, call: 0 });
        local_order_list.set_active(CallButton { floor: 0, call: 2 });
        local_order_list.set_active(CallButton { floor: 2, call: 0 });

        let mut correct_order_list = OrderList::new(5);
        correct_order_list.set_active(CallButton { floor: 4, call: 0 });
        correct_order_list.set_active(CallButton { floor: 0, call: 2 });
        correct_order_list.set_active(CallButton { floor: 2, call: 0 });
        correct_order_list.set_active(CallButton { floor: 4, call: 0 });
        correct_order_list.set_active(CallButton { floor: 2, call: 1 });
        correct_order_list.set_active(CallButton { floor: 0, call: 1 });

        let mut timed_out_order_list = OrderList::new(5);
        timed_out_order_list.set_pending(CallButton { floor: 4, call: 0 });
        timed_out_order_list.set_pending(CallButton { floor: 2, call: 1 });
        timed_out_order_list.set_pending(CallButton { floor: 3, call: 2 });
        timed_out_order_list.set_active(CallButton { floor: 0, call: 1 });

        local_order_list = service_hall_orders(&local_order_list, &timed_out_order_list);

        assert_eq!(local_order_list, correct_order_list);
    }
}
