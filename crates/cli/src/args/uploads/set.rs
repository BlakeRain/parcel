use clap::Parser;

#[derive(Debug, Parser)]
pub struct UploadSetCommand {
    /// The host to set the upload on.
    ///
    /// If not provided, the default host will be used. This can be the alias you set when
    /// adding the host, or the actual hostname/IP address.
    #[arg(long)]
    pub host: Option<String>,
    /// The team to set uploads for.
    ///
    /// This should be the slug of the team, or the team's UUID. If not provided, the
    /// uplooads for the user will be listed.
    #[arg(long)]
    pub team: Option<String>,
    /// The identification of the upload to set
    ///
    /// This can be the UUID of the upload, the slug, or the filename. If a custom slug has been
    /// specified for the upload, that can also be used. If the filename conflicts with multiple
    /// uploads, an error will be returned.
    pub upload: String,

    /// Set a new filename for the upload
    ///
    /// This will change the filename of the upload. If not provided, the filename will not be
    /// changed.
    #[arg(long)]
    pub filename: Option<String>,

    /// Set a custom slug for the upload
    ///
    /// This will change the custom slug of the upload. If not provided, the custom slug will not
    /// be changed.
    ///
    /// Note that the custom slug is not the same as the internal slug of the upload. Custom slugs
    /// must be unique across all uploads (either in your account or team).
    #[arg(long)]
    pub slug: Option<String>,

    /// Remove the custom slug from the upload
    ///
    /// This will remove the custom slug from the upload, reverting it to the internal slug.
    #[arg(long)]
    pub remove_slug: bool,

    /// Change whether the upload is public or private.
    ///
    /// This argument will change the visibility of the upload. If set to `true`, the upload will
    /// be public and accessible via a public link. If set to `false`, the upload will be private
    /// and only accessible by yourself or team members.
    #[arg(long)]
    pub public: Option<bool>,

    /// Change the download limit for the upload
    ///
    /// This will change the number of times that a public upload can be downloaded before it is
    /// locked. If not provided, the download limit will not be changed.
    #[arg(long)]
    pub download_limit: Option<usize>,

    /// Remove the download limit for the upload
    ///
    /// This will remove the download limit for the upload, allowing it to be downloaded an
    /// unlimited number of times.
    #[arg(long)]
    pub remove_download_limit: bool,

    /// Change the expiry date for the upload
    ///
    /// This will change the date on which the public link for the upload will expire. If not
    /// provided, the expiry date will not be changed.
    #[arg(long)]
    pub expiry_date: Option<String>,

    /// Remove the expiry date for the upload
    ///
    /// This will remove the expiry date for the upload, allowing it to be accessible via the
    /// public link indefinitely.
    #[arg(long)]
    pub remove_expiry_date: bool,

    /// Set the password for the upload
    ///
    /// This will set a password for the upload, which will be required to access the public link.
    /// If not provided, the password will not be changed. When this argument is provided, the
    /// tool will prompt you for the password.
    #[arg(long)]
    pub password: bool,

    /// Remove the password for the upload
    ///
    /// This will remove the password for the upload, allowing it to be accessed via the public
    /// link without a password.
    #[arg(long)]
    pub remove_password: bool,
}

