use std::net::{SocketAddr, UdpSocket};
use std::thread::sleep;

use clap::Parser;
use philosopher_nom_nom_ring::lib::messages::{InitMessages, ThinkerMessage};
use philosopher_nom_nom_ring::lib::thinker::Thinker;
use philosopher_nom_nom_ring::lib::transceiver::Transceiver;
use philosopher_nom_nom_ring::lib::utils::Id;
use philosopher_nom_nom_ring::{NETWORK_BUFFER_SIZE, TICK_INTERVAL, init_logger};

#[derive(Parser, Debug)]
pub struct ThinkerCli {
    address: SocketAddr,
    #[arg(short, long)]
    init_server: SocketAddr,
}

fn main() {
    init_logger();
    let cli = ThinkerCli::parse();
    let socket = UdpSocket::bind(cli.address).unwrap();
    let local_address = socket.local_addr().unwrap();
    let transceiver = Transceiver::new(socket);
    let id = Id::random();
    transceiver.send_reliable(InitMessages::ThinkerRequest(id.clone()), &cli.init_server);

    let mut buffer = [0; NETWORK_BUFFER_SIZE];
    let mut unhandled_messages = vec![];

    let init_params = 'outer: loop {
        while let Some(message) = transceiver.receive::<ThinkerMessage>(&mut buffer) {
            match message {
                (ThinkerMessage::Init(init_thinker_params), _) => {
                    break 'outer init_thinker_params;
                }
                message => {
                    unhandled_messages.push(message);
                }
            }
        }
        sleep(TICK_INTERVAL);
    };

    let mut thinker: Thinker = Thinker::new(
        id.clone(),
        transceiver,
        unhandled_messages,
        init_params.forks,
        init_params.next_thinkers,
        init_params.token,
        init_params.visualizer,
        init_params.available_tokens,
    );

    log::info!("Started thinker {} {}", local_address, id);
    loop {
        thinker.tick(&mut buffer);
        thinker.update_visualizer();
        sleep(TICK_INTERVAL);
    }
}
