use std::net::SocketAddr;

use crate::{
    Transceiver,
    fork_lib::fork::ForkRef,
    messages::{ForkMessages, Id, ThinkerMessage},
};

enum ForkState {
    Waiting,
    Taken,
}

enum ThinkerState {
    Thinking,
    Hungry,
    WaitingForForks([ForkState; 2]),
    Eating,
}

pub struct Thinker {
    id: Id<Thinker>,
    transceiver: Transceiver,
    state: ThinkerState,
    forks: [ForkRef; 2],
    next_thinker: SocketAddr,
}
impl Thinker {
    pub fn new(
        id: Id<Thinker>,
        transceiver: Transceiver,
        unhandled_messages: Vec<(ThinkerMessage, SocketAddr)>,
        forks: [ForkRef; 2],
        next_thinker: SocketAddr,
    ) -> Self {
        let mut thinker = Self {
            id,
            transceiver,
            state: ThinkerState::Thinking,
            forks,
            next_thinker,
        };
        unhandled_messages
            .into_iter()
            .for_each(|(message, entity)| {
                thinker.handle_message(message, entity);
            });
        thinker
    }

    pub fn handle_message(&mut self, message: ThinkerMessage, entity: SocketAddr) {
        match message {
            ThinkerMessage::Init(_) => {
                log::error!("Already initialized but got init message from {entity}");
            }
            ThinkerMessage::Token => match self.state {
                ThinkerState::Thinking
                | ThinkerState::WaitingForForks(_)
                | ThinkerState::Eating => {
                    self.transceiver
                        .send(ThinkerMessage::Token, &self.next_thinker);
                }
                ThinkerState::Hungry => {
                    self.forks.iter().for_each(|fork| {
                        self.transceiver.send(ForkMessages::Take, &fork.address);
                    });
                    self.state = ThinkerState::WaitingForForks(
                        self.forks.clone().map(|_| ForkState::Waiting),
                    );
                }
            },
            ThinkerMessage::TakeForkAccepted(id) => match &mut self.state {
                ThinkerState::WaitingForForks(forks_state) => {
                    let (_, state) = self
                        .forks
                        .iter()
                        .zip(&mut *forks_state)
                        .find(|(fork, _)| fork.id.eq(&id))
                        .unwrap();
                    *state = ForkState::Taken;
                    // TODO: maybe move to update state function
                    if forks_state
                        .iter()
                        .all(|state| matches!(state, ForkState::Taken))
                    {
                        self.state = ThinkerState::Eating;
                    }
                }
                ThinkerState::Thinking | ThinkerState::Eating | ThinkerState::Hungry => {
                    // TODO: This could happen if thinker node crashes restarts and afterwards gets
                    //  the response from the fork. This should be handled with proper error
                    //  handling. Maybe just tell the fork to release instantly.
                    panic!("Unescpected token accpeted message");
                }
            },
        }
    }

    pub fn tick(&mut self, buffer: &mut [u8]) {
        while let Some((message, entity)) = self.transceiver.receive::<ThinkerMessage>(buffer) {
            self.handle_message(message, entity);
        }
        // TODO: update states (thinking & eating time stuff)
    }
}
