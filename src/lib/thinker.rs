use std::net::SocketAddr;
use std::time::{Duration, Instant, SystemTime};

use rand::Rng;
use rand::rngs::ThreadRng;
use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::fork::ForkRef;
use crate::lib::messages::{Epoch, ForkMessages, ReqId, ThinkerMessage, Token};
use crate::lib::messages::{VisualizerMessages, VisualizerThinkerState};
use crate::lib::transceiver::Transceiver;
use crate::lib::utils::{EntityType, Id};
use crate::lib::visualizer::VisualizerRef;
use crate::{MAX_EATING_TIME, MAX_THINKING_TIME, MIN_EATING_TIME, MIN_THINKING_TIME};

const RETRY_INTERVAL: Duration = Duration::from_millis(200);

const FORK_KEEPALIVE_INTERVAL: Duration = Duration::from_millis(500);

const TOKEN_TIMEOUT: Duration = Duration::from_secs(30);

const TOKEN_JITTER_MAX: Duration = Duration::from_millis(900);

#[derive(Archive, Serialize, Deserialize, Clone, Debug)]
pub struct ThinkerRef {
    pub address: SocketAddr,
    pub id: Id<Thinker>,
}

#[derive(Debug, Clone, Copy)]
enum WaitingForkState {
    Waiting,
    Taken,
}

#[derive(Debug, Clone, Copy)]
enum HungryTokenState {
    WaitingForToken,
    TokenReceived,
}

#[derive(Debug)]
enum ThinkerState {
    Thinking { stop_thinking_at: SystemTime },
    Hungry { token_state: HungryTokenState },
    WaitingForForks([WaitingForkState; 2]),
    Eating { stop_eating_at: SystemTime },
}

impl From<&ThinkerState> for VisualizerThinkerState {
    fn from(value: &ThinkerState) -> Self {
        match value {
            ThinkerState::Thinking { .. } => VisualizerThinkerState::Thinking,
            ThinkerState::Hungry { .. } => VisualizerThinkerState::Hungry,
            ThinkerState::WaitingForForks(_) => VisualizerThinkerState::WaitingForForks,
            ThinkerState::Eating { .. } => VisualizerThinkerState::Eating,
        }
    }
}

#[derive(Debug)]
pub struct Thinker {
    id: Id<Thinker>,
    epoch: Epoch,
    req_counter: u64,

    transceiver: Transceiver,
    state: ThinkerState,
    forks: [ForkRef; 2],
    next_thinker: ThinkerRef,
    rng: ThreadRng,
    visualizer: Option<VisualizerRef>,

    last_token_seen: Instant,
    best_token: Token,

    is_token_master: bool,

    fork_pending: [Option<(ReqId, Instant)>; 2],
    fork_granted: [Option<ReqId>; 2],
    last_keepalive: Instant,

    drop_token_once: bool,
    dropped_token_once: bool,
}

impl Thinker {
    fn token_rank(t: &Token) -> (u64, uuid::Uuid) {
        (t.seq, t.issuer.value)
    }

    fn token_is_better(a: &Token, b: &Token) -> bool {
        Self::token_rank(a) > Self::token_rank(b)
    }

    pub fn new(
        id: Id<Thinker>,
        transceiver: Transceiver,
        forks: [ForkRef; 2],
        next_thinker: ThinkerRef,
        owns_token: bool,
        visualizer: Option<VisualizerRef>,
    ) -> Self {
        let mut rng = rand::rng();
        let now = Instant::now();
        let epoch = Epoch(rand::random::<u64>());

        let drop_token_once = std::env::var("DROP_TOKEN_ONCE")
            .ok()
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(0)
            > 0;

        let mut best_token = Token {
            seq: 0,
            issuer: id.clone(),
        };

        if owns_token {
            best_token = Token {
                seq: 1,
                issuer: id.clone(),
            };
            transceiver.send(
                ThinkerMessage::Token(best_token.clone()),
                &next_thinker.address,
            );
        }
        Self {
            id,
            epoch,
            req_counter: 1,

            transceiver,
            state: ThinkerState::Thinking {
                stop_thinking_at: SystemTime::now()
                    + rng.random_range(MIN_THINKING_TIME..=MAX_THINKING_TIME),
            },
            forks,
            next_thinker,
            rng,

            last_token_seen: now,
            best_token,

            is_token_master: owns_token,

            fork_pending: [None, None],
            fork_granted: [None, None],
            last_keepalive: now,

            drop_token_once,
            dropped_token_once: false,
            visualizer,
        }
    }

