//! Fast and easy order management to be used for both local and global queues!
#![allow(dead_code)]
use crate::local_elevator::elevio::poll as elevio;
use serde;
use std::vec::Vec;

#[derive(PartialEq, Copy, Clone, Debug, serde::Serialize, serde::Deserialize, Hash)]
pub enum OrderType {
    Active,
    Pending,
    None,
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
/// elevator_orders.set_active(call_button_corresponding_to_order)
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
    pub fn set_active(&mut self, button: elevio::CallButton) {
        self.modify_order(button, OrderType::Active);
    }
    /// Sets the order corresponding to button to pending. 
    /// 
    /// **Note**: If the order is already active, the order will not be set to pending but remain active.
    pub fn set_pending(&mut self, should_set: bool, button: elevio::CallButton) {
        if self.get_order_status(button) != OrderType::Active {
            if should_set {
                self.modify_order(button, OrderType::Pending);
            } else {
                self.modify_order(button, OrderType::None);
            }

        }
    }
    /// Says if the order is pending
    ///
    /// * `button` - The button (containing `floor` and `call`) that corresponds to the order to be checked
    pub fn is_pending(&self, button: elevio::CallButton) -> bool {
        return self.get_order_status(button) == OrderType::Pending;
    }

    pub fn is_active(&self, button: elevio::CallButton) -> bool {
        return self.get_order_status(button) == OrderType::Active;
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
    use crate::local_elevator::elevio::poll::CallButton;
    #[test]
    fn it_correctly_adds_orders() {
        let mut order_list = OrderList::new(5);
        order_list.set_active(CallButton { floor: 3, call: 0 });
        order_list.set_active(CallButton { floor: 1, call: 2 });
        let mut reference_order_list = OrderList::new(5);
        reference_order_list.up_queue[3] = OrderType::Active;
        reference_order_list.inside_queue[1] = OrderType::Active;
        assert!((order_list == reference_order_list));
    }

    #[test]
    fn it_correctly_clears_single_order() {
        let mut order_list = OrderList::new(5);
        order_list.set_active(CallButton { floor: 3, call: 0 });
        order_list.set_active(CallButton { floor: 1, call: 2 });
        order_list.remove_order(CallButton { floor: 3, call: 0 });
        order_list.remove_order(CallButton { floor: 1, call: 2 });
        let reference_order_list = OrderList::new(5);
        assert!((order_list == reference_order_list));
    }

    #[test]
    fn it_correctly_clears_floor_order() {
        let mut order_list = OrderList::new(5);
        order_list.set_active(CallButton { floor: 2, call: 0 });
        order_list.set_active(CallButton { floor: 2, call: 1 });
        order_list.set_active(CallButton { floor: 2, call: 2 });
        order_list.clear_orders_on_floor(2);
        let reference_order_list = OrderList::new(5);
        assert!((order_list == reference_order_list));
    }

    #[test]
    fn it_correctly_clears_all_orders() {
        let mut order_list = OrderList::new(5);
        order_list.set_active(CallButton { floor: 4, call: 0 });
        order_list.set_active(CallButton { floor: 2, call: 0 });
        order_list.set_active(CallButton { floor: 3, call: 2 });
        order_list.clear_all_orders();
        let reference_order_list = OrderList::new(5);
        assert!((order_list == reference_order_list));
    }
    #[test]
    fn it_correctly_detects_pending_order() {
        let mut order_list = OrderList::new(5);
        order_list.set_pending(true, CallButton { floor: 4, call: 0 });
        assert!(order_list.is_pending(CallButton { floor: 4, call: 0 }));
    }
}
