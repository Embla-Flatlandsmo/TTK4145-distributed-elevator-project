use std::thread::*;
use std::time;
use crossbeam_channel as cbc;

use crate::global_elevator_info::connected_elevators::ConnectedElevatorInfo;
use crate::local_elevator::elevio::poll::{CallButton, CAB};
use crate::local_elevator::fsm::elevatorfsm::ElevatorInfo;

use crate::util::constants as setting;

pub fn hall_order_receiver(assign_orders_locally_tx: cbc::Sender<CallButton>, set_pending_tx: cbc::Sender<(bool, usize, CallButton)>) {
    // The reciever for orders
    let (order_recv_tx, order_recv_rx) = cbc::unbounded::<(usize, CallButton)>();
    spawn(move || {
        crate::network_interface::bcast::rx(setting::ORDER_PORT, order_recv_tx);
    });

    loop {
        let res = order_recv_rx.recv();
        let order = res.unwrap();
        let id = order.0;
        let call_button = order.1;
        if id == setting::ID {
            assign_orders_locally_tx.send(call_button).unwrap();
        } else {
            set_pending_tx.send((true, id, call_button)).unwrap();
        }
    }
}


pub fn cab_order_backup_rx<T: serde::de::DeserializeOwned>(port: u16, assign_cab_orders_locally_tx: cbc::Sender::<CallButton>) {
    let start_time = time::Instant::now();
    let timeout = time::Duration::from_millis(500);

    let (cab_backup_recv_tx, cab_backup_recv_rx) = cbc::unbounded::<ElevatorInfo>();
    spawn(move || {
        crate::network_interface::bcast::rx(setting::CAB_BACKUP_PORT, cab_backup_recv_tx);
    });

    while time::Instant::now().duration_since(start_time)<time::Duration::from_millis(5000){
        let r = cab_backup_recv_rx.recv_timeout(timeout);
         match r {
            Ok(val) => {
                let elev_info = val.clone();
                let id = elev_info.clone().id;

                if id == setting::ID{
                    for f in 0..setting::ELEV_NUM_FLOORS {
                        let btn = CallButton{floor: f, call: CAB};
                        if elev_info.responsible_orders.is_active(btn) {
                            assign_cab_orders_locally_tx.send(btn);
                        }
                    }
                }

            }
            Err(_e) => {},
        }
    }

}
