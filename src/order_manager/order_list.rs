//! Fast and easy order management to be used for both local and global queues!

use crate::elevio::poll as elevio;
use std::vec::Vec;


/// Utility struct for managing local or global orders
/// 
/// # Example
/// ```rust
/// let num_floors = 4;
/// let global_orders = OrderList::new(num_floors);
/// let call_button_corresponding_to_order = CallButton{floor: 2, call: 1}
/// global_orders.add_order(call_button_corresponding_to_order)
/// ```
/// 
#[derive(PartialEq)]
pub struct OrderList {
    n_floors: usize,
    pub up_queue: Vec<bool>,
    pub down_queue: Vec<bool>,
    pub inside_queue: Vec <bool>,
}

impl OrderList {
    ///creates a new instance of an OrderList
    /// 
    /// * `num_floors` - The number of floors that are to take orders
    pub fn new(num_floors: u8) -> OrderList {
        let temp_n_floors = usize::from(num_floors);
        OrderList{
            n_floors: temp_n_floors,
            up_queue: vec![false; temp_n_floors],
            down_queue: vec![false; temp_n_floors],
            inside_queue: vec![false; temp_n_floors]
        }
    }

    /// Clears all orders on the specified floor
    /// 
    /// `floor` - Floor to clear
    pub fn clear_orders_on_floor(&mut self, floor: u8) {
        self.up_queue[usize::from(floor)] = false;
        self.inside_queue[usize::from(floor)] = false;
        self.down_queue[usize::from(floor)] = false;
    }
    /// Clears all orders on all the floors
    pub fn clear_all_orders(&mut self) {
        for i in 0..=self.n_floors-1 {
            self.up_queue[i] = false;
            self.down_queue[i] = false;
            self.inside_queue[i] = false;
        }
    }
    /// Removes a single order in the specified direction
    /// 
    /// `button` - The button (containing `floor` and `call`) that corresponds to the order to be removed
    pub fn remove_order(&mut self, button: elevio::CallButton) {
        self.modify_order(button, false);
    }

    /// Removes a single order in the specified direction
    /// 
    /// `button` - The button (containing `floor` and `call`) that corresponds to the order to be added
    /// 
    /// #
    /// 
    /// 
    pub fn add_order(&mut self, button: elevio::CallButton){
        self.modify_order(button, true);
    }

    fn modify_order(&mut self, button: elevio::CallButton, add_or_remove: bool) {
        match button.call {
            0 => self.up_queue[usize::from(button.floor)] = add_or_remove,
            1 => self.down_queue[usize::from(button.floor)] = add_or_remove,
            2 => self.inside_queue[usize::from(button.floor)] = add_or_remove,
            _ => {}
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
        order_list.add_order(CallButton{floor: 3, call: 0});
        order_list.add_order(CallButton{floor: 1, call: 2});
        let mut reference_order_list = OrderList::new(5);
        reference_order_list.up_queue[3] = true;
        reference_order_list.inside_queue[1] = true;
        assert!((order_list == reference_order_list));
    }

    #[test]
    fn it_correctly_clears_single_order() {
        let mut order_list = OrderList::new(5);
        order_list.add_order(CallButton{floor: 3, call: 0});
        order_list.add_order(CallButton{floor: 1, call: 2});
        order_list.remove_order(CallButton{floor: 3, call: 0});
        order_list.remove_order(CallButton{floor: 1, call: 2});
        let mut reference_order_list = OrderList::new(5);
        assert!((order_list == reference_order_list));
    }

    #[test]
    fn it_correctly_clears_floor_order() {
        let mut order_list = OrderList::new(5);
        order_list.add_order(CallButton{floor: 2, call: 0});
        order_list.add_order(CallButton{floor: 2, call: 1});
        order_list.add_order(CallButton{floor: 2, call: 2});
        order_list.clear_orders_on_floor(2);
        let mut reference_order_list = OrderList::new(5);
        assert!((order_list == reference_order_list));
    }

    #[test]
    fn it_correctly_clears_all_orders() {
        let mut order_list = OrderList::new(5);
        order_list.add_order(CallButton{floor: 4, call: 0});
        order_list.add_order(CallButton{floor: 2, call: 0});
        order_list.add_order(CallButton{floor: 3, call: 2});
        order_list.clear_all_orders();
        let mut reference_order_list = OrderList::new(5);
        assert!((order_list == reference_order_list));
    }
}