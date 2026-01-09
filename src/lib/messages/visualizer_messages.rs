use std::time::Duration;

use rkyv::{Archive, Deserialize, Serialize};

use crate::lib::fork::{Fork, ForkRef};
use crate::lib::messages::thinker_messages::TokenRef;
use crate::lib::thinker::{Thinker, ThinkerRef};
use crate::lib::utils::Id;

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum VisualizerMessages {
    Init {
        thinkers: Vec<ThinkerRef>,
        forks: Vec<ForkRef>,
    },
    ForkStateChanged {
        id: Id<Fork>,
        state: VisualizerForkState,
    },
    ThinkerStateChanged {
        id: Id<Thinker>,
        state: VisualizerThinkerState,
        token_state: Vec<VisualizerThinkerAvailableTokenState>,
    },
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum VisualizerForkState {
    Unused,
    Used(Id<Thinker>),
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum VisualizerThinkerState {
    Thinking,
    Hungry,
    WaitingForForks { token: TokenRef },
    Eating { token: TokenRef },
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub enum VisualizerThinkerAvailableTokenState {
    Passive {
        not_seen_for: Duration,
    },
    Propose {
        token_version: u32,
        propose_version: u32,
    },
}
