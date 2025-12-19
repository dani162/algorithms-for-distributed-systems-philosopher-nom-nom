use std::{
    net::{SocketAddr, UdpSocket},
    thread::sleep,
};

use clap::Parser;
use philosopher_nom_nom_ring::{
    NETWORK_BUFFER_SIZE, TICK_INTERVAL, Transceiver, messages::InitMessages,
};

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
    let local_address = socket.local_addr().unwrap();
    let transceiver = Transceiver::new(socket);
    transceiver.send(InitMessages::ForkRequest, &cli.server_address);

    let mut fork = Fork::new(transceiver);
    let mut buffer = [0; NETWORK_BUFFER_SIZE];

    log::info!("Started fork {local_address}");
    loop {
        fork.tick(&mut buffer);
        sleep(TICK_INTERVAL);
    }
}
