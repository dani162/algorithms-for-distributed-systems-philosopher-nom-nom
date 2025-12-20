use std::net::{SocketAddr, UdpSocket};

use clap::Parser;
use philosopher_nom_nom_ring::lib::messages::{InitMessages, InitThinkerParams};
use philosopher_nom_nom_ring::lib::thinker::ThinkerRef;
use philosopher_nom_nom_ring::lib::transceiver::Transceiver;
use philosopher_nom_nom_ring::lib::{fork::ForkRef, messages::ThinkerMessage};
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
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let cli = InitCli::parse();
    let socket = UdpSocket::bind(cli.address).unwrap();
    let mut waiting_forks: Vec<ForkRef> = vec![];
    let mut waiting_thinkers: Vec<ThinkerRef> = vec![];

    let transceiver: Transceiver = Transceiver::new(socket);

    let mut buffer = [0; 1024];
    log::info!("Started init server");
    loop {
        while let Some((message, entity)) = transceiver.receive::<InitMessages>(&mut buffer) {
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
                notify_entities(waiting_thinkers, waiting_forks, &transceiver);
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
) {
    thinkers.shuffle(&mut rng());
    forks.shuffle(&mut rng());

    for i in 0..thinkers.len() {
        let message = match i {
            0 => ThinkerMessage::Init(InitThinkerParams {
                owns_token: true,
                forks: [forks.last().unwrap().clone(), forks[i].clone()],
                next_thinker: thinkers[i + 1].clone(),
            }),
            i if i == thinkers.len() - 1 => ThinkerMessage::Init(InitThinkerParams {
                owns_token: false,
                forks: [forks[i - 1].clone(), forks[i].clone()],
                next_thinker: thinkers[0].clone(),
            }),
            i => ThinkerMessage::Init(InitThinkerParams {
                owns_token: false,
                forks: [forks[i - 1].clone(), forks[i].clone()],
                next_thinker: thinkers[i + 1].clone(),
            }),
        };
        transceiver.send(message, &thinkers[i].address);
    }
}
