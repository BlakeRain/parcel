use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use notify::{recommended_watcher, RecommendedWatcher, Watcher};

pub struct TailwindRebuilder {
    inner: Arc<Inner>,
}

impl Clone for TailwindRebuilder {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

pub struct Inner {
    #[allow(unused)]
    watcher: RecommendedWatcher,
}

impl TailwindRebuilder {
    pub fn new<W: AsRef<Path>, P: AsRef<Path>, I: IntoIterator<Item = P>>(
        working_dir: W,
        paths: I,
    ) -> anyhow::Result<Self> {
        let state = Arc::new(AtomicBool::new(false));
        let working_dir = working_dir.as_ref().to_path_buf();

        let tx = {
            let (tx, mut rx) = tokio::sync::mpsc::channel(1);
            let state = state.clone();
            let working_dir = working_dir.clone();
            tokio::spawn(async move {
                while (rx.recv().await).is_some() {
                    if state.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                        == Ok(false)
                    {
                        let state = state.clone();
                        let working_dir = working_dir.clone();
                        tokio::spawn(async move {
                            tracing::info!(?working_dir, "Running tailwind compiler");
                            let status = tokio::process::Command::new("npm")
                                .arg("run")
                                .arg("build-dev")
                                .current_dir(working_dir)
                                .status()
                                .await;

                            if let Err(err) = status {
                                tracing::error!("Failed to run tailwind compiler: {err}");
                            }

                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            state.store(false, Ordering::SeqCst);
                        });
                    }
                }
            });

            tx
        };

        let mut watcher = recommended_watcher(move |event| {
            let Ok(event) = event else {
                tracing::info!("Tailwind watcher is stopping");
                return;
            };

            if is_interesting_event(&event) && tx.blocking_send(()).is_err() {
                tracing::warn!("Failed to notify tailwind rebuilder task");
            }
        })?;

        for path in paths {
            watcher.watch(&working_dir.join(path), notify::RecursiveMode::Recursive)?;
        }

        let inner = Arc::new(Inner { watcher });
        Ok(Self { inner })
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
