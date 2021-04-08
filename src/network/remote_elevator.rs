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

pub fn tx<T: serde::Serialize>(port: u16, ref mut elev: Elevator, tx_enable: cbc::Receiver<bool>){

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

pub fn rx<T: serde::de::DeserializeOwned>(port: u16, elev_info_update: cbc::Sender::<RemoteElevatorUpdate>) {
    let timeout = time::Duration::from_millis(500);
    let s = sock::new_rx(port).unwrap();
    s.set_read_timeout(Some(timeout)).unwrap();
    
    let mut last_seen: HashMap<String, time::Instant> = HashMap::new();
    /// Maps ID to its corresponding elevator info
    let mut elev_info: HashMap<String, ElevatorInfo> = HashMap::new();
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
                elev_info.insert(id.clone(), info.clone());
            }
            Err(_e) => {},
        }

        // Finding lost peers
        for (id, when) in &last_seen {
            if now - *when > timeout {
                //e.lost.push(*elev_info.get(id).unwrap());
                modified = true;
            }
        }

        // .. and removing them
        for elev in &e.lost {
            last_seen.remove(&elev.id.clone());
            elev_info.remove(&elev.id.clone());
        }

        // Sending remote elevator update
        if modified {
            //e.peers = elev_info.keys().cloned().collect();
            elev_info_update.send(e).unwrap();
        }
    }
}

