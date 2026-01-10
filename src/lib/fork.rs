use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::Instant;

use rkyv::{Archive, Deserialize, Serialize};

use crate::KEEP_ALIVE_TIMEOUT;
use crate::lib::messages::thinker_messages::ForkState;
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
enum ForkStateInternal {
    Unused,
    Used {
        thinker: ThinkerRef,
        last_seen_at: Instant,
    },
}

impl From<&ForkStateInternal> for VisualizerForkState {
    fn from(val: &ForkStateInternal) -> Self {
        match val {
            ForkStateInternal::Unused => VisualizerForkState::Unused,
            ForkStateInternal::Used { thinker, .. } => {
                VisualizerForkState::Used(thinker.id.clone())
            }
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
    state: ForkStateInternal,
    queue: VecDeque<QueuedThinker>,
    transceiver: Transceiver,
    visualizer: Option<VisualizerRef>,
}

impl Fork {
    pub fn new(init_params: ForkInitParams) -> Self {
        let mut fork = Self {
            id: init_params.id,
            state: ForkStateInternal::Unused,
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

    pub fn reset(self) -> Self {
        Self::new(ForkInitParams {
            id: self.id,
            transceiver: self.transceiver.reset(),
            visualizer: self.visualizer,
            unhandled_messages: vec![],
        })
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
            ForkMessages::KeepAlive(thinker_id) => {
                match &mut self.state {
                    ForkStateInternal::Unused => {
                        if let Some(queued) = self
                            .queue
                            .iter_mut()
                            .find(|queued_thinker| queued_thinker.thinker.id.eq(&thinker_id))
                        {
                            queued.last_seen_at = Instant::now();
                            self.transceiver.send(
                                ThinkerMessage::ForkAlive {
                                    id: self.id.clone(),
                                    state: ForkState::Queued,
                                },
                                &queued.thinker.address,
                            );
                        } else {
                            let queued_thinker = QueuedThinker {
                                last_seen_at: Instant::now(),
                                thinker: ThinkerRef {
                                    id: thinker_id.clone(),
                                    address: entity,
                                },
                            };
                            self.transceiver.send(
                                ThinkerMessage::ForkAlive {
                                    id: self.id.clone(),
                                    state: ForkState::Queued,
                                },
                                &queued_thinker.thinker.address,
                            );
                            self.queue.push_back(queued_thinker);
                            log::info!(
                                "Queued Thinker {} at position {}",
                                &thinker_id,
                                self.queue.len()
                            );
                        }
                    }
                    ForkStateInternal::Used {
                        thinker,
                        last_seen_at,
                    } => {
                        if thinker.id.eq(&thinker_id) {
                            *last_seen_at = Instant::now();
                            self.transceiver.send(
                                ThinkerMessage::ForkAlive {
                                    id: self.id.clone(),
                                    state: ForkState::Taken,
                                },
                                &thinker.address,
                            );
                        } else if let Some(queued) = self
                            .queue
                            .iter_mut()
                            .find(|queued_thinker| queued_thinker.thinker.id.eq(&thinker_id))
                        {
                            queued.last_seen_at = Instant::now();
                            self.transceiver.send(
                                ThinkerMessage::ForkAlive {
                                    id: self.id.clone(),
                                    state: ForkState::Queued,
                                },
                                &queued.thinker.address,
                            );
                        } else {
                            let queued_thinker = QueuedThinker {
                                last_seen_at: Instant::now(),
                                thinker: ThinkerRef {
                                    id: thinker_id.clone(),
                                    address: entity,
                                },
                            };
                            self.transceiver.send(
                                ThinkerMessage::ForkAlive {
                                    id: self.id.clone(),
                                    state: ForkState::Queued,
                                },
                                &queued_thinker.thinker.address,
                            );
                            self.queue.push_back(queued_thinker);
                            log::info!(
                                "Queued Thinker {} at position {}",
                                &thinker_id,
                                self.queue.len()
                            );
                        }
                    }
                };
            }
            ForkMessages::Release(id) => match &self.state {
                ForkStateInternal::Used { thinker, .. } if thinker.id.eq(&id) => {
                    log::info!("Fork released by {}", thinker.id);
                    self.state = ForkStateInternal::Unused;
                }
                ForkStateInternal::Used { .. } => {
                    log::error!(
                        "Got release from {} that currently doesnt hold the fork",
                        id
                    );
                }
                ForkStateInternal::Unused => {
                    log::error!("Got release message from {id}, but is currently not used");
                }
            },
            ForkMessages::Init(_) => {
                log::error!("Already initialized but got init message from {entity}");
            }
        }
    }

    pub fn update_state(&mut self) {
        match &self.state {
            ForkStateInternal::Unused => {
                if let Some(next) = self.queue.pop_front() {
                    self.state = ForkStateInternal::Used {
                        thinker: next.thinker.clone(),
                        last_seen_at: next.last_seen_at,
                    };
                    // self.transceiver.send(
                    //     ThinkerMessage::TakeForkAccepted(self.id.clone()),
                    //     &next.thinker.address,
                    // );
                    log::info!("Fork taken by {}", next.thinker.id);
                }
            }
            ForkStateInternal::Used {
                thinker,
                last_seen_at,
            } => {
                if last_seen_at.elapsed() > KEEP_ALIVE_TIMEOUT {
                    let thinker = thinker.clone();
                    log::warn!(
                        "No keep alive from thinker {}. Releasing fork access",
                        thinker.id
                    );
                    self.state = ForkStateInternal::Unused;
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
