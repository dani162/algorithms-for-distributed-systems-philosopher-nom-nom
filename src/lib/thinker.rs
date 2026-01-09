use std::net::SocketAddr;
use std::time::Instant;

use rand::Rng;
use rand::rngs::ThreadRng;
use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::fork::ForkRef;
use crate::lib::messages::thinker_messages::{
    ForkState, Token, TokenPriority, TokenProposal, TokenRef,
};
use crate::lib::messages::visualizer_messages::{
    VisualizerThinkerAvailableTokenState, VisualizerThinkerState,
};
use crate::lib::messages::{ForkMessages, ThinkerMessage, VisualizerMessages};
use crate::lib::transceiver::Transceiver;
use crate::lib::utils::{EntityType, Id};
use crate::lib::visualizer::VisualizerRef;
use crate::{
    KEEP_ALIVE_TIMEOUT, MAX_EATING_TIME, MAX_THINKING_TIME, MIN_EATING_TIME, MIN_THINKING_TIME,
    TOKEN_TIMEOUT,
};

#[derive(Archive, Serialize, Deserialize, Clone, Debug)]
pub struct ThinkerRef {
    pub address: SocketAddr,
    pub id: Id<Thinker>,
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
            ThinkerState::WaitingForForks { token, .. } => {
                VisualizerThinkerState::WaitingForForks {
                    token: TokenRef::from(token),
                }
            }
            ThinkerState::Eating { token, .. } => VisualizerThinkerState::Eating {
                token: TokenRef::from(token),
            },
        }
    }
}

#[derive(Debug)]
struct ThinkerRefLastSeen {
    thinker: ThinkerRef,
    last_seen_at: Instant,
}

impl ThinkerRefLastSeen {
    fn is_timed_out(&self) -> bool {
        self.last_seen_at.elapsed() > KEEP_ALIVE_TIMEOUT
    }
}

#[derive(Debug)]
enum TokenRefLastSeenState {
    Passive,
    Propose(TokenProposal),
}

#[derive(Debug)]
struct TokenRefLastSeen {
    current_token_ref: TokenRef,
    current_proposal_version: u32,
    last_seen_at: Instant,
    state: TokenRefLastSeenState,
}

impl From<&TokenRefLastSeen> for VisualizerThinkerAvailableTokenState {
    fn from(value: &TokenRefLastSeen) -> Self {
        match &value.state {
            TokenRefLastSeenState::Passive => Self::Passive {
                not_seen_for: value.last_seen_at.elapsed(),
            },
            TokenRefLastSeenState::Propose(token_proposal) => Self::Propose {
                token_version: token_proposal.proposed_token.version,
                propose_version: token_proposal.propose_version,
            },
        }
    }
}

