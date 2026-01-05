use std::net::SocketAddr;
use std::time::Instant;

use rand::Rng;
use rand::rngs::ThreadRng;
use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::fork::ForkRef;
use crate::lib::messages::visualizer_messages::VisualizerThinkerState;
use crate::lib::messages::{ForkMessages, ThinkerMessage, VisualizerMessages};
use crate::lib::transceiver::Transceiver;
use crate::lib::utils::{EntityType, Id};
use crate::lib::visualizer::VisualizerRef;
use crate::{
    KEEP_ALIVE_TIMEOUT, MAX_EATING_TIME, MAX_THINKING_TIME, MIN_EATING_TIME, MIN_THINKING_TIME,
};

#[derive(Archive, Serialize, Deserialize, Clone, Debug)]
pub struct ThinkerRef {
    pub address: SocketAddr,
    pub id: Id<Thinker>,
}

#[derive(Debug, Clone)]
enum ForkState {
    Waiting,
    Taken,
}

#[derive(Debug, Clone)]
struct WaitingForForkState {
    state: ForkState,
    last_seen_at: Instant,
}

#[derive(Debug)]
enum HungryTokenState {
    WaitingForToken,
    TokenReceived,
}

#[derive(Debug)]
enum ThinkerState {
    Thinking {
        stop_thinking_at: Instant,
    },
    Hungry {
        token_state: HungryTokenState,
    },
    WaitingForForks([WaitingForForkState; 2]),
    Eating {
        stop_eating_at: Instant,
        fork_last_seen_at: [Instant; 2],
    },
}

impl From<&ThinkerState> for VisualizerThinkerState {
    fn from(value: &ThinkerState) -> Self {
        match value {
            ThinkerState::Thinking { .. } => VisualizerThinkerState::Thinking,
            ThinkerState::Hungry { .. } => VisualizerThinkerState::Hungry,
            ThinkerState::WaitingForForks { .. } => VisualizerThinkerState::WaitingForForks,
            ThinkerState::Eating { .. } => VisualizerThinkerState::Eating,
        }
    }
}

#[derive(Debug)]
pub struct ThinkerRefLastSeen {
    thinker: ThinkerRef,
    last_seen_at: Instant,
}

impl ThinkerRefLastSeen {
    pub fn is_timeouted(&self) -> bool {
        self.last_seen_at.elapsed() > KEEP_ALIVE_TIMEOUT
    }
}

#[derive(Debug)]
pub struct Thinker {
    id: Id<Thinker>,
    transceiver: Transceiver,
    state: ThinkerState,
    forks: [ForkRef; 2],
    next_thinkers: Vec<ThinkerRefLastSeen>,
    rng: ThreadRng,
    visualizer: Option<VisualizerRef>,
}
impl Thinker {
    pub fn new(
        id: Id<Thinker>,
        transceiver: Transceiver,
        unhandled_messages: Vec<(ThinkerMessage, SocketAddr)>,
        forks: [ForkRef; 2],
        next_thinkers: Vec<ThinkerRef>,
        has_token: bool,
        visualizer: Option<VisualizerRef>,
    ) -> Self {
        let mut rng = rand::rng();

        if has_token {
            transceiver.send(ThinkerMessage::Token, &next_thinkers[0].address);
        }
        let mut thinker = Self {
            id,
            transceiver,
            state: ThinkerState::Thinking {
                stop_thinking_at: Instant::now()
                    + rng.random_range(MIN_THINKING_TIME..=MAX_THINKING_TIME),
            },
            forks,
            next_thinkers: next_thinkers
                .into_iter()
                .map(|thinker| ThinkerRefLastSeen {
                    thinker,
                    last_seen_at: Instant::now(),
                })
                .collect(),
            rng,
            visualizer,
        };
        unhandled_messages
            .into_iter()
            .for_each(|(message, entity)| {
                thinker.handle_message(message, entity);
            });
        thinker
    }

    fn pass_token(&mut self) {
        if let Some(next_thinker) = &self
            .next_thinkers
            .iter()
            .find(|x| !x.is_timeouted())
            .map(|x| x.thinker.clone())
        {
            self.transceiver
                .send(ThinkerMessage::Token, &next_thinker.address);
            log::info!("Passed token to next alive thinker {}", next_thinker.id)
        } else {
            log::error!("All following thinkers are currently timed out. Dropping token.");
        }
    }

