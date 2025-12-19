use std::{
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

use rkyv::{
    Archive, Deserialize, Serialize,
    api::high::{HighSerializer, HighValidator},
    bytecheck::CheckBytes,
    de::Pool,
    rancor::Strategy,
    ser::allocator::ArenaHandle,
    util::AlignedVec,
};

pub mod messages;

pub const TICK_INTERVAL: Duration = Duration::from_millis(50);
pub const NETWORK_BUFFER_SIZE: usize = 1024;

pub struct Transceiver {
    socket: UdpSocket,
}
impl Transceiver {
    pub fn new(socket: UdpSocket) -> Self {
        socket.set_nonblocking(true).unwrap();
        Self { socket }
    }

    pub fn send(
        &self,
        message: impl for<'a> Serialize<
            HighSerializer<AlignedVec, ArenaHandle<'a>, rkyv::rancor::Error>,
        >,
        to: &SocketAddr,
    ) {
        let message = rkyv::to_bytes::<rkyv::rancor::Error>(&message).unwrap();
        self.socket.send_to(&message, to).unwrap();
    }

    pub fn receive<T>(&self, buffer: &mut [u8]) -> Option<(T, SocketAddr)>
    where
        T: Archive,
        T::Archived: for<'a> CheckBytes<HighValidator<'a, rkyv::rancor::Error>>
            + Deserialize<T, Strategy<Pool, rkyv::rancor::Error>>,
    {
        let (_, entity) = match self.socket.recv_from(buffer) {
            Ok(bytes) => bytes,
            Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock) => None?,
            Err(e) => panic!("{:?}", e),
        };
        let message = rkyv::from_bytes::<T, rkyv::rancor::Error>(buffer).unwrap();
        Some((message, entity))
    }
}
