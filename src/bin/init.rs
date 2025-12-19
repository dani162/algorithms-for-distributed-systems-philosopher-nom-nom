use std::net::{SocketAddr, UdpSocket};

use clap::Parser;
use philosopher_nom_nom_ring::{
    fork_lib::fork::ForkRef,
    messages::{InitMessages, InitThinkerParams, ThinkerMessage},
};
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
    let mut waiting_thinkers: Vec<SocketAddr> = vec![];

    let mut buf = [0; 1024];

    log::info!("Started init server");
    loop {
        let (_, entity) = socket.recv_from(&mut buf).unwrap();
        let message = rkyv::from_bytes::<InitMessages, rkyv::rancor::Error>(&buf).unwrap();
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
            InitMessages::ThinkerRequest(_) => {
                if cli.thinker > waiting_thinkers.len() {
                    waiting_thinkers.push(entity);
                    log::info!("Added thinker {entity} to queue");
                } else {
                    log::warn!(
                        "Additional thinker {entity} tried to connect, but queue was already full."
                    )
                }
            }
        }
        if cli.thinker == waiting_thinkers.len() && cli.thinker == waiting_forks.len() {
            notify_entities(waiting_thinkers, waiting_forks, &socket);
            log::info!("Notified all queued entities. Shutting down");
            return;
        }
    }
}

fn notify_entities(mut thinkers: Vec<SocketAddr>, mut forks: Vec<ForkRef>, socket: &UdpSocket) {
    thinkers.shuffle(&mut rng());
    forks.shuffle(&mut rng());

    for i in 0..thinkers.len() {
        let message = match i {
            0 => ThinkerMessage::Init(InitThinkerParams {
                owns_token: true,
                forks: [forks.last().unwrap().clone(), forks[i].clone()],
                next_thinker: thinkers[i + 1],
            }),
            i if i == thinkers.len() - 1 => ThinkerMessage::Init(InitThinkerParams {
                owns_token: false,
                forks: [forks[i - 1].clone(), forks[i].clone()],
                next_thinker: thinkers[0],
            }),
            i => ThinkerMessage::Init(InitThinkerParams {
                owns_token: false,
                forks: [forks[i - 1].clone(), forks[i].clone()],
                next_thinker: thinkers[i + 1],
            }),
        };
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&message).unwrap();
        socket.send_to(&bytes, thinkers[i]).unwrap();
    }
}
