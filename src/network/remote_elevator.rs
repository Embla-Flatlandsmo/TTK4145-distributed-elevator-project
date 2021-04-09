use crossbeam_channel as cbc;
use serde;
use crate::fsm::elevatorfsm::ElevatorInfo;
use crate::fsm::elevatorfsm::Elevator;
use std::time;
use std::collections::HashMap;
#[path = "./sock.rs"]
mod sock;

#[derive(Debug, Clone)]
pub struct RemoteElevatorUpdate {
    pub peers:  Vec<ElevatorInfo>,
    pub new:    Option<ElevatorInfo>,
    pub lost:   Vec<ElevatorInfo>,
}

pub fn tx<ElevatorInfo: serde::Serialize>(port: u16, ref mut elev: Elevator, tx_enable: cbc::Receiver<bool>){

    let s = sock::new_tx(port).unwrap();
    
    let mut enabled = true;

    let ticker = cbc::tick(time::Duration::from_millis(15));

    loop {
        cbc::select! {
            recv(tx_enable) -> enable => {
                enabled = enable.unwrap();
            },
            recv(ticker) -> _ => {
                if enabled {
                    let data = elev.get_info();
                    let serialized = serde_json::to_string(&data).unwrap();
                    let res = s.send(serialized.as_bytes());
                    match res {
                        Ok(res) => {},
                        Err(res) => {println!("Couldn't send bcast");}
                    }
                }
            }
        }
    }
}

pub fn rx<T: serde::de::DeserializeOwned>(port: u16, elev_info_update: cbc::Sender::<Vec<ElevatorInfo>>) {
    let timeout = time::Duration::from_millis(500);
    let s = sock::new_rx(port).unwrap();
    s.set_read_timeout(Some(timeout)).unwrap();
    
    let mut last_seen: HashMap<usize, time::Instant> = HashMap::new();
    let mut active_peers: HashMap<usize, ElevatorInfo> = HashMap::new();
    let mut lost_peers: HashMap<usize, ElevatorInfo> = HashMap::new();

    let mut buf = [0; 1024];

    loop {
        let mut modified = false;
        let mut e = RemoteElevatorUpdate{
            peers: Vec::new(),
            new: None,
            lost: Vec::new(),
        };

        let r = s.recv(&mut buf);
        let now = time::Instant::now();
        // Find new peers transmitting elevator info
        match r {
            Ok(n) => {
                let msg = std::str::from_utf8(&buf[..n]).unwrap();
                let info = serde_json::from_str::<ElevatorInfo>(&msg).unwrap();
                let id = info.clone().id;
                e.new = if !last_seen.contains_key(&id.clone()) {
                    modified = true;
                    Some(info.clone())
                } else {
                    None
                };
                last_seen.insert(id.clone(), now);
                active_peers.insert(id.clone(), info.clone());
                lost_peers.remove(&id.clone());
            }
            Err(_e) => {},
        }


        // Send cab calls to reconnecting node
        /*for id in e.new {
            if e.lost.contains_key(id){
                //broadcast_cab_calls_to_new_elevator(id);
            }
        }*/

        // Finding lost peers
        for (id, when_last_seen) in &last_seen {
            if now.duration_since(*when_last_seen) > timeout {
                e.lost.push(active_peers.get(id).unwrap().clone()); //what if node is not in elev_info_active?
                lost_peers.insert(id.clone(), active_peers.get(&id).unwrap().clone());
                modified = true;
            }
        }

        // .. and removing them
        for elev in &e.lost {
            last_seen.remove(&elev.id.clone());
            active_peers.remove(&elev.id.clone());
        }

        // Sending remote elevator update
        if modified {
            e.peers = active_peers.values().cloned().collect();
            elev_info_update.send(e.peers.clone()).unwrap();
        }
    }
}


