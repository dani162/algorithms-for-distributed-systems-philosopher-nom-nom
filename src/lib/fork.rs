use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::Instant;

use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::messages::{Epoch, ForkMessages, ReqId, ThinkerMessage};
use crate::lib::thinker::Thinker;
use crate::lib::transceiver::Transceiver;
use crate::lib::utils::{EntityType, Id};

#[derive(Archive, Serialize, Deserialize, Clone, Debug)]
pub struct ForkRef {
    pub address: SocketAddr,
    pub id: Id<Fork>,
}

#[derive(Debug, Clone)]
struct ForkRequest {
    addr: SocketAddr,
    thinker: Id<Thinker>,
    epoch: Epoch,
    req: ReqId,
}

#[derive(Debug, Clone)]
struct Owner {
    addr: SocketAddr,
    thinker: Id<Thinker>,
    epoch: Epoch,
    req: ReqId,
    lease_until: Instant,
}

#[derive(Debug)]
enum ForkState {
    Unused,
    Used(Owner),
}

#[derive(Debug)]
pub struct Fork {
    pub id: Id<Fork>,
    state: ForkState,
    queue: VecDeque<ForkRequest>,
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

    fn grant_next_if_any(&mut self) {
        if let Some(next) = self.queue.pop_front() {
            let now = Instant::now();
            self.state = ForkState::Used(Owner {
                addr: next.addr,
                thinker: next.thinker.clone(),
                epoch: next.epoch,
                req: next.req,
                lease_until: now + crate::FORK_LEASE,
            });

            self.transceiver.send(
                ThinkerMessage::TakeForkAccepted {
                    fork: self.id.clone(),
                    epoch: next.epoch,
                    req: next.req,
                },
                &next.addr,
            );

            log::info!("Fork granted to {}", next.addr);
        }
    }

    pub fn tick(&mut self, buffer: &mut [u8]) {
        let now = Instant::now();
        if let ForkState::Used(owner) = &self.state {
            if now >= owner.lease_until {
                log::warn!("Fork lease expired for owner {}, freeing fork", owner.addr);
                self.state = ForkState::Unused;
                self.grant_next_if_any();
            }
        }

        while let Some((message, entity)) = self.transceiver.receive::<ForkMessages>(buffer) {
            match message {
                ForkMessages::Take {
                    thinker,
                    epoch,
                    req,
                } => match &self.state {
                    ForkState::Unused => {
                        self.state = ForkState::Used(Owner {
                            addr: entity,
                            thinker: thinker.clone(),
                            epoch,
                            req,
                            lease_until: Instant::now() + crate::FORK_LEASE,
                        });

                        self.transceiver.send(
                            ThinkerMessage::TakeForkAccepted {
                                fork: self.id.clone(),
                                epoch,
                                req,
                            },
                            &entity,
                        );

                        log::info!("Fork taken by {entity}");
                    }

                    ForkState::Used(owner) => {
                        if owner.addr == entity
                            && owner.thinker.eq(&thinker)
                            && owner.epoch == epoch
                            && owner.req == req
                        {
                            self.transceiver.send(
                                ThinkerMessage::TakeForkAccepted {
                                    fork: self.id.clone(),
                                    epoch,
                                    req,
                                },
                                &entity,
                            );
                        } else {
                            let already_queued = self.queue.iter().any(|r| {
                                r.addr == entity
                                    && r.thinker.eq(&thinker)
                                    && r.epoch == epoch
                                    && r.req == req
                            });

                            if !already_queued {
                                self.queue.push_back(ForkRequest {
                                    addr: entity,
                                    thinker,
                                    epoch,
                                    req,
                                });
                                log::info!(
                                    "Queued Thinker {entity} at position {}",
                                    self.queue.len()
                                );
                            }
                        }
                    }
                },

                ForkMessages::KeepAlive { thinker, epoch } => {
                    if let ForkState::Used(owner) = &mut self.state {
                        if owner.thinker.eq(&thinker) && owner.epoch == epoch {
                            owner.lease_until = Instant::now() + crate::FORK_LEASE;
                        }
                    }
                }

                ForkMessages::Release {
                    thinker,
                    epoch,
                    req,
                } => match &self.state {
                    ForkState::Unused => {
                        log::warn!("Got Release from {entity}, but fork is unused");
                    }
                    ForkState::Used(owner) => {
                        let ok = owner.addr == entity
                            && owner.thinker.eq(&thinker)
                            && owner.epoch == epoch
                            && owner.req == req;

                        if ok {
                            self.state = ForkState::Unused;
                            log::info!("Fork released by {entity}");
                            self.grant_next_if_any();
                        } else {
                            log::warn!("Ignoring invalid Release from {entity}");
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
