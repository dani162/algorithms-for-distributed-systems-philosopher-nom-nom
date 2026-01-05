use std::net::{SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::thread::sleep;

use clap::{Parser, Subcommand};
use philosopher_nom_nom_ring::lib::config::Config;
use philosopher_nom_nom_ring::lib::fork::{Fork, ForkInitParams};
use philosopher_nom_nom_ring::lib::messages::{ForkMessages, InitMessages};
use philosopher_nom_nom_ring::lib::transceiver::{self, Transceiver};
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

#[derive(Debug, Serialize, Deserialize, Archive)]
pub struct ForkConfig {
    id: Id<Fork>,
    address: SocketAddr,
    visualizer: Option<VisualizerRef>,
}

#[derive(Parser, Debug)]
pub struct ForkCli {
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    init_logger();
    let cli = ForkCli::parse();

    let mut buffer = [0; NETWORK_BUFFER_SIZE];
    let mut unhandled_messages = vec![];
    let init_params = match cli.command {
        Commands::Config { config_file } => {
            let config = ForkConfig::read(&config_file);
            let socket = UdpSocket::bind(config.address).unwrap();
            let transceiver = Transceiver::new(socket);
            ForkInitParams {
                id: config.id,
                visualizer: config.visualizer,
                transceiver,
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

            transceiver.send_reliable(InitMessages::ForkRequest(id.clone()), &init_server);
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
            if let Some(path) = save_config_dir {
                ForkConfig {
                    id: id.clone(),
                    visualizer: visualizer.clone(),
                    address: transceiver.local_address(),
                }
                .write(&path.join(format!("fork_{}.conf", id.value)));
            }
            ForkInitParams {
                id,
                visualizer,
                transceiver,
            }
        }
    };

    let mut fork = Fork::new(init_params);
    fork.print_started();
    loop {
        fork.tick(&mut buffer);
        fork.update_visualizer();
        sleep(TICK_INTERVAL);
    }
}
