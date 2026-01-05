use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::Instant;

use rkyv::{Archive, Deserialize, Serialize};

use crate::KEEP_ALIVE_TIMEOUT;
use crate::lib::messages::visualizer_messages::VisualizerForkState;
use crate::lib::messages::{ForkMessages, ThinkerMessage, VisualizerMessages};
use crate::lib::thinker::ThinkerRef;
use crate::lib::transceiver::Transceiver;
use crate::lib::utils::{EntityType, Id};
use crate::lib::visualizer::VisualizerRef;

#[derive(Archive, Serialize, Deserialize, Clone, Debug)]
pub struct ForkRef {
    pub address: SocketAddr,
    pub id: Id<Fork>,
}

#[derive(Debug)]
enum ForkState {
    Unused,
    Used {
        thinker: ThinkerRef,
        last_seen_at: Instant,
    },
}

impl From<&ForkState> for VisualizerForkState {
    fn from(val: &ForkState) -> Self {
        match val {
            ForkState::Unused => VisualizerForkState::Unused,
            ForkState::Used { thinker, .. } => VisualizerForkState::Used(thinker.id.clone()),
        }
    }
}

#[derive(Debug)]
struct QueuedThinker {
    last_seen_at: Instant,
    thinker: ThinkerRef,
}

pub struct ForkInitParams {
    pub id: Id<Fork>,
    pub transceiver: Transceiver,
    pub visualizer: Option<VisualizerRef>,
    pub unhandled_messages: Vec<(ForkMessages, SocketAddr)>,
}

#[derive(Debug)]
pub struct Fork {
    pub id: Id<Fork>,
    state: ForkState,
    queue: VecDeque<QueuedThinker>,
    transceiver: Transceiver,
    visualizer: Option<VisualizerRef>,
}

impl Fork {
    pub fn new(init_params: ForkInitParams) -> Self {
        let mut fork = Self {
            id: init_params.id,
            state: ForkState::Unused,
            queue: VecDeque::new(),
            transceiver: init_params.transceiver,
            visualizer: init_params.visualizer,
        };
        init_params
            .unhandled_messages
            .into_iter()
            .for_each(|(message, entity)| {
                fork.handle_message(message, entity);
            });
        fork
    }

    pub fn print_started(&self) {
        log::info!(
            "Started fork {} {}",
            self.transceiver.local_address(),
            self.id,
        )
    }

    pub fn tick(&mut self, buffer: &mut [u8]) {
        while let Some((message, entity)) = self.transceiver.receive::<ForkMessages>(buffer) {
            self.handle_message(message, entity);
        }
        self.update_state();
    }

    pub fn handle_message(&mut self, message: ForkMessages, entity: SocketAddr) {
        match message {
            ForkMessages::Take(thinker_id) => {
                self.queue.push_back(QueuedThinker {
                    last_seen_at: Instant::now(),
                    thinker: ThinkerRef {
                        id: thinker_id,
                        address: entity,
                    },
                });
                log::info!("Queued Thinker {entity} at position {}", self.queue.len());
            }
            ForkMessages::KeepAlive(id) => {
                match &mut self.state {
                    ForkState::Unused => {
                        log::warn!(
                            "Got keep alive from {entity}, but fork is currently not in use"
                        );
                    }
                    ForkState::Used {
                        thinker,
                        last_seen_at,
                    } => {
                        if thinker.id.eq(&id) {
                            *last_seen_at = Instant::now();
                            self.transceiver
                                .send(ThinkerMessage::ForkAlive(self.id.clone()), &entity);
                        }
                        // TODO: here is case needed that sets keep alive for queued entites
                        //  and responds to those
                        else {
                            log::warn!(
                                "Got keep alive from {entity}, but fork is currently used by {}",
                                thinker.address
                            );
                        }
                    }
                };
            }
            ForkMessages::Release => match self.state {
                ForkState::Used { .. } => {
                    self.state = ForkState::Unused;
                    log::info!("Fork released by {entity}");
                }
                ForkState::Unused => {
                    log::error!("Got release message from {entity}, but is currently not used");
                }
            },
            ForkMessages::Init(_) => {
                log::error!("Already initialized but got init message from {entity}");
            }
        }
    }

    pub fn update_state(&mut self) {
        match &self.state {
            ForkState::Unused => {
                if let Some(next) = self.queue.pop_front() {
                    self.state = ForkState::Used {
                        thinker: next.thinker.clone(),
                        last_seen_at: next.last_seen_at,
                    };
                    self.transceiver.send(
                        ThinkerMessage::TakeForkAccepted(self.id.clone()),
                        &next.thinker.address,
                    );
                    log::info!("Fork taken by {}", next.thinker.address);
                }
            }
            ForkState::Used {
                thinker,
                last_seen_at,
            } => {
                if last_seen_at.elapsed() > KEEP_ALIVE_TIMEOUT {
                    let thinker = thinker.clone();
                    log::warn!(
                        "No keep alive from thinker {}. Releasing fork access",
                        thinker.address
                    );
                    self.state = ForkState::Unused;
                }
            }
        }
    }

    pub fn update_visualizer(&self) {
        if let Some(visualizer) = &self.visualizer {
            self.transceiver.send(
                VisualizerMessages::ForkStateChanged {
                    id: self.id.clone(),
                    state: (&self.state).into(),
                },
                &visualizer.address,
            );
        }
    }
}

impl EntityType for Fork {
    fn display_name() -> &'static str {
        "Fork"
    }
}
