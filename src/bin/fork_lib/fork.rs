use std::net::{SocketAddr, UdpSocket};

pub struct Fork {
    currently_used: bool,
    queue: Vec<SocketAddr>,
    socket: UdpSocket,
}

pub enum ForkState {
    Unused,
    Used,
}

impl Fork {
    pub fn new(socket: UdpSocket) -> Self {
        Self {
            currently_used: false,
            queue: vec![],
            socket,
        }
    }

    pub fn tick(&self) {
        todo!()
    }
}
