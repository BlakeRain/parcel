use clap::Parser;

#[derive(Debug, Parser)]
pub struct ShowUploadCommand {
    /// The host to show the upload on.
    ///
    /// If not provided, the default host will be used. This can be the alias you set when
    /// adding the host, or the actual hostname/IP address.
    #[arg(long)]
    pub host: Option<String>,
    /// The team to list uploads for.
    ///
    /// This should be the slug of the team, or the team's UUID. If not provided, the
    /// uplooads for the user will be listed.
    #[arg(long)]
    pub team: Option<String>,
    /// The identification of the upload to show
    ///
    /// This can be the UUID of the upload, the slug, or the filename. If a custom slug has been
    /// specified for the upload, that can also be used. If the filename conflicts with multiple
    /// uploads, an error will be returned.
    pub upload: String,
    /// Render as JSON instead of a table.
    #[arg(long)]
    pub json: bool,
}

