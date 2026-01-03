use std::{
    net::{SocketAddr, UdpSocket},
    thread::sleep,
};

use clap::Parser;
use philosopher_nom_nom_ring::lib::{transceiver::Transceiver, visualizer::Visualizer};
use philosopher_nom_nom_ring::{NETWORK_BUFFER_SIZE, TICK_INTERVAL};
use philosopher_nom_nom_ring::{
    init_logger,
    lib::messages::{InitMessages, VisualizerMessages},
};

#[derive(Parser, Debug)]
pub struct VisualizerCli {
    address: SocketAddr,
    #[arg(short, long)]
    init_server: SocketAddr,
}

fn main() {
    init_logger();
    let cli = VisualizerCli::parse();
    let socket = UdpSocket::bind(cli.address).unwrap();
    let transceiver = Transceiver::new(socket);
    transceiver.send_reliable(InitMessages::VisualizerRequest, &cli.init_server);

    let mut buffer = [0; NETWORK_BUFFER_SIZE];
    let mut unhandled_messages = vec![];

    let (thinkers, forks) = 'outer: loop {
        log::info!("Waiting for init");
        while let Some(message) = transceiver.receive::<VisualizerMessages>(&mut buffer) {
            log::info!("Got Message {:#?}", message);
            match message {
                (VisualizerMessages::Init { thinkers, forks }, _) => {
                    break 'outer (thinkers, forks);
                }
                message => {
                    unhandled_messages.push(message);
                }
            }
        }
        sleep(TICK_INTERVAL);
    };

    let mut visualizer = Visualizer::new(transceiver, thinkers, forks);

    log::info!("Started Visualizer");
    loop {
        visualizer.tick(&mut buffer);
        sleep(TICK_INTERVAL);
    }
}
