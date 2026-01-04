use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::thinker::Thinker;
use crate::lib::utils::Id;
use crate::lib::visualizer::VisualizerRef;

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum ForkMessages {
    Init(Option<VisualizerRef>),
    Take(Id<Thinker>),
    Release,
}
