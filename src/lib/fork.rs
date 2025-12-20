use std::collections::VecDeque;
use std::net::SocketAddr;

use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::messages::{ForkMessages, ThinkerMessage};
use crate::lib::transceiver::Transceiver;
use crate::lib::utils::{EntityType, Id};

#[derive(Archive, Serialize, Deserialize, Clone)]
pub struct ForkRef {
    pub address: SocketAddr,
    pub id: Id<Fork>,
}

enum ForkState {
    Unused,
    Used,
}

pub struct Fork {
    pub id: Id<Fork>,
    state: ForkState,
    queue: VecDeque<SocketAddr>,
    transceiver: Transceiver,
}

impl Fork {
    pub fn new(id: Id<Fork>, transceiver: Transceiver) -> Self {
        Self {
            id,
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
                            .send(ThinkerMessage::TakeForkAccepted(self.id.clone()), &entity);
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
                                .send(ThinkerMessage::TakeForkAccepted(self.id.clone()), &next);
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

impl EntityType for Fork {
    fn display_name() -> &'static str {
        "Fork"
    }
}
