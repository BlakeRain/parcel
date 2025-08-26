use clap::Subcommand;

pub mod list;

#[derive(Debug, Subcommand)]
pub enum TeamsCommand {
    /// List all teams you're a member of
    List(list::ListTeamsCommand),
}
