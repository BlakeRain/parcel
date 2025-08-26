//! Preview configuration and command execution for file uploads.

use std::{borrow::Cow, collections::HashMap, path::Path};

use anyhow::Context;
use parcel_model::upload::Upload;
use serde::Deserialize;
use tokio::{io::AsyncReadExt, process::Command};

use crate::env::Env;

#[derive(Debug, Default, Deserialize)]
pub struct PreviewConfig {
    previewers: Vec<Previewer>,
}

impl PreviewConfig {
    pub async fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let mut file = tokio::fs::File::open(path)
            .await
            .context("failed to open configuration file")?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .await
            .context("failed to read configuration file")?;
        let config =
            serde_json::from_str(&content).context("failed to parse configuration file")?;
        Ok(config)
    }

    pub fn find_previewer(&self, mime_type: &str) -> Option<&Previewer> {
        self.previewers
            .iter()
            .filter(|previewer| previewer.is_enabled())
            .find(|previewer| previewer.matcher.matches(mime_type))
    }
}

#[derive(Debug, Deserialize)]
pub struct Previewer {
    #[serde(default)]
    feature: Option<PreviewerFeature>,
    #[serde(rename = "match")]
    matcher: PreviewerMatch,
    #[serde(default)]
    commands: Vec<PreviewerCommand>,
}

impl Previewer {
    pub fn is_enabled(&self) -> bool {
        self.feature
            .as_ref()
            .is_none_or(|feature| feature.is_enabled())
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub async fn run_commands(&self, env: &Env, upload: &mut Upload) -> bool {
        for command in &self.commands {
            if !command.run_command(env, upload).await {
                tracing::warn!(
                    "Command failed for upload {} with previewer {:?}",
                    upload.id,
                    self.matcher
                );

                return false;
            }
        }

        true
    }
}

#[derive(Debug, Deserialize)]
enum PreviewerFeature {
    #[serde(rename = "libreoffice")]
    LibreOffice,
}

impl PreviewerFeature {
    fn is_enabled(&self) -> bool {
        match self {
            Self::LibreOffice => cfg!(feature = "libreoffice"),
        }
    }
}

#[derive(Debug, Deserialize)]
enum PreviewerMatch {
    #[serde(rename = "exact")]
    Exact(String),
    #[serde(rename = "prefix")]
    Prefix(String),
}

impl PreviewerMatch {
    fn matches(&self, mime_type: &str) -> bool {
        match self {
            PreviewerMatch::Exact(ref exact) => mime_type == exact,
            PreviewerMatch::Prefix(ref prefix) => mime_type.starts_with(prefix),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PreviewerCommandName {
    Direct(String),
    Platforms(HashMap<String, String>),
}

#[derive(Debug, Deserialize)]
struct PreviewerCommand {
    command: PreviewerCommandName,
    args: Vec<String>,
}

impl PreviewerCommand {
    fn select_command(&self) -> Option<&str> {
        match self.command {
            PreviewerCommandName::Direct(ref cmd) => Some(cmd),
            PreviewerCommandName::Platforms(ref platforms) => {
                platforms.get(std::env::consts::OS).map(String::as_str)
            }
        }
    }

    fn build_command(&self, env: &Env, upload: &Upload) -> Option<Command> {
        let Some(cmd) = self.select_command() else {
            tracing::warn!("No command found for platform {}", std::env::consts::OS);
            return None;
        };

        let input = env.cache_dir.join(&upload.slug);
        let input_base = upload.slug.clone();
        let output = env.cache_dir.join(format!("{}.preview", upload.slug));
        let temp_dir = env.cache_dir.join("temp");

        let context = move |var: &str| -> Result<Option<Cow<'static, str>>, std::env::VarError> {
            match var {
                "input" => Ok(Some(Cow::Owned(input.to_string_lossy().to_string()))),
                "input_base" => Ok(Some(Cow::Owned(input_base.clone()))),
                "output" => Ok(Some(Cow::Owned(output.to_string_lossy().to_string()))),
                "temp_dir" => Ok(Some(Cow::Owned(temp_dir.to_string_lossy().to_string()))),
                _ => Err(std::env::VarError::NotPresent),
            }
        };

        let mut command = Command::new(cmd);

        for arg in &self.args {
            match shellexpand::env_with_context(arg, &context) {
                Ok(expanded) => {
                    command.arg(expanded.into_owned());
                }

                Err(err) => {
                    tracing::error!(
                        command = cmd,
                        mime_type = upload.mime_type.as_deref().unwrap_or("unknown"),
                        "Failed to expand argument '{}': {}",
                        arg,
                        err
                    );

                    return None;
                }
            }
        }

        Some(command)
    }

    async fn run_command(&self, env: &Env, upload: &mut Upload) -> bool {
        let Some(mut command) = self.build_command(env, upload) else {
            tracing::warn!(
                "Failed to build command for previewer for upload {}",
                upload.id
            );

            return false;
        };

        let output = match command.output().await {
            Ok(output) => output,
            Err(err) => {
                tracing::error!(
                    command = ?self,
                    "Failed to execute command for upload {}: {}",
                    upload.id,
                    err
                );

                let error_message = format!("Failed to execute preview command: {err}");

                upload
                    .set_preview_error(&env.pool, error_message)
                    .await
                    .unwrap_or_else(|err| {
                        tracing::error!(
                            "Failed to set preview error for upload {}: {}",
                            upload.id,
                            err
                        );
                    });

                return false;
            }
        };

        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr).trim().to_string();

            tracing::error!(
                command = ?self,
                "Command failed for upload {}: {}",
                upload.id,
                error_message
            );

            upload
                .set_preview_error(&env.pool, error_message)
                .await
                .unwrap_or_else(|err| {
                    tracing::error!(
                        "Failed to set preview error for upload {}: {}",
                        upload.id,
                        err
                    );
                });

            return false;
        }

        tracing::info!(
            "Successfully executed preview command for upload {}",
            upload.id
        );

        true
    }
}