    fn handle_message(&mut self, message: ThinkerMessage, entity: SocketAddr) {
        match message {
            ThinkerMessage::Init(_) => {
                log::error!("Already initialized but got init message from {entity}");
            }

            ThinkerMessage::Token(token) => {
                if self.drop_token_once && !self.dropped_token_once {
                    self.dropped_token_once = true;
                    log::warn!("Dropped token ONCE seq={}", token.seq);
                    return;
                }

                let drop_pct: u8 = std::env::var("DROP_TOKEN_PCT")
                    .ok()
                    .and_then(|v| v.parse::<u8>().ok())
                    .unwrap_or(0)
                    .min(100);

                if drop_pct > 0 && self.rng.random_range(0..100) < drop_pct {
                    log::warn!(
                        "Dropped token seq={} (DROP_TOKEN_PCT={})",
                        token.seq,
                        drop_pct
                    );
                    return;
                }

                let now = Instant::now();
                self.last_token_seen = now;

                if Self::token_is_better(&self.best_token, &token) {
                    return;
                }
                if Self::token_is_better(&token, &self.best_token) {
                    self.best_token = token.clone();
                }

                match &mut self.state {
                    ThinkerState::Thinking { .. }
                    | ThinkerState::WaitingForForks(_)
                    | ThinkerState::Eating { .. } => {
                        self.transceiver.send(
                            ThinkerMessage::Token(self.best_token.clone()),
                            &self.next_thinker.address,
                        );
                    }

                    ThinkerState::Hungry { token_state } => match token_state {
                        HungryTokenState::WaitingForToken => {
                            *token_state = HungryTokenState::TokenReceived;
                        }
                        HungryTokenState::TokenReceived => {
                            // Token not needed at the moment, passing token to next node
                            self.transceiver.send(
                                ThinkerMessage::Token(self.best_token.clone()),
                                &self.next_thinker.address,
                            );
                        }
                    },
                }
            }

            ThinkerMessage::TakeForkAccepted { fork, epoch, req } => {
                if epoch != self.epoch {
                    return;
                }

                let idx = match self.forks.iter().position(|f| f.id.eq(&fork)) {
                    Some(i) => i,
                    None => return,
                };

                let pending_ok = self.fork_pending[idx]
                    .map(|(r, _)| r == req)
                    .unwrap_or(false);

                if !pending_ok {
                    return;
                }

                self.fork_pending[idx] = None;
                self.fork_granted[idx] = Some(req);

                if let ThinkerState::WaitingForForks(forks_state) = &mut self.state {
                    if matches!(forks_state[idx], WaitingForkState::Waiting) {
                        forks_state[idx] = WaitingForkState::Taken;
                        log::info!("Taken fork {}", self.forks[idx].address);
                    }
                }
            }
        }
    }