impl TokenRefLastSeen {
    fn is_timed_out(&self) -> bool {
        self.last_seen_at.elapsed() > TOKEN_TIMEOUT
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
                    current_token_ref: token_ref,
                    last_seen_at: Instant::now(),
                    state: TokenRefLastSeenState::Passive,
                    current_proposal_version: 0,
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
        // &self.mark_token_as_seen(&token_ref);
        for next_thinker in &self.next_thinkers {
            if next_thinker.thinker.id.eq(&broadcast_issuer) {
                return;
            }
            if next_thinker.is_timed_out() {
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
            .find(|x| !x.is_timed_out())
            .map(|x| x.thinker.clone())
        {
            self.transceiver
                .send(ThinkerMessage::Token(token), &next_thinker.address);
            // log::info!("Passed token to next alive thinker {}", next_thinker.id)
        } else {
            log::error!("All following thinkers are currently timed out. Dropping token.");
        }
    }

    fn pass_token_proposal(&self, token_proposal: TokenProposal) {
        let issuer = &token_proposal.proposed_token.issuer;
        for next_thinker in &self.next_thinkers {
            if next_thinker.is_timed_out() {
                if next_thinker.thinker.id.eq(issuer) {
                    break;
                } else {
                    continue;
                }
            }
            self.transceiver.send(
                ThinkerMessage::ProposeToken(token_proposal),
                &next_thinker.thinker.address,
            );
            break;
        }
    }

    /// returns true if passed token is still uptodate
    fn mark_token_as_seen(&mut self, token_ref: &TokenRef) -> bool {
        let last_seen = self
            .available_tokens
            .iter_mut()
            .find(|last_seen| last_seen.current_token_ref.id.eq(&token_ref.id))
            .unwrap();

        match token_ref.priority(&last_seen.current_token_ref).unwrap() {
            TokenPriority::High | TokenPriority::Equal => {
                last_seen.current_token_ref = token_ref.clone();
                last_seen.last_seen_at = Instant::now();
                true
            }
            TokenPriority::Low => {
                // Got outdated token, do nothing
                false
            }
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
                        if self.mark_token_as_seen(&TokenRef::from(&token)) {
                            self.pass_token(token);
                        } else {
                            log::warn!("Dropping outdated token {:?}", token);
                        }
                    }
                    ThinkerState::Hungry { token_state } => match token_state {
                        HungryTokenState::WaitingForToken => {
                            *token_state = HungryTokenState::TokenReceived(token)
                        }
                        HungryTokenState::TokenReceived(_) => {
                            // Token not needed at the moment, passing token to next node
                            if self.mark_token_as_seen(&TokenRef::from(&token)) {
                                self.pass_token(token);
                            } else {
                                log::warn!("Dropping outdated token {:?}", token);
                            }
                        }
                    },
                }
            }
            ThinkerMessage::ForkAlive {
                id: fork_id,
                state: new_fork_state,
            } => {
                match &mut self.state {
                    ThinkerState::Thinking { .. } | ThinkerState::Hungry { .. } => {
                        // Nothing to do here
                    }
                    ThinkerState::WaitingForForks { waiting_state, .. } => {
                        if let Some(own_fork_state) = waiting_state
                            .iter_mut()
                            .zip(&self.forks)
                            .find(|(_, fork)| fork.id.eq(&fork_id))
                            .map(|(fork_state, _)| fork_state)
                        {
                            match new_fork_state {
                                ForkState::Taken => {
                                    if matches!(own_fork_state.state, ForkState::Queued) {
                                        log::info!("Taken fork {}", fork_id);
                                        own_fork_state.state = ForkState::Taken;
                                    }
                                }
                                ForkState::Queued => (),
                            }
                            own_fork_state.last_seen_at = Instant::now()
                        } else {
                            log::warn!("Got fork keep alive from unkown fork {}", fork_id)
                        }
                    }
                    ThinkerState::Eating {
                        fork_last_seen_at, ..
                    } => {
                        match fork_last_seen_at
                            .iter_mut()
                            .zip(&self.forks)
                            .find(|(_, fork)| fork.id.eq(&fork_id))
                        {
                            Some((last, _)) => {
                                *last = Instant::now();
                            }
                            None => {
                                log::warn!("Got fork keep alive from unkown fork {}", fork_id)
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
            ThinkerMessage::ProposeToken(proposal) => {
                if let Some(last_seen_token) = self
                    .available_tokens
                    .iter_mut()
                    .find(|token| token.current_token_ref.id.eq(&proposal.proposed_token.id))
                {
                    match proposal
                        .proposed_token
                        .priority(&last_seen_token.current_token_ref)
                        .unwrap()
                    {
                        TokenPriority::Low | TokenPriority::Equal => {
                            log::warn!(
                                "Outdated proposal {:?}, current: {:?}",
                                proposal,
                                last_seen_token.current_token_ref
                            );
                            // Proposal outdated, do nothing
                        }
                        TokenPriority::High => match &last_seen_token.state {
                            TokenRefLastSeenState::Passive => {
                                if proposal.proposed_token.issuer.ne(&self.id) {
                                    last_seen_token.last_seen_at = Instant::now();
                                    self.pass_token_proposal(proposal);
                                } else {
                                    // No longer in proposing state, do nothing
                                }
                            }
                            TokenRefLastSeenState::Propose(own_proposal) => {
                                let is_own_proposal = proposal.proposed_token.issuer.eq(&self.id);
                                if !is_own_proposal {
                                    match proposal
                                        .proposed_token
                                        .priority(&own_proposal.proposed_token)
                                        .unwrap()
                                    {
                                        TokenPriority::High => {
                                            last_seen_token.last_seen_at = Instant::now();
                                            last_seen_token.current_proposal_version += 1;
                                            last_seen_token.state = TokenRefLastSeenState::Passive;
                                            log::info!(
                                                "Got proposal from more priority issuer {} for token {}. Stepping down",
                                                proposal.proposed_token.issuer,
                                                proposal.proposed_token.id,
                                            );
                                            self.pass_token_proposal(proposal);
                                        }
                                        TokenPriority::Equal => unreachable!(),
                                        TokenPriority::Low => {
                                            log::info!(
                                                "Got proposal from lower priority issuer {} for token {}. Dropping proposal",
                                                proposal.proposed_token.issuer,
                                                proposal.proposed_token.id,
                                            );
                                        }
                                    }
                                } else if proposal
                                    .propose_version
                                    .eq(&last_seen_token.current_proposal_version)
                                {
                                    let token = Token::from(proposal);
                                    *last_seen_token = TokenRefLastSeen {
                                        current_token_ref: TokenRef::from(&token),
                                        current_proposal_version: last_seen_token
                                            .current_proposal_version
                                            + 1,
                                        last_seen_at: Instant::now(),
                                        state: TokenRefLastSeenState::Passive,
                                    };
                                    log::info!("Generated new token {}", token.id);
                                    self.pass_token(token);
                                } else {
                                    log::warn!(
                                        "Dropping own outdated token proposal {:?}",
                                        proposal
                                    )
                                }
                            }
                        },
                    }
                } else {
                    log::warn!("Token proposal for unkown token {:?}", proposal);
                }
            }
            ThinkerMessage::TokenAliveBroadcast {
                token_ref,
                broadcast_issuer,
            } => {
                if self.mark_token_as_seen(&token_ref) {
                    self.token_broadcast(token_ref, broadcast_issuer);
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
            if !next_thinker.is_timed_out() {
                alive_amount += 1;
            }
            if alive_amount >= 2 {
                break;
            }
        }

        self.available_tokens.iter_mut().for_each(|last_seen| {
            if matches!(last_seen.state, TokenRefLastSeenState::Passive) && last_seen.is_timed_out()
            {
                last_seen.current_proposal_version += 1;
                last_seen.state = TokenRefLastSeenState::Propose(
                    last_seen
                        .current_token_ref
                        .generate_proposal(self.id.clone(), last_seen.current_proposal_version),
                );
                log::info!(
                    "Token {} timed out, switching in proposed state",
                    last_seen.current_token_ref.id
                );
            }
        });

        self.available_tokens
            .iter()
            .filter_map(|last_seen| match &last_seen.state {
                TokenRefLastSeenState::Passive => None,
                TokenRefLastSeenState::Propose(token_proposal) => Some(token_proposal),
            })
            .for_each(|proposal| self.pass_token_proposal(proposal.clone()));

        let active_token = match &self.state {
            ThinkerState::WaitingForForks { token, .. } | ThinkerState::Eating { token, .. } => {
                Some(TokenRef::from(token))
            }
            ThinkerState::Hungry { token_state, .. } => match token_state {
                HungryTokenState::WaitingForToken => None,
                HungryTokenState::TokenReceived(token) => Some(TokenRef::from(token)),
            },
            ThinkerState::Thinking { .. } => {
                // No token, nothing to do
                None
            }
        };
        if let Some(active_token) = active_token {
            let token_still_valid = self.mark_token_as_seen(&active_token);
            if !token_still_valid {
                self.state = ThinkerState::Hungry {
                    token_state: HungryTokenState::WaitingForToken,
                }
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
                                .send(ForkMessages::KeepAlive(self.id.clone()), &fork.address);
                        });
                        self.state = ThinkerState::WaitingForForks {
                            waiting_state: self.forks.clone().map(|_| WaitingForForkState {
                                state: ForkState::Queued,
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
                    token_state: self.available_tokens.iter().map(|el| el.into()).collect(),
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
