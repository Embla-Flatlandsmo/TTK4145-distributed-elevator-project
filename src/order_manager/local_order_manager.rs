use super::order_list;
use crate::fsm::elevatorfsm as elevatorfsm;

/*
pub struct LocalOrders {
    local_orders: order_list::OrderList
}


impl LocalOrders {
    pub fn new(n_floors: u8) -> LocalOrders {
        LocalOrders {local_orders: order_list::OrderList::new(n_floors)}
    }
}
*/

//pub fn at_ordered_floor(current_floor: usize, )

fn queue_is_empty(queue: &Vec<bool>) -> bool {
    for order in queue.iter() {
        if *order {
            return false;
        }
    }
    return true;
}

fn queue_is_empty_below(queue: &Vec<bool>, floor: usize) -> bool {
    for order in queue.iter().take(floor+1) {
        if *order {
            return false;
        }
    }
    return true;
}

fn queue_is_empty_above(queue: &Vec<bool>, floor: usize) -> bool {
    for order in queue.iter().rev().take(queue.len()-floor) {
        if *order {
            return false;
        }
    }
    return true;
}

fn order_above_or_below(fsm: elevatorfsm::Elevator) {
}
