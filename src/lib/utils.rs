use std::marker::PhantomData;

use rkyv::{Archive, Deserialize, Serialize};

pub trait EntityType {
    fn display_name() -> &'static str;
}

#[derive(Archive, Serialize, Deserialize, Eq, Debug)]
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
impl<T: EntityType> std::fmt::Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}Id({})", T::display_name(), self.value)
    }
}
