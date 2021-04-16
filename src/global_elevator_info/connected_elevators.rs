use crate::elevio::poll::{CallButton, CAB};
use crate::elevio::elev::HardwareCommand;
use crate::fsm::elevatorfsm::ElevatorInfo;
use crate::order_assigner::cost_function;
use crate::order_manager::order_list::{OrderList, OrderType};
use crossbeam_channel as cbc;
use std::time;
use crate::util::constants::{MAX_NUM_ELEV, ELEV_NUM_FLOORS, ID};
use std::thread::*;

#[derive(Clone, Debug)]
pub struct GlobalElevatorInfo {
    local_id: usize,
    global_elevators: Vec<Option<ElevatorInfo>>,
}

impl GlobalElevatorInfo {
    pub fn new(local_elev: ElevatorInfo, max_number_of_elevators: usize) -> GlobalElevatorInfo {
        let mut global_elevs: Vec<Option<ElevatorInfo>> = Vec::new();
        global_elevs.resize_with(max_number_of_elevators, || None);
        global_elevs[local_elev.clone().get_id()] = Some(local_elev.clone());
        GlobalElevatorInfo {
            local_id: local_elev.get_id(),
            global_elevators: global_elevs,
        }
    }

    pub fn find_lowest_cost_id(&self, btn: CallButton) -> usize {
        let local_elev_info;
        match self.get_local_elevator_info() {
            Some(v) => local_elev_info = v,
            None => {
                println!("No value found at local elevator ID index");
                return self.local_id;
        }
    }

        let mut lowest_cost: usize =
            cost_function::time_to_idle(local_elev_info, btn);
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
    pub fn update_remote_elevator_info(&mut self, remote_update: Vec<ElevatorInfo>) -> Vec<CallButton> {
        let mut new_global_elev_info: Vec<Option<ElevatorInfo>> = Vec::new();
        new_global_elev_info.resize_with(MAX_NUM_ELEV, || None);
        
        let mut fix_len_remote_elev_update = new_global_elev_info.clone();
        let prev_global_elev_info = self.get_global_elevators();
        new_global_elev_info[self.local_id] = self.get_local_elevator_info();
        let mut lost_orders: Vec<CallButton> = Vec::new();

        for elev in remote_update.iter() {
            fix_len_remote_elev_update[elev.get_id()] = Some(elev.clone());
        }
        
        println!("{:#?}", fix_len_remote_elev_update.clone());
        for i in 0..MAX_NUM_ELEV {
            if i != self.local_id.clone() {
                let mut remote_info: ElevatorInfo;
                let local_info: ElevatorInfo;
                match prev_global_elev_info[i].as_ref() {
                    None => {
                        new_global_elev_info[i] = fix_len_remote_elev_update[i].clone();
                    },
                    Some(vl) => {
                        local_info = vl.clone();
                        match fix_len_remote_elev_update[i].as_ref() {
                            None => {
                                lost_orders.append(&mut assign_orders_locally(local_info.responsible_orders));
                                new_global_elev_info[i] = None;
                                
                            },
                            Some(vr) => {
                                remote_info = vr.clone();
                                remote_info.responsible_orders = merge_remote_orders(local_info.clone().responsible_orders.clone(), remote_info.clone().responsible_orders.clone());
                                new_global_elev_info[i] = Some(remote_info);
                            }
                        }
                    }
                }
            }
        }
        self.global_elevators = new_global_elev_info;
        return lost_orders;
    }

    pub fn update_local_elevator_info(&mut self, local_update: ElevatorInfo) {
        self.global_elevators[self.local_id] = Some(local_update);
    }

    pub fn set_to_pending(&mut self, should_set: bool, id: usize, button: CallButton) {
        let mut elev_info: ElevatorInfo;
        match self.global_elevators[id].as_ref() {
            Some(v) => elev_info = v.clone(),
            None => return,
        }
        elev_info.responsible_orders.set_pending(should_set, button);
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

    pub fn is_active(&self, id: usize, button: CallButton) -> bool {
        let elev_info: ElevatorInfo;
        match self.global_elevators[id].as_ref() {
            Some(v) => elev_info = v.clone(),
            None => return false,
        }
        return elev_info.responsible_orders.is_active(button);
    }

    pub fn get_orders_for_lights(&self) -> OrderList {
        let mut order_lights: OrderList = OrderList::new(ELEV_NUM_FLOORS);
        for entry in self.global_elevators.iter().cloned() {
            match entry {
                Some(elev) => {
                    order_lights =
                        merge_remote_active(order_lights.clone(), elev.clone().responsible_orders.clone())
                }
                None => {}
            }
        }
        match self.get_local_elevator_info() {
            Some(v) => order_lights.inside_queue = v.responsible_orders.clone().inside_queue.clone(),
            None => {}
        }
        return order_lights;
    }

    pub fn get_global_elevators(&self) -> Vec<Option<ElevatorInfo>> {
        return self.clone().global_elevators.clone();
    }

    pub fn get_local_elevator_info(&self) -> Option<ElevatorInfo> {
        return self.clone().global_elevators[self.local_id]
            .clone();
    }
}

pub fn global_elevator_info(
    local_update: cbc::Receiver<ElevatorInfo>, 
    remote_update: cbc::Receiver<Vec<ElevatorInfo>>,
    set_pending: cbc::Receiver<(bool, usize, CallButton)>, 
    global_info_update: cbc::Sender<GlobalElevatorInfo>,
    assign_orders_locally_tx: cbc::Sender<CallButton>) {

    let mut global_info: GlobalElevatorInfo;
    
    let initial_info = local_update.recv().unwrap();
    global_info = GlobalElevatorInfo::new(initial_info, MAX_NUM_ELEV);

    let (reassign_orders_tx, reassign_orders_rx) = cbc::unbounded::<Vec<CallButton>>();

    spawn(move || {
        loop {
            match reassign_orders_rx.recv() {
                Ok(v) => {
                    for order in v.iter().cloned() {
                        assign_orders_locally_tx.send(order).unwrap();
                    }
                },
                Err(e) => {println!("Error: {}", e);}
            }
        }
    });

    let ticker = cbc::tick(time::Duration::from_millis(5000));

    loop {
        cbc::select! {
            recv(local_update) -> a => {
                let local_info = a.unwrap();
                global_info.update_local_elevator_info(local_info);
                global_info_update.send(global_info.clone()).unwrap();
            },
            recv(remote_update) -> a => {
                let remote_info = a.unwrap();
                let lost_orders = global_info.update_remote_elevator_info(remote_info);
                reassign_orders_tx.send(lost_orders).unwrap();
                global_info_update.send(global_info.clone()).unwrap()
            },
            recv(ticker) -> _ => {
                //println!("{:#?}", global_info.clone());
                //global_info_update.send(global_info.clone()).unwrap();
            },
            recv(set_pending) -> a => {
                let (should_set,id, btn) = a.unwrap();
                global_info.set_to_pending(should_set, id, btn);
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


fn assign_orders_locally(orders_to_assign: OrderList) -> Vec<CallButton>{
    let n_floors: usize = orders_to_assign.up_queue.len();
    let mut call_buttons_to_assign: Vec<CallButton> = Vec::new();
    for f in 0..n_floors {
        let mut button;
        for c in 0..=2 {
            if c != CAB {
                button = CallButton{floor: f as u8, call: c as u8};
                if orders_to_assign.is_active(button) || orders_to_assign.is_pending(button) {
                    call_buttons_to_assign.push(button);
                }
            }
        } 
    }
    return call_buttons_to_assign;
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
                return OrderType::Active;
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
    let mut new_order_list: OrderList = OrderList::new(ELEV_NUM_FLOORS);

    if ELEV_NUM_FLOORS as usize != remote_orders.up_queue.len() {
        panic!("Tried to merge elevator orders of different lengths :(");
    }

    for i in 0..ELEV_NUM_FLOORS as usize {
        new_order_list.up_queue[i] =
            merge_remote_order(local_order_info.clone().up_queue[i], remote_orders.clone().up_queue[i]);
        new_order_list.down_queue[i] = merge_remote_order(
            local_order_info.clone().down_queue[i],
            remote_orders.clone().down_queue[i],
        );
        new_order_list.inside_queue[i] = merge_remote_order(
            local_order_info.clone().inside_queue[i],
            remote_orders.clone().inside_queue[i],
        );
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
            responsible_orders: OrderList::new(ELEV_NUM_FLOORS),
        };
        let elev_id_1: ElevatorInfo = ElevatorInfo {
            id: 1,
            state: State::Moving,
            dirn: DIRN_DOWN,
            floor: 1,
            responsible_orders: OrderList::new(ELEV_NUM_FLOORS),
        };
        let mut remote_update: Vec<ElevatorInfo> = Vec::new();
        remote_update.push(elev_id_0);
        remote_update.push(elev_id_1);
        return remote_update;
    }

    #[test]
    fn it_updates_from_remote() {
        let remote_update: Vec<ElevatorInfo> = create_random_remote_update(MAX_NUM_ELEV);
        let local_elev_info: ElevatorInfo = ElevatorInfo {
            id: 2,
            state: State::Moving,
            dirn: DIRN_UP,
            floor: 3,
            responsible_orders: OrderList::new(5),
        };
        let mut global_elevator_info: GlobalElevatorInfo =
            GlobalElevatorInfo::new(local_elev_info, MAX_NUM_ELEV);
        global_elevator_info.update_remote_elevator_info(remote_update.clone());
        let mut is_identical = true;
        for elev in remote_update.iter().cloned() {
            let elev_id: usize = elev.get_id();
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
        global_elevator_info.set_to_pending(true, 0, CallButton { floor: 2, call: 1 });
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
            responsible_orders: OrderList::new(ELEV_NUM_FLOORS),
        };

        let mut global_elevator_info: GlobalElevatorInfo =
            GlobalElevatorInfo::new(local_elev_info, max_num_elev);
        let mut remote_update = create_empty_remote_update(max_num_elev);
        global_elevator_info.update_remote_elevator_info(remote_update.clone());
        global_elevator_info.set_to_pending(true, 0, CallButton { floor: 2, call: 1 });
        remote_update[0]
            .responsible_orders
            .set_active(CallButton { floor: 2, call: 1 });
        println!("{:#?}", remote_update[0].clone());
        global_elevator_info.update_remote_elevator_info(remote_update.clone());
        assert!(global_elevator_info.is_active(0, CallButton { floor: 2, call: 1 }));
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
            global_elevator_info.get_local_elevator_info().unwrap(),
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
        assert_eq!(global_elevator_info.get_local_elevator_info().unwrap(), local_elev_info);
    }

    #[test]
    fn it_correctly_merges_remote_order_list() {
        let mut order_list = OrderList::new(ELEV_NUM_FLOORS);
        order_list.set_pending(true, CallButton { floor: 3, call: 0 });
        order_list.set_pending(true, CallButton { floor: 1, call: 2 });
        order_list.set_active(CallButton { floor: 0, call: 2 });

        // Check: Is merging the lists the same as adding orders?
        let mut order_list_to_compare = order_list.clone();
        order_list_to_compare.set_active(CallButton { floor: 1, call: 0 });
        order_list_to_compare.set_active(CallButton { floor: 0, call: 2 });

        let mut order_list_update = OrderList::new(ELEV_NUM_FLOORS);
        order_list_update.set_active(CallButton { floor: 1, call: 0 });
        order_list_update.set_active(CallButton { floor: 0, call: 2 });

        order_list = merge_remote_orders(order_list.clone(), order_list_update.clone());
        assert_eq!(order_list, order_list_to_compare);
    }
}
