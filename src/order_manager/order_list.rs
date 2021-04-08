//! Fast and easy order management to be used for both local and global queues!
#![allow(dead_code)]
use crate::elevio::poll as elevio;
use serde;
use std::vec::Vec;

#[derive(PartialEq, Copy, Clone, Debug, serde::Serialize, serde::Deserialize, Hash)]
pub enum OrderType {
    Active,
    Pending,
    None,
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
        },
        _ => remote_order,
    }
}

/// Utility struct for managing local or global orders
///
/// # Example
/// ```rust
/// use elevator::elevio::poll::CallButton;
/// use elevator::order_manager::order_list::OrderList;
/// let num_floors = 4;
/// let mut elevator_orders = OrderList::new(num_floors);
/// let call_button_corresponding_to_order = CallButton{floor: 2, call: 1};
/// elevator_orders.add_order(call_button_corresponding_to_order)
/// ```
///
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize, Hash)]
pub struct OrderList {
    n_floors: usize,
    pub up_queue: Vec<OrderType>,
    pub down_queue: Vec<OrderType>,
    pub inside_queue: Vec<OrderType>,
}

impl OrderList {
    ///creates a new instance of an OrderList
    ///
    /// * `num_floors` - The number of floors that are to take orders
    pub fn new(num_floors: u8) -> OrderList {
        let temp_n_floors = usize::from(num_floors);
        OrderList {
            n_floors: temp_n_floors,
            up_queue: vec![OrderType::None; temp_n_floors],
            down_queue: vec![OrderType::None; temp_n_floors],
            inside_queue: vec![OrderType::None; temp_n_floors],
        }
    }

    /// Clears all orders on the specified floor
    ///
    /// `floor` - Floor to clear
    pub fn clear_orders_on_floor(&mut self, floor: u8) {
        self.up_queue[usize::from(floor)] = OrderType::None;
        self.inside_queue[usize::from(floor)] = OrderType::None;
        self.down_queue[usize::from(floor)] = OrderType::None;
    }
    /// Clears all orders on all the floors
    pub fn clear_all_orders(&mut self) {
        for i in 0..=self.n_floors - 1 {
            self.up_queue[i] = OrderType::None;
            self.down_queue[i] = OrderType::None;
            self.inside_queue[i] = OrderType::None;
        }
    }
    /// Removes a single order in the specified direction
    ///
    /// `button` - The button (containing `floor` and `call`) that corresponds to the order to be removed
    pub fn remove_order(&mut self, button: elevio::CallButton) {
        self.modify_order(button, OrderType::None);
    }

    /// Removes a single order in the specified direction
    ///
    /// `button` - The button (containing `floor` and `call`) that corresponds to the order to be added
    ///
    pub fn add_order(&mut self, button: elevio::CallButton) {
        self.modify_order(button, OrderType::Active);
    }
    /// Sets the order corresponding to button to pending. 
    /// 
    /// **Note**: If the order is already active, the order will not be set to pending but remain active.
    pub fn set_pending(&mut self, button: elevio::CallButton) {
        if self.get_order_status(button) != OrderType::Active {
            self.modify_order(button, OrderType::Pending);
        }
    }
    /// Says if the order is pending
    ///
    /// * `button` - The button (containing `floor` and `call`) that corresponds to the order to be checked
    pub fn is_pending(&self, button: elevio::CallButton) -> bool {
        return self.get_order_status(button) == OrderType::Pending;
    }

    /// Updates the order list with the values of a remote order
    pub fn merge_remote_orders(&mut self, remote_orders: OrderList) {
        if self.n_floors != remote_orders.n_floors {
            panic!("Tried to merge elevator orders of different lengths :(")
        }
    
        for i in 0..=self.n_floors - 1 {
            self.up_queue[i] = merge_remote_order(self.up_queue[i], remote_orders.up_queue[i]);
            self.down_queue[i] = merge_remote_order(self.down_queue[i], remote_orders.down_queue[i]);
            self.inside_queue[i] = merge_remote_order(self.inside_queue[i], remote_orders.inside_queue[i]);
        }
    }

    /// Sets both Active and Pending hall orders of `orders` to Active in its own list.
    ///
    /// * `orders` - OrderList to merge
    pub fn service_hall_orders(&mut self, orders: OrderList) {
        if self.n_floors != orders.n_floors {
            panic!("Tried to merge elevator orders of different lengths :(")
        }

        for i in 0..=self.n_floors - 1 {
            if self.up_queue[i] == OrderType::Active
                || orders.up_queue[i] == OrderType::Active
                || orders.up_queue[i] == OrderType::Pending
            {
                self.up_queue[i] = OrderType::Active;
            }
            if self.down_queue[i] == OrderType::Active
                || orders.down_queue[i] == OrderType::Active
                || orders.down_queue[i] == OrderType::Pending
            {
                self.down_queue[i] = OrderType::Active;
            }
        }
    }

