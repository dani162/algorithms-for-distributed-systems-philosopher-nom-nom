use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::fork::{Fork, ForkRef};
use crate::lib::thinker::{Thinker, ThinkerRef};
use crate::lib::utils::Id;
use crate::lib::visualizer::VisualizerRef;

// TODO: Remove clone if possible, some weird borrow shit :(
#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
pub struct Token {
    id: Id<Token>,
    pub version: usize,
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
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
pub struct TokenRef {
    pub id: Id<Token>,
    pub seq: usize,
    pub issuer: Id<Thinker>,
}

impl From<&Token> for TokenRef {
    fn from(value: &Token) -> Self {
        Self {
            id: value.id.clone(),
            seq: value.version,
            issuer: value.issuer.clone(),
        }
    }
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum ThinkerMessage {
    Init(InitThinkerParams),
    TakeForkAccepted(Id<Fork>),
    ForkAlive(Id<Fork>),
    ThinkerAliveRequest(Id<Thinker>),
    ThinkerAliveResponse(Id<Thinker>),
    Token(Token),
    TokenAliveBroadcast {
        token_ref: TokenRef,
        broadcast_issuer: Id<Thinker>,
    },
    ProposeToken {
        old_token: TokenRef,
        new_token: TokenRef,
    },
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct InitThinkerParams {
    pub token: Option<Token>,
    pub forks: [ForkRef; 2],
    pub next_thinkers: Vec<ThinkerRef>,
    pub visualizer: Option<VisualizerRef>,
    pub available_tokens: Vec<TokenRef>,
}
