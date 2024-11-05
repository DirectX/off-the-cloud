use anyhow::Context;
use args::{CommandType, OffTheCloudArgs};
use clap::Parser;
use config::Config;

pub mod args;
pub mod caldav;
pub mod config;
pub mod imap;

#[tokio::main]
async fn main() {
    let res = run().await;
    match res {
        Err(err) => log::error!("Error: {}", err),
        Ok(_) => log::info!("Done"),
    }
}

async fn run() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    pretty_env_logger::env_logger::builder().init();

    let args = OffTheCloudArgs::parse();
    log::debug!("Args: {:?}", args);

    let f = std::fs::File::open("config.yaml").context("config.yaml not found")?;
    let config: Config = serde_yaml::from_reader(f).context("config.yaml parse error")?;
    log::debug!("Config: {:?}", config);

    match args.command {
        CommandType::Imap(imap_command) => match imap_command.subcommand {
            args::ImapSubcommand::Pull(imap_pull_subcommand) => {
                imap::pull::pull(
                    &config,
                    imap_pull_subcommand.email,
                    imap_pull_subcommand.password,
                    imap_pull_subcommand.out_dir,
                    imap_pull_subcommand.export_mbox,
                    parse_size::parse_size(&imap_pull_subcommand.max_file_size).context(format!(
                        "malformed file size {:?}",
                        imap_pull_subcommand.max_file_size
                    ))? as usize,
                )
                .await?
            }
            args::ImapSubcommand::Push(imap_push_subcommand) => {
                imap::push::push(
                    &config,
                    imap_push_subcommand.email,
                    imap_push_subcommand.password,
                    imap_push_subcommand.in_dir,
                )
                .await?
            }
        },
        CommandType::Caldav(caldav_command) => match caldav_command.subcommand {
            args::CalDAVSubcommand::Pull(caldav_pull_subcommand) => {
                caldav::pull::pull(
                    &config,
                    caldav_pull_subcommand.email,
                    caldav_pull_subcommand.password,
                    caldav_pull_subcommand.out_dir,
                )
                .await?
            }
            args::CalDAVSubcommand::Push(_cal_davpush_subcommand) => {}
        },
    }

    Ok(())
}
