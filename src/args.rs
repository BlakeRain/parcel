use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(author, about, long_about = None)]
pub struct Args {
    /// Enable logging ('-v' for debug, '-vv' for tracing).
    #[arg(short = 'v', long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// SQLite connection string.
    #[arg(long, default_value = "sqlite://parcel.db", env)]
    pub db: String,

    /// Director in which to store the file cache.
    #[arg(long, default_value = "./cache", env)]
    pub cache_dir: PathBuf,
}
