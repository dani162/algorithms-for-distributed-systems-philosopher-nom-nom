use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::fork::{Fork, ForkRef};
use crate::lib::thinker::{Thinker, ThinkerRef};
use crate::lib::utils::Id;
use crate::lib::visualizer::VisualizerRef;

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum ThinkerMessage {
    Init(InitThinkerParams),
    TakeForkAccepted(Id<Fork>),
    ForkAlive(Id<Fork>),
    ThinkerAliveRequest(Id<Thinker>),
    ThinkerAliveResponse(Id<Thinker>),
    Token,
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct InitThinkerParams {
    pub owns_token: bool,
    pub forks: [ForkRef; 2],
    pub next_thinkers: Vec<ThinkerRef>,
    pub visualizer: Option<VisualizerRef>,
}
