

use std::str;
use std::time;
use std::collections::HashMap;

use crossbeam_channel as cbc;

#[path = "./sock.rs"]
mod sock;

#[derive(Debug)]
pub struct PeerUpdate {
    pub peers:  Vec<String>,
    pub new:    Option<String>,
    pub lost:   Vec<String>,
}

pub fn tx(port: u16, id: String, tx_enable: cbc::Receiver<bool>){

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
                    s.send(id.as_bytes()).unwrap();
                }
            },
        }
    }

}

pub fn rx(port: u16, peer_update: cbc::Sender<PeerUpdate>){

    let timeout = time::Duration::from_millis(500);
    let s = sock::new_rx(port).unwrap();
    s.set_read_timeout(Some(timeout)).unwrap();
    
    let mut last_seen: HashMap<String, time::Instant> = HashMap::new();
    let mut buf = [0; 1024];
    
    loop {
        let mut modified = false;
        let mut p = PeerUpdate{
            peers: Vec::new(),
            new: None,
            lost: Vec::new(),
        };
        
        let r = s.recv(&mut buf);
        let now = time::Instant::now();
        
        // Finding new peers
        match r {
            Ok(n) => {
                let id = str::from_utf8(&buf[..n]).unwrap();
                p.new = if !last_seen.contains_key(id) {
                    modified = true;
                    Some(id.to_string())
                } else {
                    None
                };
                last_seen.insert(id.to_string(), now);
            },
            Err(_e) => {},
        }
        
        // Finding lost peers
        for (id, when) in &last_seen {
            if now - *when > timeout {
                p.lost.push(id.to_string());
                modified = true;
            }
        }
        //  .. and removing them
        for id in &p.lost {
            last_seen.remove(id);
        }
        
        // Sending peer update
        if modified {
            p.peers = last_seen.keys().cloned().collect();
            p.peers.sort();
            p.lost.sort();
            peer_update.send(p).unwrap();
        }
    }

}

