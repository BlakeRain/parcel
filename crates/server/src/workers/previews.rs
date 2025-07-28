//! Generation of previews
//!
//! This worker will generate the preview images for the file uploads.
//!
//! This worker is triggered in one of two ways:
//!
//! 1. The worker gets a message on an MPSC queue that contains the ID of the upload to process, or
//! 2. The worker periodically checks the database for uploads that need preview generated.
//!
//! When scanning the database, the worker will only process uploads that have not yet had their
//! preview information configured. This is essentially the `has_preview` flag (a boolean) and the
//! `mime_type` column (a string). If the `mime_type` is not set, the worker will use `file` to
//! ascertain the MIME type of the file. If the `has_preview` flag is not set, the worker will
//! generate the preview image, using the `mime_type` to determine how to generate the preview.
//!
//! There are some caveats to this process:
//!
//! 1. If the worker sees that the preview generation failed (the `preview_error` column is set),
//!    then it won't bother trying to generate the preview again.
//! 2. If the worker sees that the upload is too large to generate a preview, it will skip the
//!    upload.
//! 3. If the worker sees that the upload is bigger than the configured maximum size for previews,
//!    it will skip the upload.

use std::sync::Arc;

use anyhow::Context;
use tokio::{process::Command, sync::mpsc::Sender, task::JoinHandle};

use parcel_model::{types::Key, upload::Upload};

use crate::env::Env;

mod config;

pub enum PreviewGenerationCommand {
    GeneratePreview(Vec<Key<Upload>>),
    Stop,
}

#[derive(Debug, Clone)]
pub struct PreviewWorker {
    sender: Sender<PreviewGenerationCommand>,
}

impl PreviewWorker {
    pub async fn generate_previews(&self, uploads: Vec<Key<Upload>>) -> anyhow::Result<()> {
        self.sender
            .send(PreviewGenerationCommand::GeneratePreview(uploads))
            .await
            .context("failed to send preview generation command")?;
        Ok(())
    }

    pub async fn stop(self) -> anyhow::Result<()> {
        self.sender
            .send(PreviewGenerationCommand::Stop)
            .await
            .context("failed to send stop command to preview generation worker")?;
        Ok(())
    }
}

pub async fn start_worker(env: Env) -> anyhow::Result<(PreviewWorker, JoinHandle<()>)> {
    // Try and load our previewer configuration file.
    let config_path = env.config_dir.join("previewers.json");
    let config = if config_path.exists() {
        config::PreviewConfig::from_file(&config_path)
            .await
            .context("failed to read 'previewers.json' configuration")?
    } else {
        tracing::warn!(
            ?config_path,
            "Previewer configuration file not found; no previews will be generated",
        );

        config::PreviewConfig::default()
    };

    let config = Arc::new(config);
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);

    let task = tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        PreviewGenerationCommand::GeneratePreview(keys) => {
                            let config = Arc::clone(&config);
                            let env = env.clone();
                            tokio::spawn(async move {
                                if let Err(err) = generate_previews(config, env.clone(), keys).await {
                                    tracing::error!("Failed to generate preview batch: {}", err);
                                }
                            });
                        },
                        PreviewGenerationCommand::Stop => {
                            tracing::info!("Stopping preview generation worker");
                            break;
                        }
                    }
                },

                _ = tokio::time::sleep(env.preview_generation_interval) => {
                    let config = Arc::clone(&config);
                    if let Err(e) = scan_for_uploads(config, env.clone()).await {
                        tracing::error!("Failed to scan for uploads to generate previews: {}", e);
                    }
                },
            }
        }
    });

    Ok((PreviewWorker { sender: tx }, task))
}

const SCAN_MAX_SIZE: u32 = 10;

async fn scan_for_uploads(config: Arc<config::PreviewConfig>, env: Env) -> anyhow::Result<()> {
    let mut offset = 0;

    loop {
        let uploads = Upload::get_all_without_preview(&env.pool, offset, SCAN_MAX_SIZE).await?;
        if uploads.is_empty() {
            tracing::info!("No uploads found that need preview generation");
            return Ok(());
        }

        let count = uploads.len() as u32;
        tracing::info!("Found {count} uploads that need preview generation");

        for upload in uploads {
            generate_preview(&config, &env, upload).await;
        }

        if count < SCAN_MAX_SIZE {
            tracing::info!("Processed all uploads that needed preview generation");
            return Ok(());
        }

        offset += count;
    }
}

async fn generate_previews(
    config: Arc<config::PreviewConfig>,
    env: Env,
    uploads: Vec<Key<Upload>>,
) -> anyhow::Result<()> {
    for id in uploads {
        let Some(upload) = Upload::get(&env.pool, id).await? else {
            tracing::warn!("Upload with ID {} not found, skipping", id);
            continue;
        };

        generate_preview(&config, &env, upload).await;
    }

    Ok(())
}

async fn generate_preview(config: &config::PreviewConfig, env: &Env, mut upload: Upload) {
    if upload.has_preview {
        tracing::info!("Upload {} already has a preview, skipping", upload.id);
        return;
    }

    if let Some(max_preview_size) = env.max_preview_size {
        if upload.size > max_preview_size as i64 {
            tracing::info!(
                "Upload {} is too large for preview generation ({} bytes), skipping",
                upload.id,
                upload.size
            );

            return;
        }
    }

    if upload.mime_type.is_none() {
        if let Err(err) = ascertain_mime_type(env, &mut upload).await {
            tracing::error!(
                "Failed to ascertain MIME type for upload {}: {}",
                upload.id,
                err
            );
            return;
        }
    }

    let Some(ref mime_type) = upload.mime_type else {
        tracing::warn!(
            "Upload {} has no MIME type, skipping preview generation",
            upload.id
        );
        return;
    };

    let Some(previewer) = config.find_previewer(mime_type) else {
        tracing::warn!(
            "No previewer found for MIME type '{}', skipping upload {}",
            mime_type,
            upload.id
        );

        return;
    };

    if !previewer.is_enabled() {
        tracing::warn!(
            "Previewer matching MIME type '{}' is not enabled, skipping upload {}",
            mime_type,
            upload.id
        );

        return;
    }

    if previewer.is_empty() {
        tracing::warn!(
            "No commands configured for previewer matching MIME type '{}', skipping upload {}",
            mime_type,
            upload.id
        );

        return;
    }

    if !previewer.run_commands(env, &mut upload).await {
        tracing::warn!("Previewer failed to run commands for upload {}", upload.id);
        return;
    }

    upload
        .set_has_preview(&env.pool, true)
        .await
        .unwrap_or_else(|err| {
            tracing::error!(
                "Failed to set has_preview for upload {}: {}",
                upload.id,
                err
            );
        });
}

async fn ascertain_mime_type(env: &Env, upload: &mut Upload) -> anyhow::Result<()> {
    let path = env.cache_dir.join(&upload.slug);
    let output = Command::new("file")
        .arg("--mime-type")
        .arg("-b")
        .arg(path)
        .output()
        .await?;
    let mime = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if mime.is_empty() {
        tracing::warn!("Failed to ascertain MIME type for upload {}", upload.id);
        return Ok(());
    }

    tracing::info!("Ascertained MIME type for upload {}: {}", upload.id, mime);
    upload.set_mime_type(&env.pool, mime.as_str()).await?;

    Ok(())
}
