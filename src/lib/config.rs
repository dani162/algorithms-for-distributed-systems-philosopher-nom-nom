use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
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

pub trait Config {
    fn write(&self, path: &Path);
    fn read(path: &Path) -> Self;
}

impl<T> Config for T
where
    T: for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, rkyv::rancor::Error>>
        + Archive
        + std::fmt::Debug,
    T::Archived: for<'a> CheckBytes<HighValidator<'a, rkyv::rancor::Error>>
        + Deserialize<T, Strategy<Pool, rkyv::rancor::Error>>,
{
    fn write(&self, path: &Path) {
        let mut file = File::create(path).unwrap();
        let message_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(self).unwrap();
        file.write_all(&message_bytes).unwrap();
    }

    fn read(path: &Path) -> Self {
        let mut file = File::open(path).unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        rkyv::from_bytes::<T, rkyv::rancor::Error>(&buffer).unwrap()
    }
}
