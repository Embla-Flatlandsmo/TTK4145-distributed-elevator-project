use crate::network::connected_elevators::ConnectedElevatorInfo;
use crate::local_elevator::elevio::poll::{CallButton, CAB};
use crossbeam_channel as cbc;
use crate::util::constants as setting;
use std::thread::*;
/*
pub fn order_transmitter(global_info_ch: cbc::Receiver<ConnectedElevatorInfo>,
    call_button_recv: cbc::Receiver<CallButton>,
    set_pending: cbc::Sender<(bool, usize, CallButton)>,
    assign_order_locally: cbc::Sender<CallButton>) {

    let mut connected_elevator_info: ConnectedElevatorInfo;
    let (check_if_active_tx, check_if_active_rx) = cbc::unbounded::<(usize, CallButton)>();
    
    let (send_bcast_tx, send_bcast_rx) = cbc::unbounded::<(usize, CallButton)>();

    {
    spawn(move || {
        crate::network::bcast::tx(setting::ORDER_PORT, send_bcast_rx, 3);
    });
    }

    cbc::select!{
        recv(global_info_ch) -> a => {
            connected_elevator_info = a.unwrap();
        }
    }
    loop {
        cbc::select!{
            recv(global_info_ch) -> a => {
                connected_elevator_info = a.unwrap();
            },
            recv(call_button_recv) -> a => {

                let call_button = a.unwrap();
                println!("{:#?}", call_button.clone());
                
                if call_button.call == CAB {
                    assign_order_locally.send(call_button).unwrap();
                }
                else {
                    let lowest_cost_id = connected_elevator_info.find_lowest_cost_id(call_button);
                    if lowest_cost_id == setting::ID {
                        assign_order_locally.send(call_button).unwrap();
                    }
                    else {
                        send_bcast_tx.send((lowest_cost_id, call_button)).unwrap();
                        set_pending.send((true, lowest_cost_id, call_button)).unwrap();
                        let check_tx = check_if_active_tx.clone();
                        spawn(move || {
                            sleep(std::time::Duration::from_secs(1));
                            check_tx.send((lowest_cost_id, call_button)).unwrap();
                        });
                    }
                }
            },
            recv(check_if_active_rx) -> a => {
                let (id, button) = a.unwrap();
                if !connected_elevator_info.is_active(id, button) {
                    assign_order_locally.send(button).unwrap();
                    set_pending.send((false, id, button)).unwrap();
                }
            }
        }
    }
}

pub fn order_receiver(assign_orders_locally_tx: cbc::Sender<CallButton>, set_pending_tx: cbc::Sender<(bool, usize, CallButton)>) {
        // The reciever for orders
        let (order_recv_tx, order_recv_rx) = cbc::unbounded::<(usize, CallButton)>();
        spawn(move || {
            crate::network::bcast::rx(setting::ORDER_PORT, order_recv_tx);
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
*/