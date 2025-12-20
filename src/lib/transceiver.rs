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

    pub fn send<T>(&self, message: T, to: &SocketAddr)
    where
        T: for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, rkyv::rancor::Error>>
            + std::fmt::Debug,
    {
        println!("Send -> {:?}", message);
        let message = rkyv::to_bytes::<rkyv::rancor::Error>(&message).unwrap();
        self.socket.send_to(&message, to).unwrap();
    }

    pub fn receive<T>(&self, buffer: &mut [u8]) -> Option<(T, SocketAddr)>
    where
        T: Archive + std::fmt::Debug,
        T::Archived: for<'a> CheckBytes<HighValidator<'a, rkyv::rancor::Error>>
            + Deserialize<T, Strategy<Pool, rkyv::rancor::Error>>,
    {
        let (_, entity) = match self.socket.recv_from(buffer) {
            Ok(bytes) => bytes,
            Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock) => None?,
            Err(e) => panic!("{:?}", e),
        };
        let message = rkyv::from_bytes::<T, rkyv::rancor::Error>(buffer).unwrap();
        println!("Receive -> {:?}", message);
        Some((message, entity))
    }
}
