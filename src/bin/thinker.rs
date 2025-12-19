use std::{
    net::{SocketAddr, UdpSocket},
    thread::sleep,
};

use clap::Parser;
use philosopher_nom_nom_ring::{
    NETWORK_BUFFER_SIZE, TICK_INTERVAL, Transceiver, messages::InitMessages,
};

use crate::thinker_lib::thinker::Thinker;

pub mod thinker_lib {
    pub mod thinker;
}

#[derive(Parser, Debug)]
pub struct ThinkerCli {
    address: SocketAddr,
    #[arg(short, long)]
    server_address: SocketAddr,
}

fn main() {
    let cli = ThinkerCli::parse();
    let socket = UdpSocket::bind(cli.address).unwrap();
    let local_address = socket.local_addr().unwrap();
    let transceiver = Transceiver::new(socket);
    transceiver.send(InitMessages::ThinkerRequest, &cli.server_address);

    let mut thinker = Thinker::new(transceiver);
    let mut buffer = [0; NETWORK_BUFFER_SIZE];

    log::info!("Started thinker {local_address}");
    loop {
        thinker.tick(&mut buffer);
        sleep(TICK_INTERVAL);
    }
}
