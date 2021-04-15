use crossbeam_channel as cbc;
use serde;
use crate::fsm::elevatorfsm::ElevatorInfo;
use crate::fsm::elevatorfsm::Elevator;
use std::time;
use crate::network::bcast;
use std::thread::*;
use std::collections::HashMap;
use crate::util::constants as setting;
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
                    send_bcast_tx.send(local_info.clone());
                }
            },
            recv(elev_info) -> new_info => {
                local_info = new_info.unwrap();
            }
        }
    }
}

pub fn remote_elev_info_rx<T: serde::de::DeserializeOwned>(port: u16, elev_info_update: cbc::Sender::<Vec<ElevatorInfo>>) {
    let timeout = time::Duration::from_millis(500);
    let s = sock::new_rx(port).unwrap();
    s.set_read_timeout(Some(timeout)).unwrap();
    
    let mut last_seen: HashMap<usize, time::Instant> = HashMap::new();
    let mut active_peers: HashMap<usize, ElevatorInfo> = HashMap::new();
    let mut lost_peers: HashMap<usize, ElevatorInfo> = HashMap::new();

    let mut buf = [0; 1024];

    loop {
        let mut modified = false;
        let mut peers = Vec::new();
        let mut lost = Vec::new();
        let mut new_elevator_id: usize = 0; //have to initialize with value

        let r = s.recv(&mut buf);
        let now = time::Instant::now();
        let mut new_elevator = false;

        // Find new peers transmitting elevator info
        // TODO: Make this receiver use bcast::rx?
        match r {
            Ok(n) => {
                let msg = std::str::from_utf8(&buf[..n]).unwrap();
                let elev_info = serde_json::from_str::<ElevatorInfo>(&msg).unwrap();
                let id = elev_info.clone().id;
                if !last_seen.contains_key(&id.clone()) {
                    modified = true;
                    new_elevator = true;
                    new_elevator_id = id.clone();
                }
                last_seen.insert(id.clone(), now);
                active_peers.insert(id.clone(), elev_info.clone());
                lost_peers.remove(&id.clone());
            }
            Err(e) => {},
        }

        // Send cab calls to reconnecting node
        if new_elevator{
            if lost_peers.contains_key(&new_elevator_id.clone()){
                //broadcast_cab_calls_to_new_elevator(id);
            }
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


