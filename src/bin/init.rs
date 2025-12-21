use std::net::{SocketAddr, UdpSocket};

use clap::Parser;
use philosopher_nom_nom_ring::lib::messages::{InitMessages, InitThinkerParams};
use philosopher_nom_nom_ring::lib::thinker::ThinkerRef;
use philosopher_nom_nom_ring::lib::transceiver::Transceiver;
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
}

fn main() {
    init_logger();
    let cli = InitCli::parse();
    let socket = UdpSocket::bind(cli.address).unwrap();
    let mut waiting_forks: Vec<ForkRef> = vec![];
    let mut waiting_thinkers: Vec<ThinkerRef> = vec![];

    let transceiver: Transceiver = Transceiver::new(socket);

    let mut buffer = [0; NETWORK_BUFFER_SIZE];
    log::info!("Started init server");
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
            }
            if cli.thinker == waiting_thinkers.len() && cli.thinker == waiting_forks.len() {
                notify_entities(waiting_thinkers, waiting_forks, &transceiver, cli.tokens);
                log::info!("Notified all queued entities. Shutting down");
                return;
            }
        }
    }
}

fn notify_entities(
    mut thinkers: Vec<ThinkerRef>,
    mut forks: Vec<ForkRef>,
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
        });
        transceiver.send(message, &thinkers[i].address);
    }
}
