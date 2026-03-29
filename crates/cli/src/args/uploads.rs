use clap::Subcommand;

pub mod list;
pub mod show;
pub mod set;

#[derive(Debug, Subcommand)]
pub enum UploadsCommand {
    /// List all uploads
    ///
    /// This command will list all the uploads for your account, or for a specific team (so long as
    /// you have access to that team). You can also specify filters to reduce the number of
    /// uploads returned, such as by filename or uploader.
    List(list::ListUploadsCommand),
    /// Show information about a specific upload
    ///
    /// This will show information about an upload, including its filename, slug, UUID,
    /// uploader, and so on. This also includes the link for the upload.
    Show(show::ShowUploadCommand),
    /// Change the settings for an upload
    ///
    /// This command allows you to change the settings for an upload, such as changing the
    /// filename, adding a custom slug, making the upload public, or setting an expiry date.
    Set(set::UploadSetCommand),
}

