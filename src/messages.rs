use std::net::SocketAddr;

use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize)]
pub enum InitRequests {
    ForkRequest,
    ThinkerRequest,
}

#[derive(Archive, Serialize, Deserialize)]
pub enum ThinkerResponses {
    Init {
        owns_token: bool,
        fork_left: SocketAddr,
        fork_right: SocketAddr,
        next_thinker: SocketAddr,
    },
    Start,
}
