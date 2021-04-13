use crate::network::global_elevator::GlobalElevatorInfo;
use crate::elevio::poll::CallButton;
use crossbeam_channel as cbc;
use std::thread::*;

pub fn order_assigner(global_info_ch: cbc::Receiver<GlobalElevatorInfo>,
    call_button_recv: cbc::Receiver<CallButton>,
    set_pending: cbc::Sender<(usize, CallButton)>, 
    order_send: cbc::Sender<(usize, CallButton)>,
    assign_order_locally: cbc::Sender<CallButton>) {

    let mut global_elevator_info: GlobalElevatorInfo;
    let (check_if_active_tx, check_if_active_rx) = cbc::unbounded::<(usize, CallButton)>();

    cbc::select!{
        recv(global_info_ch) -> a => {
            global_elevator_info = a.unwrap();
        }
    }
    loop {
        cbc::select!{
            recv(global_info_ch) -> a => {
                global_elevator_info = a.unwrap();
            },
            recv(call_button_recv) -> a => {
                /* let call_button = a.unwrap();
                assign_order_locally.send(call_button); */
                let call_button = a.unwrap();
                if call_button.call == 2 {
                    assign_order_locally.send(call_button);
                }
                else {
                    let lowest_cost_id = global_elevator_info.find_lowest_cost_id(call_button);
                    if lowest_cost_id == global_elevator_info.get_local_elevator_info().get_id() {
                        assign_order_locally.send(call_button);
                    }
                    else {
                        let res = order_send.send((lowest_cost_id, call_button));
                        match res {
                            Ok(res) => {
                                set_pending.send((lowest_cost_id, call_button));
                                let check_tx = check_if_active_tx.clone();
                                spawn(move || {
                                    sleep(std::time::Duration::from_secs(1));
                                    check_tx.send((lowest_cost_id, call_button));
                                });
                            },
                            Err(res) => {println!("Couldn't send remote order");}
                        };
                    }
                }
            },
            recv(check_if_active_rx) -> a => {
                let (id, button) = a.unwrap();
                if global_elevator_info.is_pending(id, button) {
                    assign_order_locally.send(button);
                }
            }
        }
    }
}