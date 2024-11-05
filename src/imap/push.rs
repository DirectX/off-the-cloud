use anyhow::Context;
use human_bytes::human_bytes;
use std::{env::current_dir, fs, path::PathBuf, str::FromStr};
use tokio::{net::TcpStream, time::Instant};

use crate::config::Config;

pub async fn push(
    config: &Config,
    email: String,
    password: String,
    in_dir: String,
) -> anyhow::Result<()> {
    let start = Instant::now();

    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let push_cancellation_token = cancellation_token.clone();

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

    let mut total_pushed_count = 0;

    let folder_name = format!("{in_dir}/{domain}/{email}/",);
    let folder_path = if folder_name.clone().starts_with("/") {
        PathBuf::from_str("/").unwrap().join(folder_name.clone())
    } else {
        current_dir().unwrap().join(folder_name.clone())
    };
    let folder_path = folder_path.to_str().context("wrong in_dir path")?;

    log::info!("Getting mailboxes in {}", folder_path);

    let mut mailboxes = Vec::<(String, String)>::new();

    // 5 Level scan for now
    // TODO: refactor
    for entry in fs::read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let mailbox_path = path.to_str().unwrap_or_default().to_string();
            let mailbox_name = mailbox_path[folder_path.len()..].to_string();
            log::debug!("L0 {} -> {}", mailbox_name, mailbox_path);
            mailboxes.push((mailbox_name, mailbox_path.clone())); //remove clone after testing

            for entry in fs::read_dir(&mailbox_path)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    let mailbox_path = path.to_str().unwrap_or_default().to_string();
                    let mailbox_name = mailbox_path[folder_path.len()..].to_string();
                    log::debug!("L1 {} -> {}", mailbox_name, mailbox_path);
                    mailboxes.push((mailbox_name, mailbox_path.clone())); //remove clone after testing

                    for entry in fs::read_dir(&mailbox_path)? {
                        let entry = entry?;
                        let path = entry.path();

                        if path.is_dir() {
                            let mailbox_path = path.to_str().unwrap_or_default().to_string();
                            let mailbox_name = mailbox_path[folder_path.len()..].to_string();
                            log::debug!("L2 {} -> {}", mailbox_name, mailbox_path);
                            mailboxes.push((mailbox_name, mailbox_path.clone())); //remove clone after testing

                            for entry in fs::read_dir(&mailbox_path)? {
                                let entry = entry?;
                                let path = entry.path();

                                if path.is_dir() {
                                    let mailbox_path =
                                        path.to_str().unwrap_or_default().to_string();
                                    let mailbox_name =
                                        mailbox_path[folder_path.len()..].to_string();
                                    log::debug!("L3 {} -> {}", mailbox_name, mailbox_path);
                                    mailboxes.push((mailbox_name, mailbox_path.clone())); //remove clone after testing

                                    for entry in fs::read_dir(&mailbox_path)? {
                                        let entry = entry?;
                                        let path = entry.path();

                                        if path.is_dir() {
                                            let mailbox_path =
                                                path.to_str().unwrap_or_default().to_string();
                                            let mailbox_name =
                                                mailbox_path[folder_path.len()..].to_string();
                                            log::debug!("L4 {} -> {}", mailbox_name, mailbox_path);
                                            mailboxes.push((mailbox_name, mailbox_path));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    log::info!("Found {} mailboxes", mailboxes.len());

    if mailboxes.len() > 0 {
        let imap_config = config
            .imap
            .clone()
            .context("IMAP config is not provided in config.yaml")?
            .push
            .context("IMAP push server config not provided")?;
        let imap_server = imap_config.server.clone();
        let imap_port = imap_config.port.unwrap_or(993);
        let imap_addr = (imap_server, imap_port);
        let folder_delimiter = &imap_config.folder_delimiter.unwrap_or('/').to_string();

        log::debug!("Pushing IMAP for account {email}...");
        let tcp_stream = TcpStream::connect(imap_addr.clone()).await?;
        let tls = async_native_tls::TlsConnector::new();
        let tls_stream = tls.connect(&imap_addr.0, tcp_stream).await?;

        let client = async_imap::Client::new(tls_stream);
        log::info!("Connected to IMAP server {}:{}", imap_addr.0, imap_addr.1);

        let mut imap_session = client.login(&email, &password).await.map_err(|e| e.0)?;
        log::info!("Logged in as {}", email);

        for (mailbox_name, mailbox_path) in mailboxes {
            if push_cancellation_token.is_cancelled() {
                log::debug!("Cancelling...");
                break;
            }

            let mailbox_mapped_name = match imap_config.folder_name_mappings {
                Some(ref folder_name_mappings) => {
                    if folder_name_mappings.contains_key(&mailbox_name) {
                        folder_name_mappings.get(&mailbox_name).unwrap().clone()
                    } else {
                        mailbox_name.clone()
                    }
                }
                None => mailbox_name.clone(),
            };
            let mailbox_utf7_name =
                utf7_imap::encode_utf7_imap(mailbox_mapped_name.replace("/", folder_delimiter));

            log::info!(
                "Processing mailbox {} ({:?})",
                mailbox_mapped_name,
                &mailbox_utf7_name
            );

            if let Some(err) = imap_session.create(&mailbox_utf7_name).await.err() {
                log::debug!("Unable to create folder: {}", err);
            }

            imap_session.select(&mailbox_utf7_name).await?;
            log::debug!("Mailbox {mailbox_name} selected");

            // Getting unread mails
            let mut unsent_emails: Vec<(String, String)> = Vec::new();
            for entry in fs::read_dir(&mailbox_path)? {
                let path = entry?.path();

                if path.is_dir() {
                    continue;
                }

                let filename =
                    path.to_string_lossy().to_string()[mailbox_path.len() + 1..].to_string();
                if filename.starts_with(".")
                    && path
                        .extension()
                        .is_some_and(|ext| ext.to_string_lossy() == "eml".to_string())
                {
                    unsent_emails.push((filename, path.to_string_lossy().to_string()));
                }
            }
            unsent_emails.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

            let mut pushed_count = 0;

            for entry in unsent_emails {
                if push_cancellation_token.is_cancelled() {
                    log::debug!("Cancelling...");
                    break;
                }

                let eml_file_name = entry.0;
                let eml_file_path = entry.1;
                let message_id = eml_file_name
                    .trim_start_matches('.')
                    .trim_start_matches('0')
                    .trim()
                    .trim_end_matches(".eml");

                log::debug!(
                    "Pushing message {} to {}...",
                    message_id,
                    mailbox_mapped_name
                );

                let eml_data = fs::read(&eml_file_path).ok();
                if let Some(data) = eml_data {
                    let size = data.len() as u32;

                    match imap_session
                        .append(&mailbox_utf7_name, Some(r"(\Seen)"), None, data)
                        .await
                    {
                        Ok(_) => {
                            let eml_file_name = format!(".{:0>8}.eml", message_id);
                            let new_eml_file_name = format!("{:0>8}.eml", message_id);
                            let new_eml_file_path =
                                eml_file_path.replace(&eml_file_name, &new_eml_file_name);
                            fs::rename(eml_file_path, new_eml_file_path)?;

                            pushed_count += 1;
                            total_pushed_count += 1;

                            log::debug!("{} sent ok", human_bytes(size));
                        }
                        Err(err) => log::debug!("Error pushing message: {}", err),
                    };
                }
            }
            log::info!("Uploaded {pushed_count} messages to {mailbox_mapped_name}");
        }

        imap_session.logout().await?;
    }

    log::info!(
        "Done in {:?}, {} messages uploaded ok.",
        start.elapsed(),
        total_pushed_count
    );

    Ok(())
}
