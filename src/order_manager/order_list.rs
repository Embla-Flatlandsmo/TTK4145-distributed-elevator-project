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
        for i in 0..=self.n_floors {
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