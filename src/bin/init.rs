use std::net::{SocketAddr, UdpSocket};

use clap::Parser;
use philosopher_nom_nom_ring::lib::fork::ForkRef;
use philosopher_nom_nom_ring::lib::messages::ThinkerMessage;
use philosopher_nom_nom_ring::lib::messages::VisualizerMessages;
use philosopher_nom_nom_ring::lib::messages::thinker_messages::InitThinkerParams;
use philosopher_nom_nom_ring::lib::messages::thinker_messages::Token;
use philosopher_nom_nom_ring::lib::messages::{ForkMessages, InitMessages};
use philosopher_nom_nom_ring::lib::thinker::ThinkerRef;
use philosopher_nom_nom_ring::lib::transceiver::Transceiver;
use philosopher_nom_nom_ring::lib::visualizer::VisualizerRef;
use philosopher_nom_nom_ring::{NETWORK_BUFFER_SIZE, init_logger};
use rand::{rng, seq::SliceRandom};

#[derive(Parser, Debug)]
pub struct InitCli {
    address: SocketAddr,
    #[arg(long)]
    thinker: usize,
    #[arg(long)]
    next_thinkers_amount: usize,
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
                        log::info!("Set visualizer {entity}");
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
                let tokens = (0..cli.tokens)
                    .map(|index| Token::create(waiting_thinkers[index].id.clone()))
                    .collect();
                notify_entities(
                    waiting_thinkers,
                    waiting_forks,
                    tokens,
                    waiting_visualizer,
                    &transceiver,
                    cli.next_thinkers_amount,
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
    tokens: Vec<Token>,
    visualizer: Option<VisualizerRef>,
    transceiver: &Transceiver,
    amount_next_thinkers: usize,
) {
    thinkers.shuffle(&mut rng());
    forks.shuffle(&mut rng());

    for i in 0..thinkers.len() {
        let forks_of_thinker = (0..2)
            .map(|index| {
                let next_index = (index + i) % forks.len();
                forks[next_index].clone()
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let next_thinkers = (1..=amount_next_thinkers)
            .map(|index| {
                let next_index = (index + i) % thinkers.len();
                thinkers[next_index].clone()
            })
            .collect();

        let token = tokens.iter().find(|token| token.issuer.eq(&thinkers[i].id));

        let message = ThinkerMessage::Init(InitThinkerParams {
            token: token.cloned(),
            forks: forks_of_thinker,
            next_thinkers,
            visualizer: visualizer.clone(),
            available_tokens: tokens.iter().map(|token| token.into()).collect(),
        });
        transceiver.send_reliable(message, &thinkers[i].address);
    }
    forks.iter().for_each(|fork| {
        transceiver.send_reliable(ForkMessages::Init(visualizer.clone()), &fork.address);
    });
    if let Some(visualizer) = visualizer {
        transceiver.send_reliable(
            VisualizerMessages::Init { thinkers, forks },
            &visualizer.address,
        );
    }
}
