

use std::thread::*;
use std::time;
use std::process;
use std::env;
use std::net;

use crossbeam_channel as cbc;
use serde;

mod udpnet {
    pub mod peers;
    pub mod bcast;
}

// Data types to be sent on the network must derive traits for serialization
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct CustomDataType {
    message: String,
    iteration: u64,
}


fn main() -> std::io::Result<()> {

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
            udpnet::peers::tx(peer_port, id, peer_tx_enable_rx);
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
    let (peer_update_tx, peer_update_rx) = cbc::unbounded::<udpnet::peers::PeerUpdate>();
    spawn(move ||{
        udpnet::peers::rx(peer_port, peer_update_tx);
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
        udpnet::bcast::tx(msg_port, custom_data_send_rx);
    });
    // The receiver for our custom data
    let (custom_data_recv_tx, custom_data_recv_rx) = cbc::unbounded::<CustomDataType>();
    spawn(move ||{
        udpnet::bcast::rx(msg_port, custom_data_recv_tx);
    });
    
    
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
    }


}




