use std::marker::PhantomData;

use rkyv::{Archive, Deserialize, Serialize};
use uuid::Uuid;

pub trait EntityType {
    fn display_name() -> &'static str;
}

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct Id<T> {
    pub value: Uuid,
    _phantom: PhantomData<T>,
}
impl<T> Id<T> {
    pub fn random() -> Self {
        Self {
            value: uuid::Uuid::new_v4(),
            _phantom: PhantomData,
        }
    }
}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            _phantom: PhantomData,
        }
    }
}
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl<T> Eq for Id<T> {}

impl<T: EntityType> std::fmt::Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}Id({})", T::display_name(), self.value)
    }
}

impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
