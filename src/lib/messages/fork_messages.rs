use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::thinker::Thinker;
use crate::lib::utils::Id;
use crate::lib::visualizer::VisualizerRef;

pub enum ThinkerForkState {
    Taken,
    Queued,
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum ForkMessages {
    Init(Option<VisualizerRef>),
    /// Used aquire the lock and keep it alive
    KeepAlive(Id<Thinker>),
    Release(Id<Thinker>),
}
