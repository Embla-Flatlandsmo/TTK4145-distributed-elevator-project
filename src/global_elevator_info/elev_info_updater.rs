use crossbeam_channel as cbc;
use serde;
use std::time;
use std::thread::*;
use std::collections::HashMap;

use crate::util::constants as setting;
use crate::local_elevator::fsm::elevatorfsm::ElevatorInfo;


///Transmitter local ElevatorInfo to network
pub fn local_elev_info_tx<ElevatorInfo: 'static + Clone + serde::Serialize + std::marker::Send>(
    elev_info: cbc::Receiver::<ElevatorInfo>){

    let (send_bcast_tx, send_bcast_rx) = cbc::unbounded::<ElevatorInfo>();
    {
    spawn(move || {
        crate::network_interface::bcast::tx(setting::ELEV_INFO_PORT, send_bcast_rx, 3);
    });
    }

    let ticker = cbc::tick(time::Duration::from_millis(setting::INFO_TRANSMIT_PERIOD_MILLISEC));
    let mut local_info: ElevatorInfo;

    cbc::select! {
        recv(elev_info) -> new_info => {
            local_info = new_info.unwrap();
        }
    }
    
    loop {
        cbc::select! {
            recv(ticker) -> _ => {
                    send_bcast_tx.send(local_info.clone()).unwrap();
            },
            recv(elev_info) -> new_info => {
                local_info = new_info.unwrap();
            }
        }
    }
}

///Reciver of other nodes local ElevatorInfo
pub fn remote_elev_info_rx<T: serde::de::DeserializeOwned>(
    elev_info_update: cbc::Sender::<Vec<ElevatorInfo>>,
    cab_backup_channel: cbc::Sender::<ElevatorInfo>){

    let timeout = time::Duration::from_millis(setting::TIME_UNTIL_PEER_LOST_MILLISEC);
    
    let (elev_info_recv_tx, elev_info_recv_rx) = cbc::unbounded::<ElevatorInfo>();
    spawn(move || {
        crate::network_interface::bcast::rx(setting::ELEV_INFO_PORT, elev_info_recv_tx);
    });

    let mut last_seen: HashMap<usize, time::Instant> = HashMap::new();
    let mut active_peers: HashMap<usize, ElevatorInfo> = HashMap::new();
    let mut lost_peers: HashMap<usize, ElevatorInfo> = HashMap::new();

    loop {
        let mut modified = false;
        let mut reconnected_elevator = false;
        let mut lost_peers_temp = Vec::new();

        let r = elev_info_recv_rx.recv_timeout(timeout);
        let now = time::Instant::now();

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
                        cab_backup_channel.send(lost_peers.get(&id).unwrap().clone()).unwrap();
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
            Err(_) => {},
        }   

        // Finding lost peers
        for (id, when_last_seen) in &last_seen {
            if now.duration_since(*when_last_seen) > timeout {
                lost_peers_temp.push(active_peers.get(id).unwrap().clone());
                lost_peers.insert(id.clone(), active_peers.get(&id).unwrap().clone());
                modified = true;
            }
        }

        // .. and removing them
        for elev in &lost_peers_temp {
            last_seen.remove(&elev.id.clone());
            active_peers.remove(&elev.id.clone());
        }

        // Sending remote elevator update
        if modified {
            let peers: Vec<ElevatorInfo>;
            peers = active_peers.values().cloned().collect();
            elev_info_update.send(peers.clone()).unwrap();
        }
    }
}
