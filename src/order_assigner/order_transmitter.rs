use crossbeam_channel as cbc;
use std::thread::*;

use crate::util::constants as setting;
use crate::global_elevator_info::connected_elevators::ConnectedElevatorInfo;
use crate::local_elevator::elevio::poll::{CallButton, CAB};

#[path = "./cost_function.rs"]
mod cost_function;


pub fn hall_order_transmitter(
    connected_info_ch: cbc::Receiver<ConnectedElevatorInfo>,
    call_button_recv: cbc::Receiver<CallButton>,
    set_pending: cbc::Sender<(bool, usize, CallButton)>,
    assign_order_locally: cbc::Sender<CallButton>) {

    let mut connected_elevator_info: ConnectedElevatorInfo;
    let (check_if_active_tx, check_if_active_rx) = cbc::unbounded::<(usize, CallButton)>();
    
    let (send_bcast_tx, send_bcast_rx) = cbc::unbounded::<(usize, CallButton)>();

    {
    spawn(move || {
        crate::network_interface::bcast::tx(setting::ORDER_PORT, send_bcast_rx, 5);
    });
    }

    cbc::select!{
        recv(connected_info_ch) -> a => {
            connected_elevator_info = a.unwrap();
        }
    }
    loop {
        cbc::select!{
            recv(connected_info_ch) -> a => {
                connected_elevator_info = a.unwrap();
            },
            recv(call_button_recv) -> a => {

                let call_button = a.unwrap();
                println!("{:#?}", call_button.clone());
                
                if call_button.call == CAB {
                    assign_order_locally.send(call_button).unwrap();
                }
                else {
                    let lowest_cost_id = cost_function::find_lowest_cost_id(connected_elevator_info.clone(), call_button);
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

pub fn cab_order_backup_tx<ElevatorInfo: 'static + Clone + serde::Serialize + std::marker::Send>(
    elev_info_rx: cbc::Receiver::<ElevatorInfo>){

    let (send_bcast_tx, send_bcast_rx) = cbc::unbounded::<ElevatorInfo>();
    spawn(move || {
        crate::network_interface::bcast::tx(setting::CAB_BACKUP_PORT, send_bcast_rx, 10);
    });
    
    loop {
        cbc::select! {
            recv(elev_info_rx) -> new_info => {
                let elev_info = new_info.unwrap();
                send_bcast_tx.send(elev_info.clone()).unwrap();
            }
        }
    }
}