    fn get_order_status(&self, button: elevio::CallButton) -> OrderType {
        match button.call {
            0 => self.up_queue[usize::from(button.floor)],
            1 => self.down_queue[usize::from(button.floor)],
            2 => self.inside_queue[usize::from(button.floor)],
            _ => unreachable!(),
        }
    }

    fn modify_order(&mut self, button: elevio::CallButton, order_type: OrderType) {
        match button.call {
            0 => self.up_queue[usize::from(button.floor)] = order_type,
            1 => self.down_queue[usize::from(button.floor)] = order_type,
            2 => self.inside_queue[usize::from(button.floor)] = order_type,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::elevio::poll::CallButton;
    #[test]
    fn it_correctly_adds_orders() {
        let mut order_list = OrderList::new(5);
        order_list.add_order(CallButton { floor: 3, call: 0 });
        order_list.add_order(CallButton { floor: 1, call: 2 });
        let mut reference_order_list = OrderList::new(5);
        reference_order_list.up_queue[3] = OrderType::Active;
        reference_order_list.inside_queue[1] = OrderType::Active;
        assert!((order_list == reference_order_list));
    }

    #[test]
    fn it_correctly_clears_single_order() {
        let mut order_list = OrderList::new(5);
        order_list.add_order(CallButton { floor: 3, call: 0 });
        order_list.add_order(CallButton { floor: 1, call: 2 });
        order_list.remove_order(CallButton { floor: 3, call: 0 });
        order_list.remove_order(CallButton { floor: 1, call: 2 });
        let reference_order_list = OrderList::new(5);
        assert!((order_list == reference_order_list));
    }

    #[test]
    fn it_correctly_clears_floor_order() {
        let mut order_list = OrderList::new(5);
        order_list.add_order(CallButton { floor: 2, call: 0 });
        order_list.add_order(CallButton { floor: 2, call: 1 });
        order_list.add_order(CallButton { floor: 2, call: 2 });
        order_list.clear_orders_on_floor(2);
        let reference_order_list = OrderList::new(5);
        assert!((order_list == reference_order_list));
    }

    #[test]
    fn it_correctly_clears_all_orders() {
        let mut order_list = OrderList::new(5);
        order_list.add_order(CallButton { floor: 4, call: 0 });
        order_list.add_order(CallButton { floor: 2, call: 0 });
        order_list.add_order(CallButton { floor: 3, call: 2 });
        order_list.clear_all_orders();
        let reference_order_list = OrderList::new(5);
        assert!((order_list == reference_order_list));
    }
    #[test]
    fn it_correctly_detects_pending_order() {
        let mut order_list = OrderList::new(5);
        order_list.set_pending(CallButton { floor: 4, call: 0 });
        assert!(order_list.is_pending(CallButton { floor: 4, call: 0 }));
    }

    #[test]
    fn it_correctly_merges_remote_order_list() {
        let mut order_list = OrderList::new(5);
        order_list.set_pending(CallButton { floor: 4, call: 0 });
        order_list.set_pending(CallButton { floor: 3, call: 2 });
        order_list.add_order(CallButton { floor: 0, call: 2 });

        // Check: Is merging the lists the same as adding orders?
        let mut order_list_to_compare = order_list.clone();
        order_list_to_compare.add_order(CallButton{floor: 1, call: 0});
        order_list_to_compare.add_order(CallButton{floor: 0, call: 2});

        let mut order_list_update = OrderList::new(5);
        order_list_update.add_order(CallButton{floor: 1, call: 0});
        order_list_update.add_order(CallButton{floor: 0, call: 2});

        order_list.merge_remote_orders(order_list_update);
        assert_eq!(order_list, order_list_to_compare);
    }

    #[test]
    fn it_correctly_services_hall_orders() {
        let mut local_order_list = OrderList::new(5);
        local_order_list.add_order(CallButton { floor: 4, call: 0 });
        local_order_list.add_order(CallButton { floor: 0, call: 2 });
        local_order_list.add_order(CallButton { floor: 2, call: 0 });

        let mut correct_order_list = OrderList::new(5);
        correct_order_list.add_order(CallButton { floor: 4, call: 0 });
        correct_order_list.add_order(CallButton { floor: 0, call: 2 });
        correct_order_list.add_order(CallButton { floor: 2, call: 0 });
        correct_order_list.add_order(CallButton { floor: 4, call: 0 });
        correct_order_list.add_order(CallButton { floor: 2, call: 1 });
        correct_order_list.add_order(CallButton { floor: 0, call: 1 });

        let mut timed_out_order_list = OrderList::new(5);
        timed_out_order_list.set_pending(CallButton { floor: 4, call: 0 });
        timed_out_order_list.set_pending(CallButton { floor: 2, call: 1 });
        timed_out_order_list.set_pending(CallButton { floor: 3, call: 2 });
        timed_out_order_list.add_order(CallButton { floor: 0, call: 1 });

        local_order_list.service_hall_orders(timed_out_order_list);

        assert_eq!(local_order_list, correct_order_list);
    }
}
