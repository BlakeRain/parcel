use poem::session::Session;
use serde::de::DeserializeOwned;

pub trait SessionExt {
    fn take<T: DeserializeOwned>(&self, name: &str) -> Option<T>;
}

impl SessionExt for Session {
    fn take<T: DeserializeOwned>(&self, name: &str) -> Option<T> {
        let value = self.get(name)?;
        self.remove(name);
        Some(value)
    }
}
