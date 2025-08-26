use std::path::PathBuf;

use clap::{builder::styling, Parser, Subcommand};

const VERSION: &str = env!("CARGO_PKG_VERSION");

static HELP_TEMPLATE: &str = "\
{before-help}{name} {version}
{about}

{usage-heading}
  {usage}

{all-args}{after-help}";

pub mod host;
pub mod teams;
pub mod uploads;
pub mod download;
pub mod upload;

const STYLES: styling::Styles = styling::Styles::styled()
    .header(styling::AnsiColor::Green.on_default().bold())
    .usage(styling::AnsiColor::Green.on_default().bold())
    .literal(styling::AnsiColor::Blue.on_default().bold())
    .placeholder(styling::AnsiColor::Cyan.on_default());

/// CLI for Parcel file sharing service
#[derive(Debug, Parser)]
#[command(
    name = "parcel",
    version = VERSION,
    help_template = HELP_TEMPLATE,
    styles = STYLES,
)]
pub struct Args {
    /// The location of the Parcel configuration file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// The command to run
    #[command(subcommand)]
    command: ParcelCommand,
}

#[derive(Debug, Subcommand)]
pub enum ParcelCommand {
    /// Manage Parcel hosts
    #[command(subcommand)]
    Host(host::HostCommand),
    /// Access Parcel teams
    #[command(subcommand)]
    Teams(teams::TeamsCommand),
    /// Manage uploads on Parcel
    #[command(subcommand)]
    Uploads(uploads::UploadsCommand),
    /// Download from Parcel
    Download(download::DownloadCommand),
    /// Upload to Parcel
    Upload(upload::UploadCommand),
}
