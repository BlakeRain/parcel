use clap::Subcommand;

pub mod add;
pub mod check;
pub mod list;
pub mod remove;
pub mod set;

#[derive(Debug, Subcommand)]
pub enum HostCommand {
    /// Add a new host
    Add(add::AddHostCommand),
    /// Remove a host
    Remove(remove::RemoveHostCommand),
    /// List all hosts
    List(list::ListHostsCommand),
    /// Check all hosts
    CheckAll,
    /// Check a specific host
    Check(check::CheckHostCommand),
    /// Change host settings
    Set(set::SetHostCommand),
}
