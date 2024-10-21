use std::marker::PhantomData;

use serde::Serialize;
use sqlx::{encode::IsNull, Database, Sqlite};
use uuid::Uuid;

pub struct Key<T> {
    value: Uuid,
    phantom: PhantomData<T>,
}

impl<T> Key<T> {
    pub fn new() -> Self {
        Self {
            value: Uuid::new_v4(),
            phantom: PhantomData,
        }
    }
}

impl<T> Default for Key<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> std::fmt::Debug for Key<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Key({})", self.value)
    }
}

impl<T> std::fmt::Display for Key<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<T> Clone for Key<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Key<T> {}

impl<T> PartialEq for Key<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for Key<T> {}

impl<T> PartialOrd for Key<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Key<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T> std::hash::Hash for Key<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state)
    }
}

impl<T> From<Uuid> for Key<T> {
    fn from(value: Uuid) -> Self {
        Key {
            value,
            phantom: PhantomData,
        }
    }
}

impl<T> From<Key<T>> for Uuid {
    fn from(key: Key<T>) -> Self {
        key.value
    }
}

impl<T> Serialize for Key<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.value.to_string().serialize(serializer)
    }
}

impl<'de, T> serde::Deserialize<'de> for Key<T> {
    fn deserialize<D>(deserializer: D) -> Result<Key<T>, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let value = Uuid::deserialize(deserializer)?;
        Ok(Key {
            value,
            phantom: PhantomData,
        })
    }
}

impl<T> sqlx::Type<Sqlite> for Key<T> {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <uuid::fmt::Hyphenated as sqlx::Type<Sqlite>>::type_info()
    }

    fn compatible(ty: &<Sqlite as Database>::TypeInfo) -> bool {
        <uuid::fmt::Hyphenated as sqlx::Type<Sqlite>>::compatible(ty)
    }
}

impl<'q, T> sqlx::Encode<'q, Sqlite> for Key<T> {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, Box<dyn std::error::Error + Send + Sync>> {
        uuid::fmt::Hyphenated::from_uuid(self.value).encode_by_ref(buf)
    }
}

impl<'r, T> sqlx::Decode<'r, Sqlite> for Key<T> {
    fn decode(
        value: <Sqlite as Database>::ValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let value = <uuid::fmt::Hyphenated as sqlx::Decode<Sqlite>>::decode(value)?;
        Ok(Key {
            value: value.into(),
            phantom: PhantomData,
        })
    }
}
