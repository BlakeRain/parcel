use std::sync::Arc;

use crate::config::Config;

pub struct Context {
    pub inner: Arc<ContextInner>,
}

pub struct ContextInner {
    pub config: Config,
}

impl Clone for Context {
    fn clone(&self) -> Self {
        let inner = Arc::clone(&self.inner);
        Self { inner }
    }
}

impl std::ops::Deref for Context {
    type Target = ContextInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Context {
    pub fn new(config: Config) -> Self {
        let inner = ContextInner { config };
        let inner = Arc::new(inner);
        Self { inner }
    }
}
