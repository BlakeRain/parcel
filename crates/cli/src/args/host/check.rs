use clap::Parser;

#[derive(Debug, Parser)]
pub struct CheckHostCommand {
    /// The alias or host of the host to check
    ///
    /// This can be the alias you set when adding the host, or the actual hostname/IP address.
    #[arg(long)]
    pub host: String,
}

