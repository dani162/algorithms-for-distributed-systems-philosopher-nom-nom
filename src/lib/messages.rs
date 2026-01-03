use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::fork::{Fork, ForkRef};
use crate::lib::thinker::{Thinker, ThinkerRef};
use crate::lib::utils::Id;

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Epoch(pub u64);

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReqId(pub u64);

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
pub struct Token {
    pub seq: u64,
    pub issuer: Id<Thinker>,
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum InitMessages {
    ForkRequest(Id<Fork>),
    ThinkerRequest(Id<Thinker>),
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct InitThinkerParams {
    pub owns_token: bool,
    pub forks: [ForkRef; 2],
    pub next_thinker: ThinkerRef,
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum ThinkerMessage {
    Init(InitThinkerParams),

    TakeForkAccepted {
        fork: Id<Fork>,
        epoch: Epoch,
        req: ReqId,
    },

    Token(Token),
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum ForkMessages {
    Take {
        thinker: Id<Thinker>,
        epoch: Epoch,
        req: ReqId,
    },
    Release {
        thinker: Id<Thinker>,
        epoch: Epoch,
        req: ReqId,
    },

    KeepAlive {
        thinker: Id<Thinker>,
        epoch: Epoch,
    },
}
