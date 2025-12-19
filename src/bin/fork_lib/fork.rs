use std::collections::VecDeque;
use std::net::SocketAddr;

use philosopher_nom_nom_ring::Transceiver;
use philosopher_nom_nom_ring::messages::{ForkMessages, ThinkerMessages};

pub struct Fork {
    state: ForkState,
    queue: VecDeque<SocketAddr>,
    transceiver: Transceiver,
}

pub enum ForkState {
    Unused,
    Used,
}

impl Fork {
    pub fn new(transceiver: Transceiver) -> Self {
        Self {
            state: ForkState::Unused,
            queue: VecDeque::new(),
            transceiver,
        }
    }

    pub fn tick(&mut self, buffer: &mut [u8]) {
        while let Some((message, entity)) = self.transceiver.receive::<ForkMessages>(buffer) {
            match message {
                ForkMessages::Take => match self.state {
                    ForkState::Unused => {
                        self.state = ForkState::Used;
                        self.transceiver
                            .send(ThinkerMessages::TakeForkAccepted, &entity);
                        log::info!("Fork taken by {entity}");
                    }
                    ForkState::Used => {
                        self.queue.push_back(entity);
                        log::info!("Queued Thinker {entity} at position {}", self.queue.len());
                    }
                },
                ForkMessages::Release => match self.state {
                    ForkState::Unused => {
                        log::error!("Got release message from {entity}, but is currently not used");
                    }
                    ForkState::Used => {
                        if self.queue.is_empty() {
                            self.state = ForkState::Unused;
                            log::info!("Fork released by {entity}");
                        } else {
                            let next = self.queue.pop_front().unwrap();
                            self.transceiver
                                .send(ThinkerMessages::TakeForkAccepted, &next);
                            log::info!(
                                "Fork released by {entity}, fork given to {next}, {} thinkers in queue remaining",
                                self.queue.len()
                            );
                        }
                    }
                },
            }
        }
    }
}
