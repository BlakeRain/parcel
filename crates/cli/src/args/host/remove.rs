use clap::Parser;

#[derive(Debug, Parser)]
pub struct RemoveHostCommand {
    /// The alias or host of the host to remove
    ///
    /// This can be the alias you set when adding the host, or the actual hostname/IP address.
    #[arg(long)]
    pub host: String,
}

