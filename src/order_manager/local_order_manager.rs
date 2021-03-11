use super::order_list;
use crate::fsm::elevatorfsm as elevatorfsm;
use crate::elevio::elev::{DIRN_DOWN, DIRN_STOP, DIRN_UP};

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
#[derive(Clone, Debug)]
pub enum OrderDirection{
    Above,
    Below,
    None
}
/*
pub fn at_ordered_floor(fsm: &elevatorfsm::Elevator, order_list: &order_list::OrderList) -> bool {
    let next_floor = -1;

    switch
}

*/

pub fn order_above_or_below(fsm: &elevatorfsm::Elevator, order_list: &order_list::OrderList) -> OrderDirection {
    let up_queue = (*order_list).up_queue.clone();
    let down_queue = (*order_list).down_queue.clone();
    if queue_is_empty(&up_queue) && queue_is_empty(&down_queue) {
        return OrderDirection::None;
    }

    let tmp_queue: Vec<bool>;
    let driving_direction = (*fsm).get_dirn();
    let floor = usize::from((*fsm).get_floor());
    match driving_direction {
        DIRN_UP => {
            if !queue_is_empty_above(&up_queue, floor) {
                return OrderDirection::Above;
            } else if (!queue_is_empty_above(&down_queue, floor)) {
                return OrderDirection::Above;
            } return OrderDirection::Below;
        },
        DIRN_DOWN => {
            if !queue_is_empty_below(&down_queue, floor) {
                return OrderDirection::Below;
            } else if !queue_is_empty(&up_queue, floor) {
                return OrderDirection::Below;
            }
            return OrderDirection::Above;
        },
        _ => return OrderDirection::Below
        /*
        /*Dunno what this does, maybe it's for having pressed stop button?*/
        DIRN_STOP => {
            if queue_is_empty(&down_queue) {
                tmp_queue = up_queue;
            } else {
                tmp_queue = down_queue;
            }
            for order in tmp_queue.iter().rev().take(floor) {
                if ()
            }
        }
        */
    }
}

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

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn it_finds_empty() {
        let queue: Vec<bool> = [false, false, false, false, false].to_vec();
        assert!(queue_is_empty(&queue));
    }
    #[test]
    fn it_finds_non_empty() {
        let queue: Vec<bool> = [true, false, true, true].to_vec();
        assert!(!queue_is_empty(&queue))
    }

    #[test]
    fn it_finds_empty_above() {
        let queue: Vec<bool> = [false, true, false, false, false].to_vec();
        assert!(queue_is_empty_above(&queue, 2));
    }

    #[test]
    fn it_finds_empty_below() {
        let queue: Vec<bool> = [false, false, true, false, false].to_vec();
        assert!(queue_is_empty_below(&queue, 2));
    }

    #[test]
    fn it_finds_non_empty_above() {
        let queue: Vec<bool> = [true, false, false, false, true].to_vec();
        assert!(!queue_is_empty_above(&queue, 3));
    }

    #[test]
    fn it_finds_non_empty_below() {
        let queue: Vec<bool> = [true, false, false, false, false].to_vec();
        assert!(!queue_is_empty_below(&queue, 3));
    }

}