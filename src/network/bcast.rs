
use crossbeam_channel as cbc;
use serde;

#[path = "./sock.rs"]
mod sock;



pub fn tx<T: Clone + serde::Serialize>(port: u16, ch: cbc::Receiver<T>, burst_size: usize){

    let s = sock::new_tx(port).unwrap();

    loop {
        let data = ch.recv().unwrap();
        let serialized = serde_json::to_string(&data).unwrap();
        for _i in 0..burst_size {
            let res = s.send(serialized.as_bytes());
            match res {
                Ok(_res) => {},
                Err(_res) => {println!("Couldn't send bcast");}
            }
        }
    }
}

pub fn rx<T: serde::de::DeserializeOwned>(port: u16, ch: cbc::Sender<T>){
    let s = sock::new_rx(port).unwrap();
    let mut buf = [0; 1024];
    
    loop {
        let n = s.recv(&mut buf).unwrap();
        let msg = std::str::from_utf8(&buf[..n]).unwrap();
        // Only send the message on crossbeam channel if it actually is the data we want
        match serde_json::from_str::<T>(&msg) {
            Ok(data) => ch.send(data).unwrap(),
            _ => {}
        }
    }
}