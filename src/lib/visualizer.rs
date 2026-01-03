use std::net::SocketAddr;

use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::{
    fork::ForkRef,
    messages::{VisualizerForkState, VisualizerMessages, VisualizerThinkerState},
    thinker::ThinkerRef,
    transceiver::Transceiver,
};

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
            VisualizerMessages::ForkStateChanged { fork, state } => {
                self.forks
                    .iter_mut()
                    .find(|(el, _)| el.id.eq(&fork.id))
                    .unwrap()
                    .1 = state;
            }
            VisualizerMessages::ThinkerStateChanged { thinker, state } => {
                self.thinkers
                    .iter_mut()
                    .find(|(el, _)| el.id.eq(&thinker.id))
                    .unwrap()
                    .1 = state;
            }
        }
    }

    pub fn print_state(&self) {
        print!("\x1B[2J\x1B[1;1H");
        self.thinkers
            .iter()
            .zip(&self.forks)
            .enumerate()
            .for_each(|(index, (thinker, fork))| {
                println!("T{} - {:?}", index, thinker.1);
                println!();
                println!("F{} - {:?}", index, fork.1);
                if index < self.forks.len() - 1 {
                    println!()
                }
            });
    }
}
