use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::fork::Fork;
use crate::lib::thinker::Thinker;
use crate::lib::utils::Id;

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum InitMessages {
    ForkRequest(Id<Fork>),
    ThinkerRequest(Id<Thinker>),
    VisualizerRequest,
}
