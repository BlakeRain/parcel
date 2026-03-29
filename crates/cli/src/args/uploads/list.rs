use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
pub struct ListUploadsCommand {
    /// The host to list teams for.
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
    /// How to sort the uploads.
    #[arg(long, default_value = "uploaded-at")]
    pub sort: UploadSort,
    /// Change the ordering of the uploads.
    #[arg(long, default_value = "asc")]
    pub order: UploadOrder,
    /// Filter the uploads by the given filename.
    ///
    /// This is a case-insensitive filter that will match any uploads that contain the
    /// given filename in their name.
    #[arg(long)]
    pub filename: Option<String>,
    /// Filter the uploads by the given uploader.
    ///
    /// This is only relevant if the `team` argument is not provided. This argument should
    /// be the username of the uploader or their UUID.
    #[arg(long)]
    pub uploader: Option<String>,
    /// Render as JSON instead of a table.
    #[arg(long)]
    pub json: bool,
    /// The maximum number of uploads to return.
    ///
    /// This is useful for pagination. If not provided, the server limits the number of uploads
    /// returned via the API.
    #[arg(long, default_value_t = 100)]
    pub limit: usize,
    /// The offset to start returning uploads from.
    ///
    /// This is useful for pagination. If not provided, the server will return the first
    /// `limit` uploads.
    #[arg(long, default_value_t = 0)]
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum UploadSort {
    /// Order by the upload filename.
    Filename,
    /// Order by the size of the uploads.
    Size,
    /// Order by the number of downloads.
    Downloads,
    /// Order by the date on which the public link will expire.
    ExpiryDate,
    /// Order by the date on which the upload was created.
    UploadedAt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum UploadOrder {
    Asc,
    Desc,
}
