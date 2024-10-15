use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[clap(version, about)]
pub struct OffTheCloudArgs {
    #[clap(subcommand)]
    pub command: CommandType,
}

#[derive(Debug, Subcommand)]
pub enum CommandType {
    /// Shows current progress and completed spans
    Imap(ImapCommand),
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
    #[arg(long, default_value = "INBOX")]
    pub mailbox: String,
    /// Output directory
    #[arg(long, default_value = "out")]
    pub out_dir: String,
    /// Mbox file size limit in megabytes 
    #[arg(long, default_value = "50 MB")]
    pub max_file_size: String,
}