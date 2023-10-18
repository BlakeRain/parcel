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

    /// Directory in which to store the file cache.
    #[arg(long, default_value = "./cache", env)]
    pub cache_dir: PathBuf,

    /// Cookie secret (must be 32-bytes, base64-encoded).
    #[arg(long, env)]
    pub cookie_secret: Option<String>,
}

impl Args {
    pub fn get_cookie_key(&self) -> Result<Option<Vec<u8>>, base64::DecodeError> {
        Ok(if let Some(secret) = &self.cookie_secret {
            Some(base64::engine::general_purpose::STANDARD.decode(secret)?)
        } else {
            None
        })
    }
}
