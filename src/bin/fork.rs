use std::net::{SocketAddr, UdpSocket};
use std::thread::sleep;

use clap::Parser;
use philosopher_nom_nom_ring::lib::fork::Fork;
use philosopher_nom_nom_ring::lib::messages::InitMessages;
use philosopher_nom_nom_ring::lib::transceiver::Transceiver;
use philosopher_nom_nom_ring::lib::utils::Id;
use philosopher_nom_nom_ring::{NETWORK_BUFFER_SIZE, TICK_INTERVAL};

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
