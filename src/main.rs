use std::thread::*;
use std::time;
use std::process;
use std::env;
use std::net;

use crossbeam_channel as cbc;
use serde;


mod network {
    pub mod peers;
    pub mod bcast;
}

mod elevio {
    pub mod elev;
    pub mod poll;
}

mod order_manager {
    pub mod local_order_manager;
    pub mod order_list;
}

mod fsm {
    pub mod elevatorfsm;
}

mod timer{
    pub mod timer;
}

use elevio::elev as e;

use fsm::elevatorfsm::Event as Event;

// Data types to be sent on the network must derive traits for serialization
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct CustomDataType {
    message: String,
    iteration: u64,
}


fn main() -> std::io::Result<()> {
    /* ------------------NETWORK----------------- */
    // Genreate id: either from command line, or a default rust@ip#pid
    let args: Vec<String> = env::args().collect();
    let id = if args.len() > 1 {
        args[1].clone()
    } else {
        let local_ip = net::TcpStream::connect("8.8.8.8:53").unwrap().local_addr().unwrap().ip();
        format!("rust@{}#{}", local_ip, process::id())
    };


    let msg_port = 19735;
    let peer_port = 19738;

    
    // The sender for peer discovery
    let (peer_tx_enable_tx, peer_tx_enable_rx) = cbc::unbounded::<bool>();
    {
        let id = id.clone();
        spawn(move ||{
            network::peers::tx(peer_port, id, peer_tx_enable_rx);
        });
    }
    // (periodically disable/enable the peer broadcast, to provoke new peer / peer loss messages)
    spawn(move ||{
        loop {
            sleep(time::Duration::new(6, 0));
            peer_tx_enable_tx.send(false).unwrap();
            sleep(time::Duration::new(3, 0));
            peer_tx_enable_tx.send(true).unwrap();
        }
    });
    
    // The receiver for peer discovery updates
    let (peer_update_tx, peer_update_rx) = cbc::unbounded::<network::peers::PeerUpdate>();
    spawn(move ||{
        network::peers::rx(peer_port, peer_update_tx);
    });
    
    
    // Periodically produce a custom data message
    let (custom_data_send_tx, custom_data_send_rx) = cbc::unbounded::<CustomDataType>();
    {
        let id = id.clone();
        spawn(move ||{
            let mut cd = CustomDataType{
                message: format!("Hello from node {}", id),
                iteration: 0,
            };
            loop {
                custom_data_send_tx.send(cd.clone()).unwrap();
                cd.iteration += 1;
                sleep(time::Duration::new(1, 0));
            }
        });
    }
    // The sender for our custom data
    spawn(move ||{
        network::bcast::tx(msg_port, custom_data_send_rx);
    });
    // The receiver for our custom data
    let (custom_data_recv_tx, custom_data_recv_rx) = cbc::unbounded::<CustomDataType>();
    spawn(move ||{
        network::bcast::rx(msg_port, custom_data_recv_tx);
    });



    /*----------------SINGLE ELEVATOR---------------------*/
    let elev_num_floors = 4;
    let elevator = e::ElevatorHW::init("localhost:15657", elev_num_floors)?;
    println!("Elevator started:\n{:#?}", elevator);    

    /* We should do something about all these fsms :^) 
    * Here, we initialize the fsm and transitions it into downwards moving (is there a better way to solve this?)
    */

    let (hardware_command_tx, hardware_command_rx) = cbc::unbounded::<elevio::elev::HardwareCommand>();

    let mut fsm = fsm::elevatorfsm::Elevator::new(elev_num_floors,hardware_command_tx);

    /* Spawn a thread that executes elevator commands sent from fsm on server */
    {
        let elevator = elevator.clone();
        spawn(move ||{
            loop {
                let r = hardware_command_rx.recv();
                match r {
                    Ok(c) => elevator.execute_command(c),
                    Err(_e) => {}
                }
            }
        });
    }
    
    let poll_period = time::Duration::from_millis(25);
    
    let (call_button_tx, call_button_rx) = cbc::unbounded::<elevio::poll::CallButton>();
    {
        let elevator = elevator.clone();
        spawn(move ||{
            elevio::poll::call_buttons(elevator, call_button_tx, poll_period) 
        });
    }
    
    let (floor_sensor_tx, floor_sensor_rx) = cbc::unbounded::<u8>();    
    {
        let elevator = elevator.clone();
        spawn(move ||{
            elevio::poll::floor_sensor(elevator, floor_sensor_tx, poll_period)
        });
    }
    
    let (stop_button_tx, stop_button_rx) = cbc::unbounded::<bool>();    
    {
        let elevator = elevator.clone();
        spawn(move ||{
            elevio::poll::stop_button(elevator, stop_button_tx, poll_period)
        });
    }
    
    let (obstruction_tx, obstruction_rx) = cbc::unbounded::<bool>();    
    {
        let elevator = elevator.clone();
        spawn(move ||{
            elevio::poll::obstruction(elevator, obstruction_tx, poll_period)
        });
    }
    
    let (door_timer_tx, door_timer_rx) = cbc::unbounded::<bool>();
    {
        /* Some logic for spawning timer thread
        
        Perhaps this could be made in the timer module?
        
        In any case, it needs to send a message so it can be spotted in the select loop */

    }
    
    
    // main body: receive peer updates and data from the network
    loop {
        cbc::select! {
            recv(peer_update_rx) -> a => {
                let update = a.unwrap();
                println!("{:#?}", update);
            }
            recv(custom_data_recv_rx) -> a => {
                let cd = a.unwrap();
                println!("{:#?}", cd);
            }
        }

        cbc::select! {
            recv(call_button_rx) -> a => {
                let call_button = a.unwrap();
                println!("{:#?}", call_button);
                fsm = fsm.transition(Event::OnNewOrder{btn: call_button});
                elevator.call_button_light(call_button.floor, call_button.call, true);
            },
            recv(floor_sensor_rx) -> a => {
                let floor = a.unwrap();
                fsm = fsm.transition(Event::OnFloorArrival{floor: floor});
                println!("Floor: {:#?}", floor);
                /*
                println!("Floor: {:#?}", floor);
                    if floor == 0 {
                        //e::DIRN_UP
                    } else if floor == elev_num_floors-1 {
                        //e::DIRN_DOWN
                    } else {
                        //dirn
                    };
                //elevator.motor_direction(dirn);
                */
            },
            recv(stop_button_rx) -> a => {
                let stop = a.unwrap();
                /*
                println!("Stop button: {:#?}", stop);
                for f in 0..elev_num_floors {
                    for c in 0..3 {
                        elevator.call_button_light(f, c, false);
                    }
                }
                */
            },
            recv(obstruction_rx) -> a => {
                let obstr = a.unwrap();
                /* Logic for restarting the timer */
                /*
                println!("Obstruction: {:#?}", obstr);
                elevator.motor_direction(if obstr { e::DIRN_STOP } else { dirn });
                */
            },
    	}
    }

}