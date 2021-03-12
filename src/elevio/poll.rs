use std::time;
use std::thread;
use crossbeam_channel as cbc;

use super::elev;

#[derive(Debug, Copy, Clone)]
pub struct CallButton {
    pub floor:  u8,
    pub call:   u8,
}

pub fn call_buttons(elev: elev::ElevatorHW, ch: cbc::Sender<CallButton>, period: time::Duration){

    let mut prev = vec![[false; 3]; elev.num_floors.into()];
    loop {
        for f in 0..elev.num_floors {
            for c in 0..3 {
                let v = elev.call_button(f, c);
                if v  &&  prev[f as usize][c as usize] != v {
                    ch.send(CallButton{floor: f, call: c}).unwrap();
                }
                prev[f as usize][c as usize] = v;
            }
        }
        thread::sleep(period)
    }
}

pub fn floor_sensor(elev: elev::ElevatorHW, ch: cbc::Sender<u8>, period: time::Duration){
    
    let mut prev = u8::MAX;
    loop {
        match elev.floor_sensor() {
            Some(f) => 
                if f != prev {
                    ch.send(f).unwrap();
                    prev = f;
                },
            None => (),
        }
        thread::sleep(period)
    }
}

pub fn stop_button(elev: elev::ElevatorHW, ch: cbc::Sender<bool>, period: time::Duration){
    
    let mut prev = false;
    loop {
        let v = elev.stop_button();
        if prev != v {
            ch.send(v).unwrap();
            prev = v;
        }
        thread::sleep(period)
    }
}

pub fn obstruction(elev: elev::ElevatorHW, ch: cbc::Sender<bool>, period: time::Duration){
    
    let mut prev = false;
    loop {
        let v = elev.obstruction();
        if prev != v {
            ch.send(v).unwrap();
            prev = v;
        }
        thread::sleep(period)
    }
}
