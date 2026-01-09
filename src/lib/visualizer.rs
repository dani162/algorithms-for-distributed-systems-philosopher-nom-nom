use std::net::SocketAddr;
use std::time::Instant;

use colored::{ColoredString, Colorize};
use rkyv::{Archive, Deserialize, Serialize};

use crate::KEEP_ALIVE_TIMEOUT;
use crate::lib::fork::ForkRef;
use crate::lib::messages::VisualizerMessages;
use crate::lib::messages::visualizer_messages::{
    VisualizerForkState, VisualizerThinkerAvailableTokenState, VisualizerThinkerState,
};
use crate::lib::thinker::ThinkerRef;
use crate::lib::transceiver::Transceiver;

#[derive(Archive, Serialize, Deserialize, Clone, Debug)]
pub struct VisualizerRef {
    pub address: SocketAddr,
}

#[derive(Debug)]
struct ThinkerState {
    thinker: ThinkerRef,
    visualizer_thinker_state: VisualizerThinkerState,
    visualizer_available_token_state: Vec<VisualizerThinkerAvailableTokenState>,
    last_seen: Instant,
}

#[derive(Debug)]
struct ForkState {
    fork: ForkRef,
    visualizer_fork_state: VisualizerForkState,
    last_seen: Instant,
}

#[derive(Debug)]
pub struct Visualizer {
    transceiver: Transceiver,
    thinkers: Vec<ThinkerState>,
    forks: Vec<ForkState>,
}

impl Visualizer {
    pub fn new(transceiver: Transceiver, thinkers: Vec<ThinkerRef>, forks: Vec<ForkRef>) -> Self {
        Self {
            transceiver,
            thinkers: thinkers
                .into_iter()
                .map(|thinker| ThinkerState {
                    thinker,
                    visualizer_thinker_state: VisualizerThinkerState::Thinking,
                    last_seen: Instant::now(),
                    visualizer_available_token_state: vec![],
                })
                .collect(),
            forks: forks
                .into_iter()
                .map(|fork| ForkState {
                    fork,
                    visualizer_fork_state: VisualizerForkState::Unused,
                    last_seen: Instant::now(),
                })
                .collect(),
        }
    }

    pub fn tick(&mut self, buffer: &mut [u8]) {
        while let Some((message, entity)) = self.transceiver.receive::<VisualizerMessages>(buffer) {
            self.handle_message(message, entity);
        }
        self.print_state();
    }

    pub fn handle_message(&mut self, message: VisualizerMessages, entity: SocketAddr) {
        match message {
            VisualizerMessages::Init { .. } => {
                log::error!("Already initialized but got init message from {entity}");
            }
            VisualizerMessages::ForkStateChanged { id, state } => {
                let el = self
                    .forks
                    .iter_mut()
                    .find(|fork_state| fork_state.fork.id.eq(&id))
                    .unwrap();
                el.visualizer_fork_state = state;
                el.last_seen = Instant::now();
            }
            VisualizerMessages::ThinkerStateChanged {
                id,
                state,
                token_state,
            } => {
                let el = self
                    .thinkers
                    .iter_mut()
                    .find(|thinker_state| thinker_state.thinker.id.eq(&id))
                    .unwrap();
                el.visualizer_thinker_state = state;
                el.last_seen = Instant::now();
                el.visualizer_available_token_state = token_state;
            }
        }
    }

