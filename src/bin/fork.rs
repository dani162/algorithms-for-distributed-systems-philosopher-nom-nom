use std::{
    net::{SocketAddr, UdpSocket},
    thread::sleep,
};

use clap::Parser;
use philosopher_nom_nom_ring::{TICK_INTERVAL, messages::InitRequests};

use crate::fork_lib::fork::Fork;

pub mod fork_lib {
    pub mod fork;
}

#[derive(Parser, Debug)]
pub struct ForkCli {
    address: SocketAddr,
    #[arg(short, long)]
    server_address: SocketAddr,
}

fn main() {
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let cli = ForkCli::parse();
    let socket = UdpSocket::bind(cli.address).unwrap();

    log::info!("Started fork {}", socket.local_addr().unwrap());
    let message = InitRequests::ForkRequest;
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&message).unwrap();
    socket.send_to(&bytes, cli.server_address).unwrap();

    let fork = Fork::new(socket);
    loop {
        fork.tick();
        sleep(TICK_INTERVAL);
    }
}
