use crate::network::global_elevator::GlobalElevatorInfo;
use crate::elevio::poll::CallButton;
use crossbeam_channel as cbc;

pub fn order_assigner(global_info_ch: cbc::Receiver<GlobalElevatorInfo>,
    call_button_recv: cbc::Receiver<CallButton>,
    set_pending: cbc::Sender<(usize, CallButton)>, 
    order_send: cbc::Sender<(usize, CallButton)>,
    assign_order_locally: cbc::Sender<CallButton>) {

    let mut global_elevator_info: GlobalElevatorInfo;

    loop {
        cbc::select!{
            recv(global_info_ch) -> a => {
                global_elevator_info = a.unwrap();
            },
            recv(call_button_recv) -> a => {
                let call_button = a.unwrap();
                if call_button.call == 2 {
                    assign_order_locally.send(call_button);
                }
                else {
                    let lowest_cost_id = global_elevator_info.find_lowest_cost_id(call_button);
                    let res = order_send.send((lowest_cost_id, call_button));
                    match res {
                        Ok(res) => {set_pending.send((lowest_cost_id, call_button))},
                        Err(res) => {println!("Couldn't send remote order");}
                    };
                }
            }
        }
    }

    
}