    pub fn print_state(&self) {
        print!("\x1B[2J\x1B[1;1H");
        self.thinkers
            .iter()
            .zip(&self.forks)
            .for_each(|(thinker_state, fork_state)| {
                enum UsedBy {
                    Above,
                    Bellow,
                }

                let fork_side = match &fork_state.visualizer_fork_state {
                    VisualizerForkState::Unused => None,
                    VisualizerForkState::Used(id) if id.eq(&thinker_state.thinker.id) => {
                        Some(UsedBy::Above)
                    }
                    // should not happen that is it not the next one if order is not messed up
                    VisualizerForkState::Used(_) => Some(UsedBy::Bellow),
                };
                match &fork_side {
                    Some(UsedBy::Bellow) if fork_state.last_seen.elapsed() < KEEP_ALIVE_TIMEOUT => {
                        println!("⬆️")
                    }
                    _ => println!(),
                };

                let fork_state_char = match fork_state.visualizer_fork_state {
                    VisualizerForkState::Unused => "🔓",
                    VisualizerForkState::Used(_) => "🔒",
                };
                let fork_state_str = match fork_state.visualizer_fork_state {
                    VisualizerForkState::Unused => "Unused",
                    VisualizerForkState::Used(_) => "Used",
                };
                // Fork
                let message = format!(
                    "🍴 [{}][{:-^15}]    {}",
                    fork_state_char, fork_state_str, fork_state.fork.id
                );
                println!(
                    "{}",
                    match fork_state.last_seen.elapsed().cmp(&KEEP_ALIVE_TIMEOUT) {
                        std::cmp::Ordering::Less | std::cmp::Ordering::Equal =>
                            ColoredString::from(format!(
                                "{} ({:?})",
                                message,
                                fork_state.last_seen.elapsed()
                            )),
                        std::cmp::Ordering::Greater => ColoredString::from(format!(
                            "{} {}",
                            message.strikethrough().dimmed(),
                            "(dead)".red()
                        )),
                    }
                );

                match &fork_side {
                    Some(UsedBy::Above) if fork_state.last_seen.elapsed() < KEEP_ALIVE_TIMEOUT => {
                        println!("⬇️")
                    }
                    _ => println!(),
                };

                let thinker_state_char = match thinker_state.visualizer_thinker_state {
                    VisualizerThinkerState::Thinking => "🤔",
                    VisualizerThinkerState::Hungry => "😩",
                    VisualizerThinkerState::WaitingForForks { .. } => "💤",
                    VisualizerThinkerState::Eating { .. } => "🧀",
                };
                let visualizer_state_str = match thinker_state.visualizer_thinker_state {
                    VisualizerThinkerState::Thinking => "Thinking",
                    VisualizerThinkerState::Hungry => "Hungry",
                    VisualizerThinkerState::WaitingForForks { .. } => "WaitingForForks",
                    VisualizerThinkerState::Eating { .. } => "Eating",
                };
                let message = format!(
                    "🧐 [{}][{:-^15}] {}",
                    thinker_state_char, visualizer_state_str, thinker_state.thinker.id
                );
                // Thinker
                println!(
                    "{} [tnsf: {}] [{}]",
                    match thinker_state.last_seen.elapsed().cmp(&KEEP_ALIVE_TIMEOUT) {
                        std::cmp::Ordering::Less | std::cmp::Ordering::Equal =>
                            ColoredString::from(format!(
                                "{} ({:>4}ms)",
                                message,
                                thinker_state.last_seen.elapsed().as_millis()
                            )),
                        std::cmp::Ordering::Greater => ColoredString::from(format!(
                            "{} {}",
                            message.strikethrough().dimmed(),
                            "(dead)".red()
                        )),
                    },
                    thinker_state
                        .visualizer_available_token_state
                        .iter()
                        .map(|el| match el {
                            VisualizerThinkerAvailableTokenState::Passive { not_seen_for } =>
                                format!("{:>4?}ms", not_seen_for.as_millis()),
                            VisualizerThinkerAvailableTokenState::Propose {
                                propose_version,
                                token_version,
                            } => {
                                format!(" p{propose_version}->v{token_version} ")
                            }
                        })
                        .collect::<Vec<String>>()
                        .join(","),
                    match &thinker_state.visualizer_thinker_state {
                        VisualizerThinkerState::Thinking => "".to_string(),
                        VisualizerThinkerState::Hungry => "".to_string(),
                        VisualizerThinkerState::WaitingForForks { token }
                        | VisualizerThinkerState::Eating { token } => format!(
                            "tv: {}, id: {:4}",
                            token.version,
                            token.id.value.to_string().get(0..4).unwrap()
                        ),
                    }
                );
            });
        println!();
        println!("tnsf = token not seen for");
        println!("tv = token version");
        println!("p{{propose version number}}->v{{token version number}}");
    }
}
