use clap::Parser;

#[derive(Debug, Parser)]
pub struct ListTeamsCommand {
    /// The host to list teams for
    ///
    /// If not provided, the default host will be used. This can be the alias you set when
    /// adding the host, or the actual hostname/IP address.
    #[arg(long)]
    pub host: Option<String>,
}

