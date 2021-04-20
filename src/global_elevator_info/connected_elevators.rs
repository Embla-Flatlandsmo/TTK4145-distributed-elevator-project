use crossbeam_channel as cbc;
use std::time;
use std::thread::*;

use crate::local_elevator::elevio::poll::{CallButton, CAB};
use crate::local_elevator::elevio::elev::HardwareCommand;
use crate::local_elevator::fsm::elevatorfsm::{ElevatorInfo, State};
use crate::local_elevator::fsm::order_list::{OrderList, OrderType};
use crate::util::constants::{MAX_NUM_ELEV, ELEV_NUM_FLOORS};
use crate::util::constants::ID as LOCAL_ID;


#[derive(Clone, Debug)]
pub struct ConnectedElevatorInfo {
    connected_elevators: Vec<Option<ElevatorInfo>>,
}

impl ConnectedElevatorInfo {
    fn new(local_elev: ElevatorInfo, max_number_of_elevators: usize) -> ConnectedElevatorInfo {

        let mut connected_elevs: Vec<Option<ElevatorInfo>> = Vec::new();
        connected_elevs.resize_with(max_number_of_elevators, || None);
        connected_elevs[LOCAL_ID] = Some(local_elev.clone());
        ConnectedElevatorInfo {
            connected_elevators: connected_elevs,
        }
    }

    /// Updates global info with the newest info received from remote elevators.
    fn update_remote_elevator_info(&mut self, remote_update: Vec<ElevatorInfo>) -> Vec<CallButton> {

        let mut new_connected_elev_info: Vec<Option<ElevatorInfo>> = Vec::new();
        new_connected_elev_info.resize_with(MAX_NUM_ELEV, || None);
        
        let mut fix_len_remote_elev_update = new_connected_elev_info.clone();
        let prev_connected_elev_info = self.get_connected_elevators();
        new_connected_elev_info[LOCAL_ID] = self.get_local_elevator_info();
        let mut lost_orders: Vec<CallButton> = Vec::new();

        for elev in remote_update.iter() {
            fix_len_remote_elev_update[elev.get_id()] = Some(elev.clone());
        }
        
        for i in 0..MAX_NUM_ELEV {
            if i != LOCAL_ID {
                let mut remote_info: ElevatorInfo;
                let existing_info: ElevatorInfo;
                match prev_connected_elev_info[i].as_ref() {
                    None => {
                        new_connected_elev_info[i] = fix_len_remote_elev_update[i].clone();
                    },
                    Some(vl) => {
                        existing_info = vl.clone();
                        match fix_len_remote_elev_update[i].as_ref() {
                            None => {
                                lost_orders.append(&mut assign_orders_locally(existing_info.responsible_orders));
                                new_connected_elev_info[i] = None;
                                
                            },
                            Some(vr) => {
                                remote_info = vr.clone();
                                if (existing_info.state != State::MovTimedOut && remote_info.state == State::MovTimedOut) 
                                || (existing_info.state != State::ObstrTimedOut && remote_info.state == State::ObstrTimedOut) {
                                    lost_orders.append(&mut assign_orders_locally(existing_info.responsible_orders.clone()));
                                }
                                remote_info.responsible_orders = merge_remote_orders(existing_info.clone().responsible_orders.clone(), remote_info.clone().responsible_orders.clone());
                                new_connected_elev_info[i] = Some(remote_info);
                            }
                        }
                    }
                }
            }
        }
        println!("{:#?}", new_connected_elev_info.clone());
        self.connected_elevators = new_connected_elev_info;
        return lost_orders;
    }

    fn update_local_elevator_info(&mut self, local_update: ElevatorInfo) {
        self.connected_elevators[LOCAL_ID] = Some(local_update);
    }

    fn set_to_pending(&mut self, should_set: bool, id: usize, button: CallButton) {
        let mut elev_info: ElevatorInfo;
        match self.connected_elevators[id].as_ref() {
            Some(v) => elev_info = v.clone(),
            None => return,
        }
        elev_info.responsible_orders.set_pending(should_set, button);
        self.connected_elevators[id] = Some(elev_info);
    }

    pub fn is_pending(&self, id: usize, button: CallButton) -> bool {
        let elev_info: ElevatorInfo;
        match self.connected_elevators[id].as_ref() {
            Some(v) => elev_info = v.clone(),
            None => return false,
        }
        return elev_info.responsible_orders.is_pending(button);
    }

    pub fn is_active(&self, id: usize, button: CallButton) -> bool {
        let elev_info: ElevatorInfo;
        match self.connected_elevators[id].as_ref() {
            Some(v) => elev_info = v.clone(),
            None => return false,
        }
        return elev_info.responsible_orders.is_active(button);
    }

    pub fn get_orders_for_lights(&self) -> OrderList {
        let mut order_lights: OrderList = OrderList::new(ELEV_NUM_FLOORS);
        for entry in self.connected_elevators.iter().cloned() {
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

    pub fn get_connected_elevators(&self) -> Vec<Option<ElevatorInfo>> {
        return self.clone().connected_elevators.clone();
    }

    pub fn get_local_elevator_info(&self) -> Option<ElevatorInfo> {
        return self.clone().connected_elevators[LOCAL_ID]
            .clone();
    }
}

pub fn connected_elevator_info(
    local_update: cbc::Receiver<ElevatorInfo>, 
    remote_update: cbc::Receiver<Vec<ElevatorInfo>>,
    set_pending: cbc::Receiver<(bool, usize, CallButton)>, 
    global_info_update: cbc::Sender<ConnectedElevatorInfo>,
    assign_orders_locally_tx: cbc::Sender<CallButton>) {

    let mut global_info: ConnectedElevatorInfo;
    
    let initial_info = local_update.recv().unwrap();
    global_info = ConnectedElevatorInfo::new(initial_info, MAX_NUM_ELEV);

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

pub fn set_order_lights(
    global_info_rx: cbc::Receiver<ConnectedElevatorInfo>, 
    set_lights_tx: cbc::Sender<HardwareCommand>) {

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


fn assign_orders_locally(orders_to_assign: OrderList) -> Vec<CallButton> {
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