    pub fn handle_message(&mut self, message: ThinkerMessage, entity: SocketAddr) {
        match &message {
            ThinkerMessage::Init { .. } => {
                log::error!("Already initialized but got init message from {entity}");
            }
            ThinkerMessage::Token => {
                match &mut self.state {
                    ThinkerState::Thinking { .. }
                    | ThinkerState::WaitingForForks { .. }
                    | ThinkerState::Eating { .. } => {
                        // Token not needed at the moment, passing token to next node
                        self.pass_token();
                    }
                    ThinkerState::Hungry { token_state } => match token_state {
                        HungryTokenState::WaitingForToken => {
                            *token_state = HungryTokenState::TokenReceived
                        }
                        HungryTokenState::TokenReceived => {
                            // Token not needed at the moment, passing token to next node
                            self.pass_token();
                        }
                    },
                }
            }
            ThinkerMessage::TakeForkAccepted(id) => match &mut self.state {
                ThinkerState::WaitingForForks(wait_states) => {
                    let (entity, wait_state) = self
                        .forks
                        .iter()
                        .zip(wait_states)
                        .find(|(fork, _)| fork.id.eq(id))
                        .unwrap();
                    match wait_state.state {
                        ForkState::Waiting => {
                            *wait_state = WaitingForForkState {
                                state: ForkState::Taken,
                                last_seen_at: Instant::now(),
                            };
                            log::info!("Taken fork {}", entity.address);
                        }
                        ForkState::Taken => {
                            log::error!("Got fork, but i already own it");
                        }
                    }
                }
                ThinkerState::Thinking { .. }
                | ThinkerState::Eating { .. }
                | ThinkerState::Hungry { .. } => {
                    // This could happen if thinker node crashes restarts and afterwards gets
                    //  the response from the fork.
                }
            },
            ThinkerMessage::ForkAlive(id) => {
                match &mut self.state {
                    ThinkerState::Thinking { .. } | ThinkerState::Hungry { .. } => {
                        // Nothing to do here
                    }
                    ThinkerState::WaitingForForks(waiting_fork_state) => match waiting_fork_state
                        .iter_mut()
                        .zip(&self.forks)
                        .find(|(_, fork)| fork.id.eq(id))
                    {
                        Some((waiting_fork_state, _)) => {
                            waiting_fork_state.last_seen_at = Instant::now();
                        }
                        None => {
                            log::warn!("Got fork keep alive from unkown fork {}", id)
                        }
                    },
                    ThinkerState::Eating {
                        fork_last_seen_at, ..
                    } => {
                        match fork_last_seen_at
                            .iter_mut()
                            .zip(&self.forks)
                            .find(|(_, fork)| fork.id.eq(id))
                        {
                            Some((last, _)) => {
                                *last = Instant::now();
                            }
                            None => {
                                log::warn!("Got fork keep alive from unkown fork {}", id)
                            }
                        }
                    }
                };
            }
            ThinkerMessage::ThinkerAliveRequest(_) => {
                self.transceiver.send(
                    ThinkerMessage::ThinkerAliveResponse(self.id.clone()),
                    &entity,
                );
            }
            ThinkerMessage::ThinkerAliveResponse(id) => {
                if let Some(alive_thinker) = self
                    .next_thinkers
                    .iter_mut()
                    .find(|next_thinker| next_thinker.thinker.id.eq(id))
                {
                    alive_thinker.last_seen_at = Instant::now();
                } else {
                    log::warn!("Got keep alive response from unkown thinker {}", id);
                }
            }
        }
    }