    fn update_state(&mut self) {
        match &mut self.state {
            ThinkerState::Thinking { stop_thinking_at } => {
                if SystemTime::now() >= *stop_thinking_at {
                    self.state = ThinkerState::Hungry {
                        token_state: HungryTokenState::WaitingForToken,
                    };
                    log::info!("Got hungry");
                }
            }

            ThinkerState::Hungry { token_state } => match token_state {
                HungryTokenState::WaitingForToken => {}
                HungryTokenState::TokenReceived => {
                    for i in 0..2 {
                        let req = ReqId(self.req_counter);
                        self.req_counter += 1;

                        self.fork_pending[i] = Some((req, Instant::now()));
                        self.transceiver.send(
                            ForkMessages::Take {
                                thinker: self.id.clone(),
                                epoch: self.epoch,
                                req,
                            },
                            &self.forks[i].address,
                        );
                    }

                    self.state = ThinkerState::WaitingForForks([WaitingForkState::Waiting; 2]);
                    log::info!("Got token, requesting forks");
                }
            },

            ThinkerState::WaitingForForks(forks_state) => {
                for i in 0..2 {
                    if let Some((req, last_sent)) = self.fork_pending[i] {
                        if Instant::now().duration_since(last_sent) > RETRY_INTERVAL {
                            self.fork_pending[i] = Some((req, Instant::now()));
                            self.transceiver.send(
                                ForkMessages::Take {
                                    thinker: self.id.clone(),
                                    epoch: self.epoch,
                                    req,
                                },
                                &self.forks[i].address,
                            );
                        }
                    }
                }

                if forks_state
                    .iter()
                    .all(|s| matches!(s, WaitingForkState::Taken))
                {
                    self.state = ThinkerState::Eating {
                        stop_eating_at: SystemTime::now()
                            + self.rng.random_range(MIN_EATING_TIME..=MAX_EATING_TIME),
                    };
                    log::info!("Start eating");
                }
            }

            ThinkerState::Eating { stop_eating_at } => {
                if SystemTime::now() >= *stop_eating_at {
                    self.transceiver.send(
                        ThinkerMessage::Token(self.best_token.clone()),
                        &self.next_thinker.address,
                    );

                    for i in 0..2 {
                        if let Some(req) = self.fork_granted[i] {
                            self.transceiver.send(
                                ForkMessages::Release {
                                    thinker: self.id.clone(),
                                    epoch: self.epoch,
                                    req,
                                },
                                &self.forks[i].address,
                            );
                            self.fork_granted[i] = None;
                        }
                    }

                    self.state = ThinkerState::Thinking {
                        stop_thinking_at: SystemTime::now()
                            + self.rng.random_range(MIN_THINKING_TIME..=MAX_THINKING_TIME),
                    };

                    log::info!(
                        "Start Thinking, transfer token to {}, release forks",
                        self.next_thinker.address
                    );
                }
            }
        }
    }

    pub fn tick(&mut self, buffer: &mut [u8]) {
        while let Some((message, entity)) = self.transceiver.receive::<ThinkerMessage>(buffer) {
            self.handle_message(message, entity);
        }

        self.update_state();

        if Instant::now().duration_since(self.last_keepalive) > FORK_KEEPALIVE_INTERVAL {
            for i in 0..2 {
                if self.fork_granted[i].is_some() {
                    self.transceiver.send(
                        ForkMessages::KeepAlive {
                            thinker: self.id.clone(),
                            epoch: self.epoch,
                        },
                        &self.forks[i].address,
                    );
                }
            }
            self.last_keepalive = Instant::now();
        }

        let waiting_for_token = matches!(
            self.state,
            ThinkerState::Hungry {
                token_state: HungryTokenState::WaitingForToken
            }
        );

        if self.is_token_master && waiting_for_token {
            let jitter_ms =
                (self.id.value.as_u128() % (TOKEN_JITTER_MAX.as_millis() as u128 + 1)) as u64;
            let jitter = Duration::from_millis(jitter_ms);

            if Instant::now().duration_since(self.last_token_seen) > TOKEN_TIMEOUT + jitter {
                let new_token = Token {
                    seq: self.best_token.seq + 1,
                    issuer: self.id.clone(),
                };
                self.best_token = new_token;
                self.last_token_seen = Instant::now();

                if let ThinkerState::Hungry { token_state } = &mut self.state {
                    *token_state = HungryTokenState::TokenReceived;
                }

                log::warn!(
                    "Token timeout (master + hungry) -> regenerated token seq={}",
                    self.best_token.seq
                );
            }
        }
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
