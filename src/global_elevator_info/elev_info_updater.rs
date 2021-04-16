use crossbeam_channel as cbc;
use serde;
use crate::fsm::elevatorfsm::ElevatorInfo;
use std::time;
use std::thread::*;
use std::collections::HashMap;
use crate::util::constants as setting;
use crate::elevio::poll::{CallButton, CAB};


#[path = "./sock.rs"]
mod sock;

#[derive(Debug, Clone)]
pub struct RemoteElevatorUpdate {
    pub peers:  Vec<ElevatorInfo>,
    pub new:    Option<ElevatorInfo>,
    pub lost:   Vec<ElevatorInfo>,
}

pub fn local_elev_info_tx<ElevatorInfo: 'static + Clone + serde::Serialize + std::marker::Send>(elev_info: cbc::Receiver::<ElevatorInfo>, tx_enable: cbc::Receiver<bool>){

    let (send_bcast_tx, send_bcast_rx) = cbc::unbounded::<ElevatorInfo>();
    {
    spawn(move || {
        crate::network::bcast::tx(setting::PEER_PORT, send_bcast_rx, 3);
    });
    }
    let mut enabled = true;

    let ticker = cbc::tick(time::Duration::from_millis(15));
    let mut local_info: ElevatorInfo;

    cbc::select! {
        recv(elev_info) -> new_info => {
            local_info = new_info.unwrap();
        }
    }
    
    loop {
        cbc::select! {
            recv(tx_enable) -> enable => {
                enabled = enable.unwrap();
            },
            recv(ticker) -> _ => {
                if enabled {
                    send_bcast_tx.send(local_info.clone()).unwrap();
                }
            },
            recv(elev_info) -> new_info => {
                local_info = new_info.unwrap();
            }
        }
    }
}


pub fn remote_elev_info_rx<T: serde::de::DeserializeOwned>(
    port: u16, 
    elev_info_update: cbc::Sender::<Vec<ElevatorInfo>>,
    cab_backup_channel: cbc::Sender::<ElevatorInfo>)
    {
    let timeout = time::Duration::from_millis(500);
    //let s = sock::new_rx(port).unwrap();
    //s.set_read_timeout(Some(timeout)).unwrap();
    
    let (elev_info_recv_tx, elev_info_recv_rx) = cbc::unbounded::<ElevatorInfo>();
    spawn(move || {
        crate::network::bcast::rx(setting::PEER_PORT, elev_info_recv_tx);
    });

    let mut last_seen: HashMap<usize, time::Instant> = HashMap::new();
    let mut active_peers: HashMap<usize, ElevatorInfo> = HashMap::new();
    let mut lost_peers: HashMap<usize, ElevatorInfo> = HashMap::new();

    loop {
        let mut modified = false;
        let mut peers = Vec::new();
        let mut lost = Vec::new();
        let mut reconnected_elevator = false;

        let r = elev_info_recv_rx.recv_timeout(timeout);
        let now = time::Instant::now();
        
        // Find new peers transmitting elevator info
        // TODO: Make this receiver use bcast::rx?
        match r {
            Ok(val) => {
                let elev_info = val.clone();
                let id = elev_info.clone().id;
                if !last_seen.contains_key(&id.clone()) {
                    modified = true;
                    reconnected_elevator = true;
                }
                

                // Send cab calls to reconnecting node
                if reconnected_elevator {
                    if lost_peers.contains_key(&id.clone()){
                        cab_backup_channel.send(lost_peers.get(&id).unwrap().clone());
                    }
                } 

                match active_peers.get(&id).cloned() {
                    Some(existing_info) => {
                        if existing_info != elev_info.clone() {
                            modified = true;
                        }
                    }
                    None => {}
                }

                last_seen.insert(id.clone(), now);
                active_peers.insert(id.clone(), elev_info.clone());
                lost_peers.remove(&id.clone());
            }
            Err(_e) => {},
        }   

        // Finding lost peers
        for (id, when_last_seen) in &last_seen {
            if now.duration_since(*when_last_seen) > timeout {
                lost.push(active_peers.get(id).unwrap().clone()); //what if node is not in elev_info_active?
                lost_peers.insert(id.clone(), active_peers.get(&id).unwrap().clone());
                modified = true;
            }
        }

        // .. and removing them
        for elev in &lost {
            last_seen.remove(&elev.id.clone());
            active_peers.remove(&elev.id.clone());
        }

        // Sending remote elevator update
        if modified {
            peers = active_peers.values().cloned().collect();
            elev_info_update.send(peers.clone()).unwrap();
        }
    }
}


/*
pub fn cab_order_backup_tx<ElevatorInfo: 'static + Clone + serde::Serialize + std::marker::Send>(elev_info_rx: cbc::Receiver::<ElevatorInfo>){

    let (send_bcast_tx, send_bcast_rx) = cbc::unbounded::<ElevatorInfo>();
    spawn(move || {
        crate::network::bcast::tx(setting::CAB_BACKUP_PORT, send_bcast_rx, 10);
    });
    
    loop {
        cbc::select! {
            recv(elev_info_rx) -> new_info => {
                let elev_info = new_info.unwrap();
                send_bcast_tx.send(elev_info.clone()).unwrap(); //cab_order_backup_rx on other nodes get this
            }
        }
    }
}

pub fn cab_order_backup_rx<T: serde::de::DeserializeOwned>(port: u16, assign_cab_orders_locally_tx: cbc::Sender::<CallButton>) {
    let start_time = time::Instant::now();
    let timeout = time::Duration::from_millis(500);

    let (cab_backup_recv_tx, cab_backup_recv_rx) = cbc::unbounded::<ElevatorInfo>();
    spawn(move || {
        crate::network::bcast::rx(setting::CAB_BACKUP_PORT, cab_backup_recv_tx);
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
*/