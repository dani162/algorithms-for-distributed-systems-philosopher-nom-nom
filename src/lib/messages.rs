use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::fork::{Fork, ForkRef};
use crate::lib::thinker::{Thinker, ThinkerRef};
use crate::lib::utils::Id;

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
    TakeForkAccepted(Id<Fork>),
    Token,
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum ForkMessages {
    Take,
    Release,
}
