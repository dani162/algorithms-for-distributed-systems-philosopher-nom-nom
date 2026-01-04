use std::net::SocketAddr;

use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::fork::ForkRef;
use crate::lib::messages::VisualizerMessages;
use crate::lib::messages::visualizer_messages::{VisualizerForkState, VisualizerThinkerState};
use crate::lib::thinker::ThinkerRef;
use crate::lib::transceiver::Transceiver;

#[derive(Archive, Serialize, Deserialize, Clone, Debug)]
pub struct VisualizerRef {
    pub address: SocketAddr,
}

#[derive(Debug)]
pub struct Visualizer {
    transceiver: Transceiver,
    thinkers: Vec<(ThinkerRef, VisualizerThinkerState)>,
    forks: Vec<(ForkRef, VisualizerForkState)>,
}

impl Visualizer {
    pub fn new(transceiver: Transceiver, thinkers: Vec<ThinkerRef>, forks: Vec<ForkRef>) -> Self {
        Self {
            transceiver,
            thinkers: thinkers
                .into_iter()
                .map(|thinker| (thinker, VisualizerThinkerState::Thinking))
                .collect(),
            forks: forks
                .into_iter()
                .map(|fork| (fork, VisualizerForkState::Unused))
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
                self.forks
                    .iter_mut()
                    .find(|(fork, _)| fork.id.eq(&id))
                    .unwrap()
                    .1 = state;
            }
            VisualizerMessages::ThinkerStateChanged { id, state } => {
                self.thinkers
                    .iter_mut()
                    .find(|(thinker, _)| thinker.id.eq(&id))
                    .unwrap()
                    .1 = state;
            }
        }
    }

    pub fn print_state(&self) {
        print!("\x1B[2J\x1B[1;1H");
        self.thinkers.iter().zip(&self.forks).for_each(
            |((thinker, thinker_state), (fork, fork_state))| {
                enum UsedBy {
                    Above,
                    Bellow,
                }

                let fork_side = match fork_state {
                    VisualizerForkState::Unused => None,
                    VisualizerForkState::Used(id) if id.eq(&thinker.id) => Some(UsedBy::Above),
                    // should not happen that is it not the next one if order is not messed up
                    VisualizerForkState::Used(_) => Some(UsedBy::Bellow),
                };
                match &fork_side {
                    Some(UsedBy::Bellow) => println!("⬆️"),
                    _ => println!(),
                };

                let fork_state_char = match fork_state {
                    VisualizerForkState::Unused => "🔓",
                    VisualizerForkState::Used(_) => "🔒",
                };
                let fork_state_str = match fork_state {
                    VisualizerForkState::Unused => "Unused",
                    VisualizerForkState::Used(_) => "Used",
                };
                // Fork
                println!(
                    "🍴 [{}][{:-^15}] {}",
                    fork_state_char, fork_state_str, fork.id
                );

                match &fork_side {
                    Some(UsedBy::Above) => println!("⬇️"),
                    _ => println!(),
                };

                let thinker_state_char = match thinker_state {
                    VisualizerThinkerState::Thinking => "🤔",
                    VisualizerThinkerState::Hungry => "😩",
                    VisualizerThinkerState::WaitingForForks => "💤",
                    VisualizerThinkerState::Eating => "🧀",
                };
                let visualizer_state_str = match thinker_state {
                    VisualizerThinkerState::Thinking => "Thinking",
                    VisualizerThinkerState::Hungry => "Hungry",
                    VisualizerThinkerState::WaitingForForks => "WaitingForForks",
                    VisualizerThinkerState::Eating => "Eating",
                };
                // Thinker
                println!(
                    "🧐 [{}][{:-^15}] {}",
                    thinker_state_char, visualizer_state_str, thinker.id.value
                );
            },
        );
    }
}