    pub fn update_state(&mut self) {
        let mut alive_amount = 0;
        for next_thinker in self.next_thinkers.iter() {
            self.transceiver.send(
                ThinkerMessage::ThinkerAliveRequest(self.id.clone()),
                &next_thinker.thinker.address,
            );
            if !next_thinker.is_timeouted() {
                alive_amount += 1;
            }
            if alive_amount >= 2 {
                break;
            }
        }

        match &self.state {
            ThinkerState::Thinking { stop_thinking_at } => {
                match Instant::now().cmp(stop_thinking_at) {
                    std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => {
                        log::info!("Got hungry");
                        self.state = ThinkerState::Hungry {
                            token_state: HungryTokenState::WaitingForToken,
                        };
                    }
                    std::cmp::Ordering::Less => {
                        // Nothing to do here
                    }
                }
            }
            ThinkerState::Hungry { token_state } => {
                match token_state {
                    HungryTokenState::WaitingForToken => {
                        // Nothing to do here
                    }
                    HungryTokenState::TokenReceived => {
                        self.forks.iter().for_each(|fork| {
                            self.transceiver
                                .send(ForkMessages::Take(self.id.clone()), &fork.address);
                        });
                        self.state = ThinkerState::WaitingForForks(self.forks.clone().map(|_| {
                            WaitingForForkState {
                                state: ForkState::Waiting,
                                last_seen_at: Instant::now(),
                            }
                        }));
                        log::info!("Got token, requesting forks");
                    }
                }
                // Nothing to do here
            }
            ThinkerState::WaitingForForks(forks_state) => {
                let expired = forks_state.iter().any(|waiting_fork_state| {
                    waiting_fork_state.last_seen_at.elapsed() > KEEP_ALIVE_TIMEOUT
                });
                if expired {
                    self.forks.iter().for_each(|fork| {
                        self.transceiver
                            .send(ForkMessages::Release(self.id.clone()), &fork.address);
                    });
                    self.pass_token();
                    self.state = ThinkerState::Hungry {
                        token_state: HungryTokenState::WaitingForToken,
                    }
                } else {
                    self.forks.iter().for_each(|fork| {
                        self.transceiver
                            .send(ForkMessages::KeepAlive(self.id.clone()), &fork.address);
                    });
                    let all_taken = forks_state
                        .iter()
                        .all(|el| matches!(el.state, ForkState::Taken));

                    if all_taken {
                        self.state = ThinkerState::Eating {
                            stop_eating_at: Instant::now()
                                + self.rng.random_range(MIN_EATING_TIME..=MAX_EATING_TIME),
                            fork_last_seen_at: forks_state
                                .clone()
                                .map(|waiting_state| waiting_state.last_seen_at),
                        };
                        log::info!("Start eating");
                    }
                }
            }
            ThinkerState::Eating {
                stop_eating_at,
                fork_last_seen_at,
            } => {
                match Instant::now().cmp(stop_eating_at) {
                    std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => {
                        self.pass_token();
                        self.forks.iter().for_each(|fork| {
                            self.transceiver
                                .send(ForkMessages::Release(self.id.clone()), &fork.address)
                        });
                        self.state = ThinkerState::Thinking {
                            stop_thinking_at: Instant::now()
                                + self.rng.random_range(MIN_THINKING_TIME..=MAX_THINKING_TIME),
                        };
                        log::info!("Start Thinking, release forks");
                    }
                    std::cmp::Ordering::Less => {
                        self.forks.iter().for_each(|fork| {
                            self.transceiver
                                .send(ForkMessages::KeepAlive(self.id.clone()), &fork.address);
                        });
                        let expired = fork_last_seen_at
                            .iter()
                            .all(|at| at.elapsed() > KEEP_ALIVE_TIMEOUT);
                        if expired {
                            self.pass_token();
                            self.forks.iter().for_each(|fork| {
                                self.transceiver
                                    .send(ForkMessages::Release(self.id.clone()), &fork.address)
                            });
                            self.state = ThinkerState::Hungry {
                                token_state: HungryTokenState::WaitingForToken,
                            };
                        } else {
                            // Nothing to do here
                        }
                    }
                }
            }
        }
    }

    pub fn tick(&mut self, buffer: &mut [u8]) {
        while let Some((message, entity)) = self.transceiver.receive::<ThinkerMessage>(buffer) {
            self.handle_message(message, entity);
        }
        self.update_state();
    }

    pub fn update_visualizer(&self) {
        if let Some(visualizer) = &self.visualizer {
            self.transceiver.send(
                VisualizerMessages::ThinkerStateChanged {
                    id: self.id.clone(),
                    state: (&self.state).into(),
                },
                &visualizer.address,
            );
        }
    }
}

impl EntityType for Thinker {
    fn display_name() -> &'static str {
        "Thinker"
    }
}
