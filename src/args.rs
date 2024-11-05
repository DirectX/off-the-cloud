use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[clap(version, about)]
pub struct OffTheCloudArgs {
    #[clap(subcommand)]
    pub command: CommandType,
}

#[derive(Debug, Subcommand)]
pub enum CommandType {
    /// IMAP
    Imap(ImapCommand),
    Caldav(CalDAVCommand),
}

#[derive(Debug, Args)]
pub struct ImapCommand {
    #[clap(subcommand)]
    pub subcommand: ImapSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ImapSubcommand {
    /// Pulls data with IMAP protocol
    Pull(ImapPullSubcommand),
    /// Pushes data with IMAP protocol
    Push(ImapPushSubcommand),
}

#[derive(Debug, Args)]
pub struct ImapPullSubcommand {
    /// E-mail
    #[arg(long)]
    pub email: String,
    /// Password
    #[arg(long)]
    pub password: String,
    /// Output directory
    #[arg(long, default_value = "messages")]
    pub out_dir: String,
    /// Export messages in Mbox format
    #[arg(long, default_value_t = false)]
    pub export_mbox: bool,
    /// Mbox file size limit in megabytes (applies only if --export-mbox is set)
    #[arg(long, default_value = "50 MB")]
    pub max_file_size: String,
}

#[derive(Debug, Args)]
pub struct ImapPushSubcommand {
    /// E-mail
    #[arg(long)]
    pub email: String,
    /// Password
    #[arg(long)]
    pub password: String,
    /// Input directory
    #[arg(long, default_value = "messages")]
    pub in_dir: String,
}

#[derive(Debug, Args)]
pub struct CalDAVCommand {
    #[clap(subcommand)]
    pub subcommand: CalDAVSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum CalDAVSubcommand {
    /// Pulls data with CalDAV protocol
    Pull(CalDAVPullSubcommand),
    /// Pushes data with CalDAV protocol
    Push(CalDAVPushSubcommand),
}

#[derive(Debug, Args)]
pub struct CalDAVPullSubcommand {
    /// E-mail
    #[arg(long)]
    pub email: String,
    /// Password
    #[arg(long)]
    pub password: String,
    /// Output directory
    #[arg(long, default_value = "calendars")]
    pub out_dir: String,
}

#[derive(Debug, Args)]
pub struct CalDAVPushSubcommand {
    /// E-mail
    #[arg(long)]
    pub email: String,
    /// Password
    #[arg(long)]
    pub password: String,
    /// Input directory
    #[arg(long, default_value = "calendars")]
    pub in_dir: String,
}