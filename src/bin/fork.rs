use std::{
    net::{SocketAddr, UdpSocket},
    thread::sleep,
};

use clap::Parser;
use philosopher_nom_nom_ring::{
    NETWORK_BUFFER_SIZE, TICK_INTERVAL, Transceiver,
    fork_lib::fork::Fork,
    messages::{Id, InitMessages},
};

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

    let mut buffer = [0; NETWORK_BUFFER_SIZE];

    let id = Id::random();
    transceiver.send(InitMessages::ForkRequest(id.clone()), &cli.server_address);
    let mut fork = Fork::new(id, transceiver);
    log::info!("Started fork {local_address}");
    loop {
        fork.tick(&mut buffer);
        sleep(TICK_INTERVAL);
    }
}
