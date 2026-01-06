use std::net::{SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::thread::sleep;

use clap::{Parser, Subcommand};
use philosopher_nom_nom_ring::lib::config::Config;
use philosopher_nom_nom_ring::lib::fork::ForkRef;
use philosopher_nom_nom_ring::lib::messages::thinker_messages::TokenRef;
use philosopher_nom_nom_ring::lib::messages::{InitMessages, ThinkerMessage};
use philosopher_nom_nom_ring::lib::thinker::{Thinker, ThinkerInitParams, ThinkerRef};
use philosopher_nom_nom_ring::lib::transceiver::Transceiver;
use philosopher_nom_nom_ring::lib::utils::Id;
use philosopher_nom_nom_ring::lib::visualizer::VisualizerRef;
use philosopher_nom_nom_ring::{NETWORK_BUFFER_SIZE, TICK_INTERVAL, init_logger};
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Subcommand, Debug)]
enum Commands {
    Config {
        config_file: PathBuf,
    },
    InitServer {
        address: SocketAddr,
        #[arg(short, long)]
        save_config_dir: Option<PathBuf>,
        #[arg(short, long)]
        init_server: SocketAddr,
    },
}

#[derive(Parser, Debug)]
pub struct ThinkerCli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Serialize, Deserialize, Archive)]
struct ThinkerConfig {
    id: Id<Thinker>,
    address: SocketAddr,
    visualizer: Option<VisualizerRef>,
    forks: [ForkRef; 2],
    next_thinkers: Vec<ThinkerRef>,
    available_tokens: Vec<TokenRef>,
}

fn main() {
    init_logger();
    let cli = ThinkerCli::parse();

    let mut buffer = [0; NETWORK_BUFFER_SIZE];
    let mut unhandled_messages = vec![];
    let init_params = match cli.command {
        Commands::Config { config_file } => {
            let config = ThinkerConfig::read(&config_file);
            let socket = UdpSocket::bind(config.address).unwrap();
            let transceiver = Transceiver::new(socket);
            ThinkerInitParams {
                id: config.id,
                transceiver,
                unhandled_messages,
                forks: config.forks,
                next_thinkers: config.next_thinkers,
                // Config is used to restart a node if crashes
                // Token probably already regenerated from other nodes
                token: None,
                available_tokens: config.available_tokens,
                visualizer: config.visualizer,
            }
        }
        Commands::InitServer {
            address,
            save_config_dir,
            init_server,
        } => {
            let socket = UdpSocket::bind(address).unwrap();
            let transceiver = Transceiver::new(socket);
            let id = Id::random();

            transceiver.send_reliable(InitMessages::ThinkerRequest(id.clone()), &init_server);
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

            if let Some(path) = save_config_dir {
                ThinkerConfig {
                    id: id.clone(),
                    visualizer: init_params.visualizer.clone(),
                    address: transceiver.local_address(),
                    forks: init_params.forks.clone(),
                    next_thinkers: init_params.next_thinkers.clone(),
                    available_tokens: init_params.available_tokens.clone(),
                }
                .write(&path.join(format!("thinker_{}.conf", id.value)));
            }

            ThinkerInitParams {
                id,
                transceiver,
                unhandled_messages,
                forks: init_params.forks,
                next_thinkers: init_params.next_thinkers,
                token: init_params.token,
                available_tokens: init_params.available_tokens,
                visualizer: init_params.visualizer,
            }
        }
    };

    let mut thinker: Thinker = Thinker::new(init_params);
    thinker.print_started();
    loop {
        thinker.tick(&mut buffer);
        thinker.update_visualizer();
        sleep(TICK_INTERVAL);
    }
}
