use std::{marker::PhantomData, net::SocketAddr};

use rkyv::{Archive, Deserialize, Serialize};

use crate::{
    fork_lib::fork::{Fork, ForkRef},
    thinker_lib::thinker::Thinker,
};

#[derive(Archive, Serialize, Deserialize, Eq)]
pub struct Id<T> {
    pub value: String,
    _phantom: PhantomData<T>,
}
impl<T> Id<T> {
    pub fn random() -> Self {
        Self {
            value: uuid::Uuid::new_v4().to_string(),
            _phantom: PhantomData,
        }
    }
}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            _phantom: PhantomData,
        }
    }
}
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

#[derive(Archive, Serialize, Deserialize)]
pub enum InitMessages {
    ForkRequest(Id<Fork>),
    ThinkerRequest(Id<Thinker>),
}

#[derive(Archive, Serialize, Deserialize)]
pub struct InitThinkerParams {
    pub owns_token: bool,
    pub forks: [ForkRef; 2],
    pub next_thinker: SocketAddr,
}

#[derive(Archive, Serialize, Deserialize)]
pub enum ThinkerMessage {
    Init(InitThinkerParams),
    TakeForkAccepted(Id<Fork>),
    Token,
}

#[derive(Archive, Serialize, Deserialize)]
pub enum ForkMessages {
    Take,
    Release,
}
