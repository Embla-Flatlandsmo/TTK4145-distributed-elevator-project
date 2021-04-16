#![allow(dead_code)]

use std::net::TcpStream;
use std::sync::*;
use std::fmt;
use std::io::*;


#[derive(Clone, Debug)]
pub struct ElevatorHW {
        socket:     Arc<Mutex<TcpStream>>,
    pub num_floors: u8,
}


pub const DIRN_DOWN:    u8 = u8::MAX;
pub const DIRN_STOP:    u8 = 0;
pub const DIRN_UP:      u8 = 1;

#[derive(PartialEq, Debug)]
pub enum HardwareCommand{
    DoorLight{on: bool},
    MotorDirection{dirn: u8},
    CallButtonLight{floor: u8, call: u8, on: bool},
    StopLight{on: bool},
    FloorLight{floor: u8}
}

impl ElevatorHW {

    pub fn init(addr: &str, num_floors: u8) -> Result<ElevatorHW> {
        Ok(Self {
            socket: Arc::new(Mutex::new( TcpStream::connect(addr)? )),
            num_floors: num_floors,
        })
    }
    
    
    pub fn motor_direction(&self, dirn: u8){
        let buf = [1, dirn, 0, 0];
        let mut sock = self.socket.lock().unwrap();
        sock.write(&buf).unwrap();
    }
    
    pub fn call_button_light(&self, floor: u8, call: u8, on: bool){
        let buf = [2, call, floor, on as u8];
        let mut sock = self.socket.lock().unwrap();
        sock.write(&buf).unwrap();
    }
    
    pub fn floor_indicator(&self, floor: u8){
        let buf = [3, floor, 0, 0];
        let mut sock = self.socket.lock().unwrap();
        sock.write(&buf).unwrap();
    }
    
    pub fn door_light(&self, on: bool){
        let buf = [4, on as u8, 0, 0];
        let mut sock = self.socket.lock().unwrap();
        sock.write(&buf).unwrap();
    }
    
    pub fn stop_button_light(&self, on: bool){
        let buf = [5, on as u8, 0, 0];
        let mut sock = self.socket.lock().unwrap();
        sock.write(&buf).unwrap();
    }
    
    
    
    pub fn call_button(&self, floor: u8, call: u8) -> bool {
        let mut buf = [6, call, floor, 0];
        let mut sock = self.socket.lock().unwrap();
        sock.write(&mut buf).unwrap();
        sock.read(&mut buf).unwrap();
        return buf[1] != 0;
    }
    
    pub fn floor_sensor(&self) -> Option<u8> {
        let mut buf = [7, 0, 0, 0];
        let mut sock = self.socket.lock().unwrap();
        sock.write(&mut buf).unwrap();
        sock.read(&mut buf).unwrap();
        if buf[1] != 0 {
            Some(buf[2])
        } else {
            None
        }
    }
    
    pub fn stop_button(&self) -> bool {
        let mut buf = [8, 0, 0, 0];
        let mut sock = self.socket.lock().unwrap();
        sock.write(&mut buf).unwrap();
        sock.read(&mut buf).unwrap();
        return buf[1] != 0;
    }
    
    pub fn obstruction(&self) -> bool {
        let mut buf = [9, 0, 0, 0];
        let mut sock = self.socket.lock().unwrap();
        sock.write(&mut buf).unwrap();
        sock.read(&mut buf).unwrap();
        return buf[1] != 0;
    }

    /// Helper function so elev can interact with the hardwarecommand struct
    pub fn execute_command(&self, command: HardwareCommand) {
        match command {
            HardwareCommand::CallButtonLight{floor, call, on} => self.call_button_light(floor, call, on),
            HardwareCommand::DoorLight{on} => self.door_light(on),
            HardwareCommand::FloorLight{floor} => self.floor_indicator(floor),
            HardwareCommand::MotorDirection{dirn} => self.motor_direction(dirn),
            HardwareCommand::StopLight{on} => self.stop_button_light(on)
        }
    }
}

impl fmt::Display for ElevatorHW {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let addr = self.socket.lock().unwrap().peer_addr().unwrap();
        write!(f, "Elevator@{}({})", addr, self.num_floors)
    }
}






