use clap::{builder::styling, Parser, Subcommand, ValueEnum};

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
