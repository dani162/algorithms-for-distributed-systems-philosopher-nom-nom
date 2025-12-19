use std::net::SocketAddr;

use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize)]
pub enum InitMessages {
    ForkRequest,
    ThinkerRequest,
}

#[derive(Archive, Serialize, Deserialize)]
pub enum ThinkerMessages {
    Init {
        owns_token: bool,
        fork_left: SocketAddr,
        fork_right: SocketAddr,
        next_thinker: SocketAddr,
    },
    Start,
    TakeForkAccepted,
    Token,
}

#[derive(Archive, Serialize, Deserialize)]
pub enum ForkMessages {
    Take,
    Release,
}
