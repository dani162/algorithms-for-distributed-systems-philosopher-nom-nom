use std::net::SocketAddr;
use std::time::Instant;

use rand::Rng;
use rand::rngs::ThreadRng;
use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::fork::ForkRef;
use crate::lib::messages::thinker_messages::{Token, TokenRef};
use crate::lib::messages::visualizer_messages::VisualizerThinkerState;
use crate::lib::messages::{ForkMessages, ThinkerMessage, VisualizerMessages};
use crate::lib::thinker;
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
    TokenReceived(Token),
}

#[derive(Debug)]
enum ThinkerState {
    Thinking {
        stop_thinking_at: Instant,
    },
    Hungry {
        token_state: HungryTokenState,
    },
    WaitingForForks {
        token: Token,
        waiting_state: [WaitingForForkState; 2],
    },
    Eating {
        token: Token,
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
struct ThinkerRefLastSeen {
    thinker: ThinkerRef,
    last_seen_at: Instant,
}

impl ThinkerRefLastSeen {
    fn is_timeouted(&self) -> bool {
        self.last_seen_at.elapsed() > KEEP_ALIVE_TIMEOUT
    }
}

#[derive(Debug)]
struct TokenRefLastSeen {
    token_ref: TokenRef,
    last_seen_at: Instant,
}

impl TokenRefLastSeen {
    fn _is_timeouted(&self) -> bool {
        self.last_seen_at.elapsed() > KEEP_ALIVE_TIMEOUT
    }
}

pub struct ThinkerInitParams {
    pub id: Id<Thinker>,
    pub transceiver: Transceiver,
    pub unhandled_messages: Vec<(ThinkerMessage, SocketAddr)>,
    pub forks: [ForkRef; 2],
    pub next_thinkers: Vec<ThinkerRef>,
    pub token: Option<Token>,
    pub available_tokens: Vec<TokenRef>,
    pub visualizer: Option<VisualizerRef>,
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
    available_tokens: Vec<TokenRefLastSeen>,
}
impl Thinker {
    pub fn new(init_params: ThinkerInitParams) -> Self {
        let mut rng = rand::rng();

        if let Some(token) = init_params.token {
            init_params.transceiver.send(
                ThinkerMessage::Token(token),
                &init_params.next_thinkers[0].address,
            );
        }
        let mut thinker = Self {
            id: init_params.id,
            transceiver: init_params.transceiver,
            state: ThinkerState::Thinking {
                stop_thinking_at: Instant::now()
                    + rng.random_range(MIN_THINKING_TIME..=MAX_THINKING_TIME),
            },
            forks: init_params.forks,
            next_thinkers: init_params
                .next_thinkers
                .into_iter()
                .map(|thinker| ThinkerRefLastSeen {
                    thinker,
                    last_seen_at: Instant::now(),
                })
                .collect(),
            rng,
            visualizer: init_params.visualizer,
            available_tokens: init_params
                .available_tokens
                .into_iter()
                .map(|token_ref| TokenRefLastSeen {
                    token_ref,
                    last_seen_at: Instant::now(),
                })
                .collect(),
        };
        init_params
            .unhandled_messages
            .into_iter()
            .for_each(|(message, entity)| {
                thinker.handle_message(message, entity);
            });
        thinker
    }

    pub fn print_started(&self) {
        log::info!(
            "Started Thinker {} {}",
            self.transceiver.local_address(),
            self.id,
        )
    }

    fn token_broadcast(&self, token_ref: TokenRef, broadcast_issuer: Id<Thinker>) {
        for next_thinker in &self.next_thinkers {
            if next_thinker.thinker.id.eq(&broadcast_issuer) {
                return;
            }
            if next_thinker.is_timeouted() {
                continue;
            }
            self.transceiver.send(
                ThinkerMessage::TokenAliveBroadcast {
                    token_ref,
                    broadcast_issuer,
                },
                &next_thinker.thinker.address,
            );
            return;
        }
        log::error!(
            "All following thinkers are currently timed out. Dropping token alive broadcast."
        )
    }

    fn pass_token(&self, token: Token) {
        if let Some(next_thinker) = &self
            .next_thinkers
            .iter()
            .find(|x| !x.is_timeouted())
            .map(|x| x.thinker.clone())
        {
            self.transceiver
                .send(ThinkerMessage::Token(token), &next_thinker.address);
            log::info!("Passed token to next alive thinker {}", next_thinker.id)
        } else {
            log::error!("All following thinkers are currently timed out. Dropping token.");
        }
    }

    pub fn handle_message(&mut self, message: ThinkerMessage, entity: SocketAddr) {
        match message {
            ThinkerMessage::Init { .. } => {
                log::error!("Already initialized but got init message from {entity}");
            }
            ThinkerMessage::Token(token) => {
                match &mut self.state {
                    ThinkerState::Thinking { .. }
                    | ThinkerState::WaitingForForks { .. }
                    | ThinkerState::Eating { .. } => {
                        // Token not needed at the moment, passing token to next node
                        self.pass_token(token);
                    }
                    ThinkerState::Hungry { token_state } => match token_state {
                        HungryTokenState::WaitingForToken => {
                            *token_state = HungryTokenState::TokenReceived(token)
                        }
                        HungryTokenState::TokenReceived(_) => {
                            // Token not needed at the moment, passing token to next node
                            self.pass_token(token);
                        }
                    },
                }
            }
            ThinkerMessage::TakeForkAccepted(id) => match &mut self.state {
                ThinkerState::WaitingForForks { waiting_state, .. } => {
                    let (entity, wait_state) = self
                        .forks
                        .iter()
                        .zip(waiting_state)
                        .find(|(fork, _)| fork.id.eq(&id))
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
                    ThinkerState::WaitingForForks { waiting_state, .. } => match waiting_state
                        .iter_mut()
                        .zip(&self.forks)
                        .find(|(_, fork)| fork.id.eq(&id))
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
                            .find(|(_, fork)| fork.id.eq(&id))
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
                    .find(|next_thinker| next_thinker.thinker.id.eq(&id))
                {
                    alive_thinker.last_seen_at = Instant::now();
                } else {
                    log::warn!("Got keep alive response from unkown thinker {}", id);
                }
            }
            ThinkerMessage::ProposeToken { .. } => todo!(),
            ThinkerMessage::TokenAliveBroadcast {
                token_ref,
                broadcast_issuer,
            } => {
                if let Some(available_token) = self
                    .available_tokens
                    .iter_mut()
                    .find(|el| el.token_ref.id.eq(&token_ref.id))
                {
                    available_token.last_seen_at = Instant::now();
                    self.token_broadcast(token_ref, broadcast_issuer);
                } else {
                    log::warn!(
                        "Token alive broadcast message from unkown token {:?}",
                        token_ref
                    )
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
                    HungryTokenState::TokenReceived(token) => {
                        self.token_broadcast(token.into(), self.id.clone());
                        self.forks.iter().for_each(|fork| {
                            self.transceiver
                                .send(ForkMessages::Take(self.id.clone()), &fork.address);
                        });
                        self.state = ThinkerState::WaitingForForks {
                            waiting_state: self.forks.clone().map(|_| WaitingForForkState {
                                state: ForkState::Waiting,
                                last_seen_at: Instant::now(),
                            }),
                            token: token.clone(),
                        };
                        log::info!("Got token, requesting forks");
                    }
                }
                // Nothing to do here
            }
            ThinkerState::WaitingForForks {
                waiting_state,
                token,
            } => {
                let expired = waiting_state.iter().any(|waiting_fork_state| {
                    waiting_fork_state.last_seen_at.elapsed() > KEEP_ALIVE_TIMEOUT
                });
                if expired {
                    self.forks.iter().for_each(|fork| {
                        self.transceiver
                            .send(ForkMessages::Release(self.id.clone()), &fork.address);
                    });
                    self.pass_token(token.clone());
                    self.state = ThinkerState::Hungry {
                        token_state: HungryTokenState::WaitingForToken,
                    }
                } else {
                    self.forks.iter().for_each(|fork| {
                        self.transceiver
                            .send(ForkMessages::KeepAlive(self.id.clone()), &fork.address);
                    });
                    self.token_broadcast(token.into(), self.id.clone());
                    let all_taken = waiting_state
                        .iter()
                        .all(|el| matches!(el.state, ForkState::Taken));

                    if all_taken {
                        self.state = ThinkerState::Eating {
                            stop_eating_at: Instant::now()
                                + self.rng.random_range(MIN_EATING_TIME..=MAX_EATING_TIME),
                            fork_last_seen_at: waiting_state
                                .clone()
                                .map(|waiting_state| waiting_state.last_seen_at),
                            token: token.clone(),
                        };
                        log::info!("Start eating");
                    }
                }
            }
            ThinkerState::Eating {
                stop_eating_at,
                fork_last_seen_at,
                token,
            } => match Instant::now().cmp(stop_eating_at) {
                std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => {
                    self.pass_token(token.clone());
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
                        self.pass_token(token.clone());
                        self.forks.iter().for_each(|fork| {
                            self.transceiver
                                .send(ForkMessages::Release(self.id.clone()), &fork.address)
                        });
                        self.state = ThinkerState::Hungry {
                            token_state: HungryTokenState::WaitingForToken,
                        };
                    } else {
                        self.token_broadcast(token.into(), self.id.clone());
                    }
                }
            },
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
