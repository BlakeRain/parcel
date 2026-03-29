use clap::Parser;

#[derive(Debug, Parser)]
pub struct AddHostCommand {
    /// The alias for this host
    ///
    /// Aliases are used to refer to hosts in commands. If not provided, the host can only  be
    /// referred to by its hostname/IP address. All aliases must be unique.
    #[arg(long)]
    pub alias: Option<String>,
    /// Whether this is the default host
    #[arg(long)]
    pub default: bool,
    /// The API key to use for this host
    ///
    /// You can create an API key from your account settings in the Parcel web interface.
    #[arg(long)]
    pub key: String,
    /// Whether to use HTTPS
    #[arg(long, default_value_t = true)]
    #[arg(num_args(0..=1), default_missing_value("true"))]
    pub https: bool,
    /// The hostname or IP address of the host to add
    #[arg(long)]
    pub host: String,
}

