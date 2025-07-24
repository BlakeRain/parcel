use clap::{builder::styling, Parser, Subcommand};

const VERSION: &str = env!("CARGO_PKG_VERSION");

static HELP_TEMPLATE: &str = "\
{before-help}{name} {version}
{about}

{usage-heading}
  {usage}

{all-args}{after-help}";

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
    Host(HostCommand),
    /// Access Parcel teams
    #[command(subcommand)]
    Teams(TeamsCommand),
    /// Manage uploads on Parcel
    #[command(subcommand)]
    Uploads(UploadsCommand),
    /// Download from Parcel
    #[command(subcommand)]
    Download(DownloadCommand),
    /// Upload to Parcel
    #[command(subcommand)]
    Upload(UploadCommand),
}

#[derive(Debug, Subcommand)]
pub enum HostCommand {
    /// Add a new host
    Add(AddHostCommand),
    /// Remove a host
    Remove(RemoveHostCommand),
    /// List all hosts
    List(ListHostsCommand),
    /// Check all hosts
    CheckAll,
    /// Check a specific host
    Check(CheckHostCommand),
    /// Change host settings
    Set(SetHostCommand),
}

#[derive(Debug, Subcommand)]
pub enum TeamsCommand {}

#[derive(Debug, Subcommand)]
pub enum UploadsCommand {}

#[derive(Debug, Subcommand)]
pub enum DownloadCommand {}

#[derive(Debug, Subcommand)]
pub enum UploadCommand {}

#[derive(Debug, Parser)]
pub struct AddHostCommand {
    /// The alias for this host
    ///
    /// Aliases are used to refer to hosts in commands. If not provided, the host can only  be
    /// referred to by its hostname/IP address. All aliases must be unique.
    #[arg(short, long)]
    pub alias: Option<String>,
    /// Whether this is the default host
    #[arg(short, long)]
    pub default: bool,
    /// The API key to use for this host
    ///
    /// You can create an API key from your account settings in the Parcel web interface.
    #[arg(short, long)]
    pub key: String,
    /// Whether to use HTTPS
    #[arg(long, default_value_t = true)]
    #[arg(num_args(0..=1), default_missing_value("true"))]
    pub https: bool,
    /// The hostname or IP address of the host to add
    #[arg(long)]
    pub host: String,
}

#[derive(Debug, Parser)]
pub struct RemoveHostCommand {
    /// The alias or host of the host to remove
    ///
    /// This can be the alias you set when adding the host, or the actual hostname/IP address.
    #[arg(short, long)]
    pub host: String,
}

#[derive(Debug, Parser)]
pub struct ListHostsCommand {}

#[derive(Debug, Parser)]
pub struct CheckHostCommand {
    /// The alias or host of the host to check
    ///
    /// This can be the alias you set when adding the host, or the actual hostname/IP address.
    #[arg(short, long)]
    pub host: String,
}

#[derive(Debug, Parser)]
pub struct SetHostCommand {
    /// The alias or host of the host to set
    ///
    /// This can be the alias you set when adding the host, or the actual hostname/IP address.
    #[arg(long)]
    pub host: String,
    /// The new alias for this host
    ///
    /// This option will assign a new alias to the host. If not provided, the alias will not be
    /// changed.
    #[arg(short, long)]
    pub alias: Option<String>,
    /// Remove the alias from this host
    #[arg(short, long)]
    pub remove_alias: bool,
    /// Set this host as the default host
    ///
    /// This will set the host as the default host for all commands that require a host where one
    /// is not specified. If a default host is already set, it will be replaced.
    #[arg(short, long)]
    pub default: bool,
    /// The API key to use for this host
    ///
    /// You can create an API key from your account settings in the Parcel web interface.
    #[arg(short, long)]
    pub key: Option<String>,
    /// Whether to use HTTPS
    #[arg(long)]
    #[arg(num_args(0..=1), default_missing_value("true"))]
    pub https: Option<bool>,
}
