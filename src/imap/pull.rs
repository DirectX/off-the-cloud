use anyhow::Context;
use futures::TryStreamExt;
use std::{
    env::current_dir,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    str::FromStr,
};
use tokio::net::TcpStream;

use crate::config::Config;

pub async fn pull(
    config: &Config,
    email: String,
    password: String,
    out_dir: String,
    max_file_size: u64,
) -> anyhow::Result<()> {
    let imap_config = config
        .imap
        .clone()
        .context("IMAP config is not provided in config.yaml")?;
    let imap_server = imap_config.server.clone();
    let imap_port = imap_config.port.unwrap_or(993);
    let imap_addr = (imap_server, imap_port);

    log::debug!("Pulling IMAP for account {email}...");

    let tcp_stream = TcpStream::connect(imap_addr.clone()).await?;
    let tls = async_native_tls::TlsConnector::new();
    let tls_stream = tls.connect(&imap_addr.0, tcp_stream).await?;

    let client = async_imap::Client::new(tls_stream);
    log::debug!("Connected to IMAP server {}:{}", imap_addr.0, imap_addr.1);

    let mut imap_session = client.login(&email, &password).await.map_err(|e| e.0)?;
    log::debug!("Logged in as {}", email);

    let mailbox_stream = imap_session
        .list(None, Some("*"))
        .await
        .context("error getting mailbox listt")?;
    let mailboxes: Vec<_> = mailbox_stream.try_collect().await?;

    log::debug!("Loaded {} mailboxes", mailboxes.len());

    for mailbox in mailboxes {
        let mailbox_name = mailbox.name();
        let mailbox_readable_name = utf7_imap::decode_utf7_imap(mailbox_name.to_string());
        log::debug!("Mailbox: {:?}", mailbox_readable_name);

        if mailbox_readable_name != "INBOX" {
            break;
        }

        imap_session.select(&mailbox_name).await?;
        log::debug!("{mailbox_name} selected");

        let folder_name = format!("{out_dir}/{email}/{mailbox_readable_name}",);
        fs::create_dir_all(&folder_name)?;

        let batch_size = 10;
        let mut message_id = 1u32;
        let mut bytes_written = 0usize;

        let mut part_id = 1;
        let file_name = format!("part-{:0>4}.mbox", part_id);

        let file_path = if folder_name.clone().starts_with("/") {
            PathBuf::from_str("/")
                .unwrap()
                .join(folder_name.clone())
                .join(file_name.clone())
        } else {
            current_dir()
                .unwrap()
                .join(folder_name.clone())
                .join(file_name.clone())
        };

        log::debug!("Creating part {}", file_path.to_string_lossy());
        log::debug!("Max file size: {max_file_size}");

        let mut out_file = File::create(file_path).context("Unable to open file")?;

        loop {
            let sequence_set = format!("{message_id}:{}", message_id + batch_size - 1);
            log::debug!("Querying {sequence_set}");

            let messages_stream = imap_session
                .fetch(sequence_set, "RFC822")
                .await
                .context("error getting messages")?;
            let messages: Vec<_> = messages_stream.try_collect().await?;

            if messages.len() == 0 {
                log::debug!("No more messages");
                break;
            } else {
                let mut current_message_id = message_id;

                for message in messages {
                    let body = message.body().context("message did not have a body!")?;
                    let body = std::str::from_utf8(body)
                        .context("message was not valid utf-8")?
                        .as_bytes();

                    let file_name = format!("{:0>8}.eml", current_message_id);
                    let file_path = if folder_name.clone().starts_with("/") {
                        PathBuf::from_str("/")
                            .unwrap()
                            .join(folder_name.clone())
                            .join(file_name.clone())
                    } else {
                        current_dir()
                            .unwrap()
                            .join(folder_name.clone())
                            .join(file_name.clone())
                    };
                    fs::write(file_path, body)
                        .context("unable to write file")
                        .context("unable to save *.eml file")?;
                    log::debug!("{} bytes eml message added", body.len());

                    let dt = chrono::Utc::now();
                    let timestamp = dt.format("%a %b %e %T %Y").to_string();
                    let prefix_string = format!("From MAILER-DAEMON {timestamp}\n");
                    let prefix = prefix_string.as_bytes();

                    let suffix_string = format!("\n\n");
                    let suffix = suffix_string.as_bytes();

                    out_file
                        .write(prefix)
                        .context("unable to write file prefix")?;
                    out_file.write(body).context("error writing data to file")?;
                    out_file
                        .write(suffix)
                        .context("unable to write file suffix")?;
                    log::debug!("{} bytes message added", prefix.len() + body.len() + suffix.len());

                    bytes_written += prefix.len() + body.len() + suffix.len();

                    current_message_id += 1;
                }
            }

            message_id += batch_size;
            break;
        }

        out_file.flush().context("error flushing file")?;
    }

    imap_session.logout().await?;

    log::debug!("Done");

    Ok(())
}
