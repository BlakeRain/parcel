use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use minijinja::Environment;
use notify::{recommended_watcher, RecommendedWatcher, Watcher};
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};

use super::functions::add_to_environment;

pub struct TemplateReloader {
    path: PathBuf,
    state: Arc<Mutex<ReloaderState>>,
    #[allow(unused)]
    watcher: RecommendedWatcher,
    env: Arc<RwLock<Environment<'static>>>,
}

#[derive(Default)]
struct ReloaderState {
    changed: bool,
}

impl TemplateReloader {
    fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        let state = Arc::new(Mutex::new(ReloaderState::default()));

        let tx = {
            let (tx, mut rx) = tokio::sync::mpsc::channel(1);
            let state = state.clone();
            tokio::spawn(async move {
                while (rx.recv().await).is_some() {
                    let mut state = state.lock().await;
                    state.changed = true;
                }
            });

            tx
        };

        let mut watcher = {
            let path = path.clone();
            recommended_watcher(move |event| {
                let Ok(event) = event else {
                    return;
                };

                if is_interesting_event(&event) && tx.blocking_send(()).is_err() {
                    tracing::warn!(?path, "Failed to notify template reloader");
                }
            })
        }
        .expect("to create a watcher");

        watcher
            .watch(&path, notify::RecursiveMode::Recursive)
            .expect("to begin watching a path");

        let env = load_environment(&path);
        let env = Arc::new(RwLock::new(env));

        Self {
            path: path.to_path_buf(),
            state,
            watcher,
            env,
        }
    }

    async fn acquire(&self) -> EnvironmentGuard<'_> {
        let mut state = self.state.lock().await;

        if state.changed {
            state.changed = false;
            let mut env = self.env.write().await;
            *env = load_environment(&self.path);
        }

        EnvironmentGuard {
            guarded: self.env.read().await,
        }
    }
}

fn is_interesting_event(event: &notify::Event) -> bool {
    use notify::event::{
        EventKind::*,
        ModifyKind::{self, *},
    };

    matches!(
        event.kind,
        Create(_) | Remove(_) | Modify(Data(_) | Name(_) | ModifyKind::Any)
    )
}

fn load_environment(path: &Path) -> Environment<'static> {
    let mut environment = Environment::new();
    environment.set_loader(minijinja::path_loader(path));
    add_to_environment(&mut environment);
    environment
}

pub struct EnvironmentGuard<'a> {
    guarded: RwLockReadGuard<'a, Environment<'static>>,
}

impl std::ops::Deref for EnvironmentGuard<'_> {
    type Target = Environment<'static>;

    fn deref(&self) -> &Self::Target {
        &self.guarded
    }
}

impl AsRef<Environment<'static>> for EnvironmentGuard<'_> {
    fn as_ref(&self) -> &Environment<'static> {
        &self.guarded
    }
}

pub async fn get_templates() -> EnvironmentGuard<'static> {
    const BASE_DIR: &str = std::env!("CARGO_MANIFEST_DIR");
    static RELOADER: std::sync::OnceLock<TemplateReloader> = std::sync::OnceLock::new();
    RELOADER
        .get_or_init(|| TemplateReloader::new(format!("{BASE_DIR}/templates")))
        .acquire()
        .await
}
