use anyhow::Context;
use async_walkdir::{Filtering, WalkDir};
use futures_lite::stream::StreamExt;
use std::{
    env::current_dir,
    ffi::OsString,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    str::FromStr,
};
use tokio::{net::TcpStream, time::Instant};

use crate::config::Config;

pub async fn pull(
    config: &Config,
    email: String,
    password: String,
    out_dir: String,
    export_mbox: bool,
    max_file_size: usize,
) -> anyhow::Result<()> {
    let start = Instant::now();

    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let pull_cancellation_token = cancellation_token.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        log::info!("\nShutting down...");
        cancellation_token.cancel();
    });

    let domain = email
        .split("@")
        .last()
        .context("wrong email address {email}")?;
    log::info!("Domain: {domain}");

    let mut total_pulled_utf8_count = 0;
    let mut total_pulled_bin_count = 0;

    let imap_config = config
        .imap
        .clone()
        .context("IMAP config is not provided in config.yaml")?
        .pull
        .context("IMAP pull server config not provided")?;
    let imap_server = imap_config.server.clone();
    let imap_port = imap_config.port.unwrap_or(993);
    let imap_addr = (imap_server, imap_port);

    log::debug!("Pulling IMAP for account {email}...");
    let tcp_stream = TcpStream::connect(imap_addr.clone()).await?;
    let tls = async_native_tls::TlsConnector::new();
    let tls_stream = tls.connect(&imap_addr.0, tcp_stream).await?;

    let client = async_imap::Client::new(tls_stream);
    log::info!("Connected to IMAP server {}:{}", imap_addr.0, imap_addr.1);

    let mut imap_session = client.login(&email, &password).await.map_err(|e| e.0)?;
    log::info!("Logged in as {}", email);

    let mailbox_stream = imap_session
        .list(None, Some("*"))
        .await
        .context("error getting mailbox listt")?;
    let mailboxes: Vec<_> = mailbox_stream.try_collect().await?;

    log::info!("Loaded {} mailboxes", mailboxes.len());

    for mailbox in mailboxes {
        let mailbox_name = mailbox.name();
        let mailbox_readable_name = utf7_imap::decode_utf7_imap(mailbox_name.to_string());
        log::info!("Mailbox: {:?}", mailbox_readable_name);

        imap_session.select(&mailbox_name).await?;
        log::debug!("{mailbox_name} selected");

        let folder_name = format!("{out_dir}/{domain}/{email}/{mailbox_readable_name}",);
        let folder_path = if folder_name.clone().starts_with("/") {
            PathBuf::from_str("/").unwrap().join(folder_name.clone())
        } else {
            current_dir().unwrap().join(folder_name.clone())
        };
        let folder_path = folder_path.to_str().context("wrong in_dir path")?;
        fs::create_dir_all(&folder_path)?;

        log::info!("Folder {folder_path}");

        let entries = WalkDir::new(folder_path).filter(|entry| async move {
            match entry.file_type().await {
                Ok(file_type) => {
                    if file_type.is_file() {
                        if entry.path().extension() == Some("eml".as_ref()) {
                            Filtering::Continue
                        } else {
                            Filtering::Ignore
                        }
                    } else {
                        Filtering::IgnoreDir
                    }
                }
                Err(_) => Filtering::Continue,
            }
        });

        let last = entries.last().await;

        let starting_message_id = match last {
            Some(dir_entry_result) => match dir_entry_result {
                Ok(dir_entry) => {
                    let os_str_one = OsString::from("1");
                    let last_file_string = String::from(
                        dir_entry
                            .path()
                            .file_stem()
                            .unwrap_or(os_str_one.as_os_str())
                            .to_str()
                            .unwrap_or("1"),
                    );
                    let last_file_str = last_file_string
                        .trim_start_matches('.')
                        .trim_start_matches('0');
                    let last_file: usize = last_file_str.parse()?;
                    last_file + 1
                }
                Err(_) => 1,
            },
            None => 1,
        };

        log::info!("Starting message id: {}", starting_message_id);

        let batch_size = 200;
        let mut message_id = starting_message_id;

        if export_mbox {
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
            let mut out_file = File::create(file_path).context("Unable to open file")?;

            while !pull_cancellation_token.is_cancelled() {
                let sequence_set = format!("{message_id}:{}", message_id + batch_size - 1);
                log::info!("Querying {sequence_set}");

                let messages_stream = imap_session
                    .fetch(sequence_set, "RFC822")
                    .await
                    .context("error getting messages")?;
                let messages: Vec<_> = messages_stream.try_collect().await?;

                if messages.len() == 0 {
                    log::debug!("No more messages");
                    break;
                } else {
                    let mut current_message_id = message_id - 1;

                    for message in messages {
                        current_message_id += 1;
                        let body = message.body().context("message did not have a body!")?;
                        let body_string = std::str::from_utf8(body).ok();
                        if body_string.is_none() {
                            let bin_file_name = format!("{:0>8}.bin", current_message_id);
                            let bin_file_path = if folder_name.clone().starts_with("/") {
                                PathBuf::from_str("/")
                                    .unwrap()
                                    .join(folder_name.clone())
                                    .join(bin_file_name.clone())
                            } else {
                                current_dir()
                                    .unwrap()
                                    .join(folder_name.clone())
                                    .join(bin_file_name.clone())
                            };
                            fs::write(bin_file_path, body)
                                .context("unable to write file")
                                .context("unable to save *.bin file")?;
                            log::debug!("{} bytes bin data stored", body.len());

                            log::warn!(
                                "Message {} had invalid UTF-8. Storing as binary in {}.",
                                current_message_id,
                                bin_file_name
                            );
                            continue;
                        }
                        let body = body_string.unwrap().as_bytes();

                        let dt = chrono::Utc::now();
                        let timestamp = dt.format("%a %b %e %T %Y").to_string();
                        let prefix_string = format!("From MAILER-DAEMON {timestamp}\n");
                        let prefix = prefix_string.as_bytes();

                        let suffix_string = format!("\n\n");
                        let suffix = suffix_string.as_bytes();

                        if bytes_written + prefix.len() + body.len() + suffix.len() > max_file_size
                        {
                            log::debug!(
                                "File size exceed limit {} > {}",
                                prefix.len() + body.len() + suffix.len(),
                                max_file_size
                            );
                            out_file.flush().context("error flushing file")?;

                            part_id += 1;

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
                            out_file = File::create(file_path).context("Unable to open file")?;

                            bytes_written = 0;
                        }

                        out_file
                            .write(prefix)
                            .context("unable to write file prefix")?;
                        out_file.write(body).context("error writing data to file")?;
                        out_file
                            .write(suffix)
                            .context("unable to write file suffix")?;
                        log::debug!(
                            "{} bytes message added",
                            prefix.len() + body.len() + suffix.len()
                        );

                        bytes_written += prefix.len() + body.len() + suffix.len();
                    }
                }

                message_id += batch_size;
            }

            out_file.flush().context("error flushing file")?;
        } else {
            while !pull_cancellation_token.is_cancelled() {
                let sequence_set = format!("{message_id}:{}", message_id + batch_size - 1);
                log::info!("Querying {sequence_set}");

                let messages_stream = imap_session
                    .fetch(sequence_set, "RFC822")
                    .await
                    .context("error getting messages")?;
                let messages: Vec<_> = messages_stream.try_collect().await?;

                if messages.len() == 0 {
                    log::debug!("No more messages");
                    break;
                } else {
                    log::debug!("Fetching {} messages", messages.len());

                    let mut current_message_id = message_id - 1;

                    for message in messages {
                        current_message_id += 1;
                        let body = message.body().context("message did not have a body!")?;
                        let body_string = std::str::from_utf8(body).ok();
                        if body_string.is_none() {
                            let bin_file_name = format!(".{:0>8}.bin", current_message_id);
                            let bin_file_path = if folder_name.clone().starts_with("/") {
                                PathBuf::from_str("/")
                                    .unwrap()
                                    .join(folder_name.clone())
                                    .join(bin_file_name.clone())
                            } else {
                                current_dir()
                                    .unwrap()
                                    .join(folder_name.clone())
                                    .join(bin_file_name.clone())
                            };
                            fs::write(bin_file_path, body)
                                .context("unable to write file")
                                .context("unable to save *.bin file")?;

                            total_pulled_bin_count += 1;

                            log::debug!("{} bytes bin data stored", body.len());
                            log::warn!(
                                "Message {} had invalid UTF-8. Storing as binary in .{}.",
                                current_message_id,
                                bin_file_name
                            );
                            continue;
                        }
                        let body = body_string.unwrap().as_bytes();

                        let file_name = format!(".{:0>8}.eml", current_message_id);
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

                        total_pulled_utf8_count += 1;

                        log::debug!("{} bytes eml message added", body.len());
                    }
                }

                message_id += batch_size;
            }
        }
    }

    imap_session.logout().await?;

    log::info!(
        "Done in {:?}, {} new messages stored, {} was non-UTF8",
        start.elapsed(),
        total_pulled_utf8_count,
        total_pulled_bin_count
    );

    Ok(())
}
