use std::path::PathBuf;

use base64::Engine;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, about, long_about = None)]
pub struct Args {
    /// Enable logging ('-v' for debug, '-vv' for tracing).
    #[arg(short = 'v', long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// SQLite connection string.
    #[arg(long, default_value = "sqlite://parcel.db", env)]
    pub db: String,

    /// Directory in which to store the configuration files.
    #[arg(long, default_value = "./etc", env)]
    pub config_dir: PathBuf,

    /// Directory in which to store the file cache.
    #[arg(long, default_value = "./cache", env)]
    pub cache_dir: PathBuf,

    /// Cookie secret (must be 32-bytes, base64-encoded).
    #[arg(long, env)]
    pub cookie_secret: Option<String>,

    /// Plausible analytics domain.
    #[arg(long, env)]
    pub analytics_domain: Option<String>,

    /// Plausible analytics script URL.
    #[arg(long, env)]
    pub plausible_script: Option<String>,

    /// Interval at which the preview generation worker checks for uploads to process.
    #[arg(long, default_value = "10m", env)]
    pub preview_generation_interval: humantime::Duration,

    /// Maximum size of an upload that can have a preview generated.
    #[arg(long, env)]
    pub max_preview_size: Option<u64>,

    /// Allowed CORS origin(s). Can be specified multiple times. If not specified, CORS is disabled
    /// and only same-origin requests are allowed.
    #[arg(long = "cors-origin", env = "CORS_ORIGINS", value_delimiter = ',')]
    pub cors_origins: Vec<String>,

    /// Trust X-Forwarded-For and similar proxy headers for client IP detection.
    /// Enable this if running behind a reverse proxy (nginx, Cloudflare, etc.).
    /// When disabled, only the direct peer address is used.
    #[arg(long, env = "TRUST_PROXY")]
    pub trust_proxy: bool,
}

impl Args {
    pub fn get_cookie_key(&self) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(if let Some(secret) = &self.cookie_secret {
            Some(base64::engine::general_purpose::STANDARD.decode(secret)?)
        } else {
            None
        })
    }
}
