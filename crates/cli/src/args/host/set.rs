use clap::Parser;

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
    #[arg(long)]
    pub alias: Option<String>,
    /// Remove the alias from this host
    #[arg(long)]
    pub remove_alias: bool,
    /// Set this host as the default host
    ///
    /// This will set the host as the default host for all commands that require a host where one
    /// is not specified. If a default host is already set, it will be replaced.
    #[arg(long)]
    pub default: bool,
    /// The API key to use for this host
    ///
    /// You can create an API key from your account settings in the Parcel web interface.
    #[arg(long)]
    pub key: Option<String>,
    /// Whether to use HTTPS
    #[arg(long)]
    #[arg(num_args(0..=1), default_missing_value("true"))]
    pub https: Option<bool>,
}

