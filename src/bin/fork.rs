use std::net::{SocketAddr, UdpSocket};
use std::thread::sleep;

use clap::Parser;
use philosopher_nom_nom_ring::lib::fork::Fork;
use philosopher_nom_nom_ring::lib::messages::{ForkMessages, InitMessages};
use philosopher_nom_nom_ring::lib::transceiver::Transceiver;
use philosopher_nom_nom_ring::lib::utils::Id;
use philosopher_nom_nom_ring::{NETWORK_BUFFER_SIZE, TICK_INTERVAL, init_logger};

#[derive(Parser, Debug)]
pub struct ForkCli {
    address: SocketAddr,
    #[arg(short, long)]
    init_server: SocketAddr,
}

fn main() {
    init_logger();
    let cli = ForkCli::parse();
    let socket = UdpSocket::bind(cli.address).unwrap();
    let local_address = socket.local_addr().unwrap();
    let transceiver = Transceiver::new(socket);

    let mut buffer = [0; NETWORK_BUFFER_SIZE];
    let mut unhandled_messages = vec![];

    let id = Id::random();
    transceiver.send_reliable(InitMessages::ForkRequest(id.clone()), &cli.init_server);
    let visualizer = 'outer: loop {
        while let Some(message) = transceiver.receive::<ForkMessages>(&mut buffer) {
            match message {
                (ForkMessages::Init(init_thinker_params), _) => {
                    break 'outer init_thinker_params;
                }
                message => {
                    unhandled_messages.push(message);
                }
            }
        }
        sleep(TICK_INTERVAL);
    };

    let mut fork = Fork::new(id.clone(), transceiver, visualizer);
    log::info!("Started fork {local_address} {id}");
    loop {
        fork.tick(&mut buffer);
        fork.update_visualizer();
        sleep(TICK_INTERVAL);
    }
}
