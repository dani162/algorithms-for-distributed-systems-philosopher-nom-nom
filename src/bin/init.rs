use std::net::{SocketAddr, UdpSocket};
use std::thread::sleep;
use std::time::Duration;

use clap::Parser;
use philosopher_nom_nom_ring::lib::messages::{
    ForkMessages, InitMessages, InitThinkerParams, VisualizerMessages,
};
use philosopher_nom_nom_ring::lib::thinker::ThinkerRef;
use philosopher_nom_nom_ring::lib::transceiver::Transceiver;
use philosopher_nom_nom_ring::lib::visualizer::VisualizerRef;
use philosopher_nom_nom_ring::lib::{fork::ForkRef, messages::ThinkerMessage};
use philosopher_nom_nom_ring::{NETWORK_BUFFER_SIZE, init_logger};
use rand::{rng, seq::SliceRandom};

#[derive(Parser, Debug)]
pub struct InitCli {
    address: SocketAddr,
    #[arg(long)]
    thinker: usize,
    #[arg(long)]
    tokens: usize,
    #[arg(long)]
    visualizer: bool,
}

fn main() {
    init_logger();
    let cli = InitCli::parse();
    let socket = UdpSocket::bind(cli.address).unwrap();
    let mut waiting_forks: Vec<ForkRef> = vec![];
    let mut waiting_thinkers: Vec<ThinkerRef> = vec![];
    let mut waiting_visualizer: Option<VisualizerRef> = None;

    let transceiver: Transceiver = Transceiver::new(socket);

    let mut buffer = [0; NETWORK_BUFFER_SIZE];
    log::info!("Started init server, {:?}", cli);
    loop {
        while let Some((message, entity)) = transceiver.receive::<InitMessages>(&mut buffer) {
            buffer = [0; NETWORK_BUFFER_SIZE];
            match message {
                InitMessages::ForkRequest(id) => {
                    if cli.thinker > waiting_forks.len() {
                        waiting_forks.push(ForkRef {
                            address: entity,
                            id,
                        });
                        log::info!("Added fork {entity} to queue");
                    } else {
                        log::warn!(
                            "Additional fork {entity} tried to connect, but queue was already full."
                        )
                    }
                }
                InitMessages::ThinkerRequest(id) => {
                    if cli.thinker > waiting_thinkers.len() {
                        waiting_thinkers.push(ThinkerRef {
                            address: entity,
                            id,
                        });
                        log::info!("Added thinker {entity} to queue");
                    } else {
                        log::warn!(
                            "Additional thinker {entity} tried to connect, but queue was already full."
                        )
                    }
                }
                InitMessages::VisualizerRequest => {
                    if cli.visualizer && waiting_visualizer.is_none() {
                        let _ = waiting_visualizer.insert(VisualizerRef { address: entity });
                    } else if waiting_visualizer.is_some() {
                        log::warn!(
                            "Additional visualizer {entity} tried to connect, but one is already waiting."
                        );
                    } else {
                        log::warn!(
                            "Expected no visualizer because --visualizer was not passed as an cli argument."
                        );
                    }
                }
            }
            if cli.thinker == waiting_thinkers.len()
                && cli.thinker == waiting_forks.len()
                && (!cli.visualizer || waiting_visualizer.is_some())
            {
                notify_entities(
                    waiting_thinkers,
                    waiting_forks,
                    waiting_visualizer,
                    &transceiver,
                    cli.tokens,
                );
                log::info!("Notified all queued entities. Shutting down");
                return;
            }
        }
    }
}

fn notify_entities(
    mut thinkers: Vec<ThinkerRef>,
    mut forks: Vec<ForkRef>,
    visualizer: Option<VisualizerRef>,
    transceiver: &Transceiver,
    amount_tokens: usize,
) {
    thinkers.shuffle(&mut rng());
    forks.shuffle(&mut rng());

    for i in 0..thinkers.len() {
        let is_last = i + 1 == thinkers.len();
        let owns_token = i < amount_tokens;

        let next_fork = if is_last {
            forks.first().unwrap().clone()
        } else {
            forks[i + 1].clone()
        };

        let next_thinker = if is_last {
            thinkers.first().unwrap().clone()
        } else {
            thinkers[i + 1].clone()
        };

        let message = ThinkerMessage::Init(InitThinkerParams {
            owns_token,
            forks: [forks[i].clone(), next_fork],
            next_thinker,
            visualizer: visualizer.clone(),
        });
        transceiver.send_reliable(message, &thinkers[i].address);
    }
    forks.iter().for_each(|fork| {
        transceiver.send_reliable(ForkMessages::Init(visualizer.clone()), &fork.address);
    });
    println!("{:#?}", visualizer);
    if let Some(visualizer) = visualizer {
        transceiver.send_reliable(
            VisualizerMessages::Init { thinkers, forks },
            &visualizer.address,
        );
    }
    sleep(Duration::from_secs(1000));
}
