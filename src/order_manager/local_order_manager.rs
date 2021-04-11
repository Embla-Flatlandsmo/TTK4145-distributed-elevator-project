use super::order_list;
use super::order_list::OrderType;
use crate::elevio::elev::{DIRN_DOWN, DIRN_STOP, DIRN_UP};
use crate::fsm::elevatorfsm;

pub fn choose_direction(fsm: &mut elevatorfsm::Elevator) -> u8 {
    let dirn = fsm.get_dirn();
    let order_list = fsm.get_orders();
    let floor = usize::from(fsm.get_floor());
    let orders_above: bool = order_above(&order_list, floor);
    let orders_below: bool = order_below(&order_list, floor);
    match dirn {
        DIRN_UP => {
            if orders_above {
                return DIRN_UP;
            } else if orders_below {
                return DIRN_DOWN;
            } else {
                return DIRN_STOP;
            }
        }
        DIRN_DOWN => {
            if orders_below {
                return DIRN_DOWN;
            } else if orders_above {
                return DIRN_UP;
            } else {
                return DIRN_STOP;
            }
        }
        DIRN_STOP => {
            if orders_below {return DIRN_DOWN;}
            else if orders_above {return DIRN_UP;}
            else {return DIRN_STOP;}
        },
        _ => DIRN_STOP,
    }
}

pub fn should_stop(fsm: &mut elevatorfsm::Elevator) -> bool {
    let dirn = fsm.get_dirn();
    let order_list = fsm.get_orders();
    let floor = usize::from(fsm.get_floor());
    match dirn {
        DIRN_DOWN => {
            return {
                order_list.down_queue[floor] == OrderType::Active ||
                order_list.inside_queue[floor] == OrderType::Active ||
                !order_below(&order_list, floor)
            }
        }
        DIRN_UP => {
            return {
                order_list.up_queue[floor] == OrderType::Active ||
                order_list.inside_queue[floor] == OrderType::Active ||
                !order_above(&order_list, floor)
            }
        }
        _ => true,
    }
}

fn order_below(order_list: &order_list::OrderList, floor: usize) -> bool {
    let up_queue = &(*order_list).up_queue;
    let down_queue = &(*order_list).down_queue;
    let inside_queue = &(*order_list).inside_queue;

    return single_queue_order_below(up_queue, floor)
        || single_queue_order_below(down_queue, floor)
        || single_queue_order_below(inside_queue, floor);
}

fn order_above(order_list: &order_list::OrderList, floor: usize) -> bool {
    let up_queue = &(*order_list).up_queue;
    let down_queue = &(*order_list).down_queue;
    let inside_queue = &(*order_list).inside_queue;

    return single_queue_order_above(up_queue, floor)
        || single_queue_order_above(down_queue, floor)
        || single_queue_order_above(inside_queue, floor);
}

fn single_queue_order_below(queue: &[OrderType], floor: usize) -> bool {
    for &order in queue.iter().take(floor) {
        if order == OrderType::Active{
            return true;
        }
    }
    return false;
}

fn single_queue_order_above(queue: &[OrderType], floor: usize) -> bool {
    for &order in queue.iter().skip(floor + 1) {
        if order == OrderType::Active {
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
        order_list.set_active(CallButton { floor: 3, call: 0 });
        order_list.set_active(CallButton { floor: 1, call: 2 });
        assert!(order_above(&order_list, 1));
    }

    #[test]
    fn it_finds_order_in_the_top() {
        let mut order_list = order_list::OrderList::new(5);
        order_list.set_active(CallButton { floor: 4, call: 2 });
        assert!(order_above(&order_list, 1));
    }

    #[test]
    fn it_finds_order_in_the_bottom() {
        let mut order_list = order_list::OrderList::new(5);
        order_list.set_active(CallButton { floor: 0, call: 2 });
        assert!(order_below(&order_list, 1));
    }

    #[test]
    fn it_finds_order_below() {
        let mut order_list = order_list::OrderList::new(5);
        order_list.set_active(CallButton { floor: 3, call: 0 });
        order_list.set_active(CallButton { floor: 0, call: 2 });
        assert!(order_below(&order_list, 1));
    }

    #[test]
    fn it_finds_no_order_below() {
        let mut order_list = order_list::OrderList::new(5);
        order_list.set_active(CallButton { floor: 3, call: 0 });
        order_list.set_active(CallButton { floor: 4, call: 2 });
        assert!(!order_below(&order_list, 2));
    }

    #[test]
    fn it_finds_no_order_above() {
        let mut order_list = order_list::OrderList::new(5);
        order_list.set_active(CallButton { floor: 1, call: 0 });
        order_list.set_active(CallButton { floor: 0, call: 1 });
        assert!(!order_above(&order_list, 3));
    }
}
