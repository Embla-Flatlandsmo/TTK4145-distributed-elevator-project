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
*/
pub fn order_chooseDirection(fsm: &mut elevatorfsm::Elevator) -> u8 {
    let dirn = fsm.get_dirn();
    let order_list = fsm.get_orders();
    let floor = usize::from(fsm.get_floor());
    let orders_above: bool = order_above(&order_list, floor);
    let orders_below: bool = order_below(&order_list, floor);
    match dirn {
        DIRN_UP => {
            if orders_above {return DIRN_UP;}
            else if orders_below {return DIRN_DOWN;}
            else {return DIRN_STOP;}
        },
        DIRN_DOWN => {
            if orders_below {return DIRN_DOWN;}
            else if orders_above {return DIRN_UP;}
            else {return DIRN_STOP;}
        },
        DIRN_STOP => {
            if orders_below {return DIRN_DOWN;}
            else if orders_above {return DIRN_UP;}
            else {return DIRN_STOP;}
        },
        _ => DIRN_STOP
    }
}

pub fn order_shouldStop(fsm: &mut elevatorfsm::Elevator) -> bool {
    let dirn = fsm.get_dirn();
    let order_list = fsm.get_orders();
    let floor = usize::from(fsm.get_floor());
    match dirn {
        DIRN_DOWN => {
            return {
                order_list.down_queue[floor] || 
                order_list.inside_queue[floor] ||
                !order_below(&order_list,floor)
            }
        },
        DIRN_UP => {
            return {
                order_list.up_queue[floor] ||
                order_list.inside_queue[floor] ||
                !order_above(&order_list,floor)
            }
        },
        _ => {true}
    }
}


fn order_below(order_list: &order_list::OrderList, floor: usize) -> bool {
    let up_queue = &(*order_list).up_queue;
    let down_queue = &(*order_list).down_queue;
    let inside_queue = &(*order_list).inside_queue;

    return single_queue_order_below(up_queue, floor) 
    || single_queue_order_below(down_queue, floor) || single_queue_order_below(inside_queue, floor);
}


fn order_above(order_list: &order_list::OrderList, floor: usize) -> bool {
    let up_queue = &(*order_list).up_queue;
    let down_queue = &(*order_list).down_queue;
    let inside_queue = &(*order_list).inside_queue;

    return single_queue_order_above(up_queue, floor) 
    || single_queue_order_above(down_queue, floor) || single_queue_order_above(inside_queue, floor);
}

fn single_queue_order_below(queue: &[bool], floor: usize) -> bool {
    for &order in queue.iter().take(floor+1) {
        if order {
            return true;
        }
    }
    return false;
}

fn single_queue_order_above(queue: &[bool], floor: usize) -> bool {
    for &order in queue.iter().rev().take(queue.len()-floor) {
        if order {
            return true;
        }
    }
    return false;
}

#[cfg(test)]
mod test {
    use super::*; 
    use crate::elevio::poll::CallButton;
    #[test]
    fn it_finds_order_above() {
        let mut order_list = order_list::OrderList::new(5);
        order_list.add_order(CallButton{floor: 3, call: 0});
        order_list.add_order(CallButton{floor: 1, call: 2});
        assert!(order_above(&order_list, 1));
    }
    #[test]
    fn it_finds_order_below() {
        let mut order_list = order_list::OrderList::new(5);
        order_list.add_order(CallButton{ floor: 3, call: 0});
        order_list.add_order(CallButton{floor: 1, call: 2});
        assert!(order_below(&order_list, 1));
    }

    #[test]
    fn it_finds_no_order_below() {
        let mut order_list = order_list::OrderList::new(5);
        order_list.add_order(CallButton{floor: 3, call: 0});
        order_list.add_order(CallButton{floor: 4, call: 2});
        assert!(!order_below(&order_list, 2));
    }

    #[test]
    fn it_finds_no_order_above() {
        let mut order_list = order_list::OrderList::new(5);
        order_list.add_order(CallButton{floor: 1, call: 0});
        order_list.add_order(CallButton{floor: 0, call: 1});
        assert!(!order_above(&order_list, 3));
    }

}