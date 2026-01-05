use std::net::{SocketAddr, UdpSocket};

use rkyv::api::high::{HighSerializer, HighValidator};
use rkyv::de::Pool;
use rkyv::rancor::Strategy;
use rkyv::ser::allocator::ArenaHandle;
use rkyv::util::AlignedVec;
use rkyv::{Archive, Deserialize, Serialize, bytecheck::CheckBytes};

#[derive(Debug)]
pub struct Transceiver {
    socket: UdpSocket,
}
impl Transceiver {
    pub fn new(socket: UdpSocket) -> Self {
        socket.set_nonblocking(true).unwrap();
        Self { socket }
    }

    pub fn send_reliable<T>(&self, message: T, to: &SocketAddr)
    where
        T: for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, rkyv::rancor::Error>>
            + Archive
            + std::fmt::Debug,
        T::Archived: for<'a> CheckBytes<HighValidator<'a, rkyv::rancor::Error>>
            + Deserialize<T, Strategy<Pool, rkyv::rancor::Error>>,
    {
        let message_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&message).unwrap();
        self.socket.send_to(&message_bytes, to).unwrap();
    }

    pub fn send<T>(&self, message: T, to: &SocketAddr)
    where
        T: for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, rkyv::rancor::Error>>
            + Archive
            + std::fmt::Debug,
        T::Archived: for<'a> CheckBytes<HighValidator<'a, rkyv::rancor::Error>>
            + Deserialize<T, Strategy<Pool, rkyv::rancor::Error>>,
    {
        self.send_reliable(message, to);
    }

    pub fn receive<T>(&self, buffer: &mut [u8]) -> Option<(T, SocketAddr)>
    where
        T: Archive + std::fmt::Debug,
        T::Archived: for<'a> CheckBytes<HighValidator<'a, rkyv::rancor::Error>>
            + Deserialize<T, Strategy<Pool, rkyv::rancor::Error>>,
    {
        let (len, entity) = match self.socket.recv_from(buffer) {
            Ok(bytes) => bytes,
            Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock) => None?,
            Err(e) => panic!("{:?}", e),
        };
        let message = rkyv::from_bytes::<T, rkyv::rancor::Error>(&buffer[0..len]).unwrap();
        Some((message, entity))
    }

    pub fn local_address(&self) -> SocketAddr {
        self.socket.local_addr().unwrap()
    }
}
