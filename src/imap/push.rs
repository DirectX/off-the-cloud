use anyhow::Context;
use async_imap::types::Flag;
use futures::TryStreamExt;
use std::{
    env::current_dir,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};
use tokio::{net::TcpStream, time::Instant};
use walkdir::{DirEntry, WalkDir};

use crate::config::Config;

pub async fn push(
    config: &Config,
    email: String,
    password: String,
    in_dir: String,
) -> anyhow::Result<()> {
    let start = Instant::now();

    let folder_name = format!("{in_dir}/{email}/",);
    let folder_path = if folder_name.clone().starts_with("/") {
        PathBuf::from_str("/").unwrap().join(folder_name.clone())
    } else {
        current_dir().unwrap().join(folder_name.clone())
    };
    let folder_path = folder_path.to_str().context("wrong in_dir path")?;

    log::info!("Getting mailboxes in {}", folder_path);

    let mut mailboxes = Vec::<(String, String)>::new();
    WalkDir::new(folder_path)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| e.file_type().is_dir())
        .filter_map(|v| v.ok())
        .for_each(|x| {
            let mailbox_path = x.path().to_str().unwrap_or_default().to_string();
            let mailbox_name = mailbox_path[folder_path.len()..].to_string();
            mailboxes.push((mailbox_name, mailbox_path));
        });
    log::info!("Found {} mailboxes", mailboxes.len());

    if mailboxes.len() > 0 {
        let imap_config = config
            .imap
            .clone()
            .context("IMAP config is not provided in config.yaml")?
            .push
            .context("IMAP pull server config not provided")?;
        let imap_server = imap_config.server.clone();
        let imap_port = imap_config.port.unwrap_or(993);
        let imap_addr = (imap_server, imap_port);
        let folder_delimiter = &imap_config.folder_delimiter.unwrap_or('/').to_string();

        log::debug!("Pulling IMAP for account {email}...");
        let tcp_stream = TcpStream::connect(imap_addr.clone()).await?;
        let tls = async_native_tls::TlsConnector::new();
        let tls_stream = tls.connect(&imap_addr.0, tcp_stream).await?;

        let client = async_imap::Client::new(tls_stream);
        log::info!("Connected to IMAP server {}:{}", imap_addr.0, imap_addr.1);

        let mut imap_session = client.login(&email, &password).await.map_err(|e| e.0)?;
        log::info!("Logged in as {}", email);

        for (mailbox_name, mailbox_path) in mailboxes {
            let mailbox_utf7_name =
                utf7_imap::encode_utf7_imap(mailbox_name.replace("/", folder_delimiter));
            log::info!(
                "Processing mailbox {} ({:?})",
                mailbox_name,
                mailbox_utf7_name
            );

            if let Some(err) = imap_session.create(&mailbox_utf7_name).await.err() {
                log::warn!("Unable to create folder: {}", err);
            }

            // imap_session.select(&mailbox_utf7_name).await?;
            // log::debug!("Mailbox {mailbox_name} selected");

            WalkDir::new(mailbox_path)
                .min_depth(1)
                .into_iter()
                .filter_entry(|e| {
                    e.file_type().is_file() && e.path().extension() == Some("eml".as_ref())
                })
                .filter_map(|v| v.ok())
                .for_each(|x| {
                    let file_path = x.path().display().to_string();
                    let message_id = Path::new(&file_path)
                        .file_stem()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default()
                        .trim_start_matches('0');

                    let data = fs::read(&file_path).ok();
                    if let Some(data) = data {
                        log::debug!("Message {} size: {}", message_id, data.len());

                        imap_session
                            .append(mailbox_utf7_name, Some(r"\Seen"), None, data)
                            .await
                            .context("error adding message")?;
                    }
                });
        }

        imap_session.logout().await?;
    }

    // let mailbox_stream = imap_session
    //     .list(None, Some("*"))
    //     .await
    //     .context("error getting mailbox listt")?;
    // let mailboxes: Vec<_> = mailbox_stream.try_collect().await?;

    // log::info!("Loaded {} mailboxes", mailboxes.len());

    // for mailbox in mailboxes {
    //     let mailbox_name = mailbox.name();
    //     let mailbox_readable_name = utf7_imap::decode_utf7_imap(mailbox_name.to_string());
    //     log::info!("Mailbox: {:?}", mailbox_readable_name);

    //     imap_session.select(&mailbox_name).await?;
    //     log::debug!("{mailbox_name} selected");

    //     let folder_name = format!("{in_dir}/{email}/{mailbox_readable_name}",);
    //     fs::create_dir_all(&folder_name)?;

    //     let batch_size = 200;
    //     let mut message_id = 1u32;

    //     if export_mbox {
    //         let mut bytes_written = 0usize;
    //         let mut part_id = 1;

    //         let file_name = format!("part-{:0>4}.mbox", part_id);
    //         let file_path = if folder_name.clone().starts_with("/") {
    //             PathBuf::from_str("/")
    //                 .unwrap()
    //                 .join(folder_name.clone())
    //                 .join(file_name.clone())
    //         } else {
    //             current_dir()
    //                 .unwrap()
    //                 .join(folder_name.clone())
    //                 .join(file_name.clone())
    //         };

    //         log::debug!("Creating part {}", file_path.to_string_lossy());
    //         let mut out_file = File::create(file_path).context("Unable to open file")?;

    //         loop {
    //             let sequence_set = format!("{message_id}:{}", message_id + batch_size - 1);
    //             log::info!("Querying {sequence_set}");

    //             let messages_stream = imap_session
    //                 .fetch(sequence_set, "RFC822")
    //                 .await
    //                 .context("error getting messages")?;
    //             let messages: Vec<_> = messages_stream.try_collect().await?;

    //             if messages.len() == 0 {
    //                 log::debug!("No more messages");
    //                 break;
    //             } else {
    //                 let mut current_message_id = message_id - 1;

    //                 for message in messages {
    //                     current_message_id += 1;
    //                     let body = message.body().context("message did not have a body!")?;
    //                     let body = std::str::from_utf8(body).ok();
    //                     if body.is_none() {
    //                         log::warn!("Message {} had invalid UTF-8", current_message_id);
    //                         continue;
    //                     }
    //                     let body = body.unwrap().as_bytes();

    //                     let dt = chrono::Utc::now();
    //                     let timestamp = dt.format("%a %b %e %T %Y").to_string();
    //                     let prefix_string = format!("From MAILER-DAEMON {timestamp}\n");
    //                     let prefix = prefix_string.as_bytes();

    //                     let suffix_string = format!("\n\n");
    //                     let suffix = suffix_string.as_bytes();

    //                     if bytes_written + prefix.len() + body.len() + suffix.len() > max_file_size {
    //                         log::debug!("File size exceed limit {} > {}", prefix.len() + body.len() + suffix.len(), max_file_size);
    //                         out_file.flush().context("error flushing file")?;

    //                         part_id += 1;

    //                         let file_name = format!("part-{:0>4}.mbox", part_id);
    //                         let file_path = if folder_name.clone().starts_with("/") {
    //                             PathBuf::from_str("/")
    //                                 .unwrap()
    //                                 .join(folder_name.clone())
    //                                 .join(file_name.clone())
    //                         } else {
    //                             current_dir()
    //                                 .unwrap()
    //                                 .join(folder_name.clone())
    //                                 .join(file_name.clone())
    //                         };

    //                         log::debug!("Creating part {}", file_path.to_string_lossy());
    //                         out_file = File::create(file_path).context("Unable to open file")?;

    //                         bytes_written = 0;
    //                     }

    //                     out_file
    //                         .write(prefix)
    //                         .context("unable to write file prefix")?;
    //                     out_file.write(body).context("error writing data to file")?;
    //                     out_file
    //                         .write(suffix)
    //                         .context("unable to write file suffix")?;
    //                     log::debug!("{} bytes message added", prefix.len() + body.len() + suffix.len());

    //                     bytes_written += prefix.len() + body.len() + suffix.len();
    //                 }
    //             }

    //             message_id += batch_size;
    //         }

    //         out_file.flush().context("error flushing file")?;
    //     } else {
    //         loop {
    //             let sequence_set = format!("{message_id}:{}", message_id + batch_size - 1);
    //             log::info!("Querying {sequence_set}");

    //             let messages_stream = imap_session
    //                 .fetch(sequence_set, "RFC822")
    //                 .await
    //                 .context("error getting messages")?;
    //             let messages: Vec<_> = messages_stream.try_collect().await?;

    //             if messages.len() == 0 {
    //                 log::debug!("No more messages");
    //                 break;
    //             } else {
    //                 let mut current_message_id = message_id - 1;

    //                 for message in messages {
    //                     current_message_id += 1;
    //                     let body = message.body().context("message did not have a body!")?;
    //                     let body = std::str::from_utf8(body).ok();
    //                     if body.is_none() {
    //                         log::warn!("Message {} had invalid UTF-8", current_message_id);
    //                         continue;
    //                     }
    //                     let body = body.unwrap().as_bytes();

    //                     let file_name = format!("{:0>8}.eml", current_message_id);
    //                     let file_path = if folder_name.clone().starts_with("/") {
    //                         PathBuf::from_str("/")
    //                             .unwrap()
    //                             .join(folder_name.clone())
    //                             .join(file_name.clone())
    //                     } else {
    //                         current_dir()
    //                             .unwrap()
    //                             .join(folder_name.clone())
    //                             .join(file_name.clone())
    //                     };
    //                     fs::write(file_path, body)
    //                         .context("unable to write file")
    //                         .context("unable to save *.eml file")?;
    //                     log::debug!("{} bytes eml message added", body.len());
    //                 }
    //             }

    //             message_id += batch_size;
    //         }
    //     }
    // }

    log::info!("Done in {:?}", start.elapsed());

    Ok(())
}
