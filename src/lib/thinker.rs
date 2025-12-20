use std::net::SocketAddr;
use std::time::SystemTime;

use rand::Rng;
use rand::rngs::ThreadRng;
use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::fork::ForkRef;
use crate::lib::messages::{ForkMessages, ThinkerMessage};
use crate::lib::transceiver::Transceiver;
use crate::lib::utils::{EntityType, Id};
use crate::{MAX_EATING_TIME, MAX_THINKING_TIME, MIN_THINKING_TIME};

#[derive(Archive, Serialize, Deserialize, Clone)]
pub struct ThinkerRef {
    pub address: SocketAddr,
    pub id: Id<Thinker>,
}

enum WaitingForkState {
    Waiting,
    Taken,
}

enum HungryTokenState {
    WaitingForToken,
    TokenReceived,
}

enum ThinkerState {
    Thinking { stop_thinking_at: SystemTime },
    Hungry { token_state: HungryTokenState },
    WaitingForForks([WaitingForkState; 2]),
    Eating { stop_eating_at: SystemTime },
}

pub struct Thinker {
    _id: Id<Thinker>,
    transceiver: Transceiver,
    state: ThinkerState,
    forks: [ForkRef; 2],
    next_thinker: ThinkerRef,
    rng: ThreadRng,
}
impl Thinker {
    pub fn new(
        id: Id<Thinker>,
        transceiver: Transceiver,
        unhandled_messages: Vec<(ThinkerMessage, SocketAddr)>,
        forks: [ForkRef; 2],
        next_thinker: ThinkerRef,
    ) -> Self {
        let mut rng = rand::rng();
        let mut thinker = Self {
            _id: id,
            transceiver,
            state: ThinkerState::Thinking {
                stop_thinking_at: SystemTime::now()
                    + rng.random_range(MIN_THINKING_TIME..=MAX_EATING_TIME),
            },
            forks,
            next_thinker,
            rng,
        };
        unhandled_messages
            .into_iter()
            .for_each(|(message, entity)| {
                thinker.handle_message(message, entity);
            });
        thinker
    }

    pub fn handle_message(&mut self, message: ThinkerMessage, entity: SocketAddr) {
        match message {
            ThinkerMessage::Init(_) => {
                log::error!("Already initialized but got init message from {entity}");
            }
            ThinkerMessage::Token => match &mut self.state {
                ThinkerState::Thinking { .. }
                | ThinkerState::WaitingForForks(_)
                | ThinkerState::Eating { .. } => {
                    // Token not needed at the moment, passing token to next node
                    self.transceiver
                        .send(ThinkerMessage::Token, &self.next_thinker.address);
                }
                ThinkerState::Hungry { token_state } => match token_state {
                    HungryTokenState::WaitingForToken => {
                        *token_state = HungryTokenState::TokenReceived
                    }
                    HungryTokenState::TokenReceived => {
                        // Token not needed at the moment, passing token to next node
                        self.transceiver
                            .send(ThinkerMessage::Token, &self.next_thinker.address);
                    }
                },
            },
            ThinkerMessage::TakeForkAccepted(id) => match &mut self.state {
                ThinkerState::WaitingForForks(forks_state) => {
                    let (entity, waiting_fork_state) = self
                        .forks
                        .iter()
                        .zip(&mut *forks_state)
                        .find(|(fork, _)| fork.id.eq(&id))
                        .unwrap();
                    match waiting_fork_state {
                        WaitingForkState::Waiting => {
                            *waiting_fork_state = WaitingForkState::Taken;
                            log::info!("Taken fork {}", entity.address);
                        }
                        WaitingForkState::Taken => {
                            panic!("Got fork, but already own it at the moment")
                        }
                    }
                }
                ThinkerState::Thinking { .. }
                | ThinkerState::Eating { .. }
                | ThinkerState::Hungry { .. } => {
                    // TODO: This could happen if thinker node crashes restarts and afterwards gets
                    //  the response from the fork. This should be handled with proper error
                    //  handling. Maybe just tell the fork to release instantly.
                    panic!("Unescpected token accpeted message");
                }
            },
        }
    }

    pub fn update_state(&mut self) {
        match &self.state {
            ThinkerState::Thinking { stop_thinking_at } => {
                match SystemTime::now().cmp(stop_thinking_at) {
                    std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => {
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
                            self.transceiver.send(ForkMessages::Take, &fork.address);
                        });
                        self.state = ThinkerState::WaitingForForks(
                            self.forks.clone().map(|_| WaitingForkState::Waiting),
                        );
                        log::info!("Got token, requesting forks");
                    }
                }
                // Nothing to do here
            }
            ThinkerState::WaitingForForks(forks_state) => {
                if forks_state
                    .iter()
                    .all(|state| matches!(state, WaitingForkState::Taken))
                {
                    self.state = ThinkerState::Eating {
                        stop_eating_at: SystemTime::now()
                            + self.rng.random_range(MIN_THINKING_TIME..=MAX_THINKING_TIME),
                    };
                    log::info!("Start eating");
                }
            }
            ThinkerState::Eating { stop_eating_at } => {
                match SystemTime::now().cmp(stop_eating_at) {
                    std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => {
                        self.transceiver
                            .send(ThinkerMessage::Token, &self.next_thinker.address);
                        self.state = ThinkerState::Thinking {
                            stop_thinking_at: SystemTime::now()
                                + self.rng.random_range(MIN_THINKING_TIME..=MAX_THINKING_TIME),
                        }
                    }
                    std::cmp::Ordering::Less => {
                        // Nothing to do here
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
}

impl EntityType for Thinker {
    fn display_name() -> &'static str {
        "Thinker"
    }
}
