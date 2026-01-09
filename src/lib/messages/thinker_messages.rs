use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::fork::{Fork, ForkRef};
use crate::lib::thinker::{Thinker, ThinkerRef};
use crate::lib::utils::{EntityType, Id};
use crate::lib::visualizer::VisualizerRef;

// TODO: Remove clone if possible, some weird borrow shit :(
#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
pub struct Token {
    pub id: Id<Token>,
    pub version: u32,
    pub issuer: Id<Thinker>,
}

impl Token {
    pub fn create(issuer: Id<Thinker>) -> Self {
        Self {
            id: Id::random(),
            version: 0,
            issuer,
        }
    }

    pub fn priority(&self, other: &TokenRef) -> Option<TokenPriority> {
        TokenRef::from(self).priority(other)
    }
}

impl EntityType for Token {
    fn display_name() -> &'static str {
        "Token"
    }
}

impl From<TokenProposal> for Token {
    fn from(value: TokenProposal) -> Self {
        Self {
            id: value.proposed_token.id,
            version: value.proposed_token.version,
            issuer: value.proposed_token.issuer,
        }
    }
}

pub enum TokenPriority {
    High,
    Equal,
    Low,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
pub struct TokenRef {
    pub id: Id<Token>,
    pub version: u32,
    pub issuer: Id<Thinker>,
}

impl TokenRef {
    pub fn priority(&self, other: &Self) -> Option<TokenPriority> {
        if self.id.ne(&other.id) {
            return None;
        }
        Some(match self.version.cmp(&other.version) {
            std::cmp::Ordering::Greater => TokenPriority::High,
            std::cmp::Ordering::Less => TokenPriority::Low,
            std::cmp::Ordering::Equal => match self.issuer.cmp(&other.issuer) {
                std::cmp::Ordering::Less => TokenPriority::Low,
                std::cmp::Ordering::Equal => TokenPriority::Equal,
                std::cmp::Ordering::Greater => TokenPriority::High,
            },
        })
    }

    pub fn generate_proposal(&self, issuer: Id<Thinker>, proposal_version: u32) -> TokenProposal {
        TokenProposal {
            proposed_token: TokenRef {
                id: self.id.clone(),
                version: self.version + 1,
                issuer,
            },
            propose_version: proposal_version,
        }
    }
}

impl From<&Token> for TokenRef {
    fn from(value: &Token) -> Self {
        Self {
            id: value.id.clone(),
            version: value.version,
            issuer: value.issuer.clone(),
        }
    }
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
pub struct TokenProposal {
    pub proposed_token: TokenRef,
    pub propose_version: u32,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
pub enum ForkState {
    Queued,
    Taken,
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum ThinkerMessage {
    Init(InitThinkerParams),
    ForkAlive {
        id: Id<Fork>,
        state: ForkState,
    },
    ThinkerAliveRequest(Id<Thinker>),
    ThinkerAliveResponse(Id<Thinker>),
    Token(Token),
    TokenAliveBroadcast {
        token_ref: TokenRef,
        broadcast_issuer: Id<Thinker>,
    },
    ProposeToken(TokenProposal),
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct InitThinkerParams {
    pub token: Option<Token>,
    pub forks: [ForkRef; 2],
    pub next_thinkers: Vec<ThinkerRef>,
    pub visualizer: Option<VisualizerRef>,
    pub available_tokens: Vec<TokenRef>,
}
