use clap::Parser;

use crate::context::Context;

#[derive(Debug, Parser)]
pub struct ListTeamsCommand {
    /// The host to list teams for
    ///
    /// If not provided, the default host will be used. This can be the alias you set when
    /// adding the host, or the actual hostname/IP address.
    #[arg(long)]
    pub host: Option<String>,
}

impl ListTeamsCommand {
    pub async fn run(self, _context: Context) -> anyhow::Result<()> {
        Ok(())
    }
}
