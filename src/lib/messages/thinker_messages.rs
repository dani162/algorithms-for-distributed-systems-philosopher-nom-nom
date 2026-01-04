use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::fork::{Fork, ForkRef};
use crate::lib::thinker::ThinkerRef;
use crate::lib::utils::Id;
use crate::lib::visualizer::VisualizerRef;

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum ThinkerMessage {
    Init(InitThinkerParams),
    TakeForkAccepted(Id<Fork>),
    Token,
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct InitThinkerParams {
    pub owns_token: bool,
    pub forks: [ForkRef; 2],
    pub next_thinker: ThinkerRef,
    pub visualizer: Option<VisualizerRef>,
}
