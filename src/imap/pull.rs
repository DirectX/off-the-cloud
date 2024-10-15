use std::{env::current_dir, fs::{self, File}, io::Write, path::PathBuf, str::FromStr};
use anyhow::Context;
use tokio::net::TcpStream;
use futures::TryStreamExt;

use crate::config::Config;

pub async fn pull(config: &Config, email: String, password: String, mailbox: String, out_dir: String, max_file_size: u64) -> anyhow::Result<()> {
    let imap_config = config.imap.clone().context("IMAP config is not provided in config.yaml")?;
    let imap_server = imap_config.server.clone();
    let imap_port = imap_config.port.unwrap_or(993);
    let imap_addr = (imap_server, imap_port);

    log::debug!("Pulling IMAP for account {email}...");

    let tcp_stream = TcpStream::connect(imap_addr.clone()).await?;
    let tls = async_native_tls::TlsConnector::new();
    let tls_stream = tls.connect(&imap_addr.0, tcp_stream).await?;

    let client = async_imap::Client::new(tls_stream);
    println!("Connected to IMAP server {}:{}", imap_addr.0, imap_addr.1);

    let mut imap_session = client.login(&email, &password).await.map_err(|e| e.0)?;
    println!("Logged in as {}", email);

    imap_session.select(&mailbox).await?;
    println!("{mailbox} selected");

    let folder_name = format!("{out_dir}/{email}/{}", mailbox.to_lowercase());
    fs::create_dir_all(&folder_name)?;

    let mut message_id = 7000u32;
    let mut bytes_written = 0u64;
    
    let mut part_id = 1;
    let file_name = format!("part-{:0>4}.mbox", part_id);
    
    let file_path = if folder_name.clone().starts_with("/") {
        PathBuf::from_str("/")
            .unwrap()
            .join(folder_name.clone())
            .join(file_name.clone())
    } else {
        current_dir().unwrap().join(folder_name.clone()).join(file_name.clone())
    };

    log::debug!("Creating part {}", file_path.to_string_lossy());
    log::debug!("Max file size: {max_file_size}");
    
    let mut out_file = File::create(file_path).expect("Unable to open file");
    loop {
        let sequence_set = format!("{message_id}");

        log::debug!("Querying {sequence_set}");

        let messages_stream = imap_session.fetch(sequence_set, "RFC822").await?;
        let messages: Vec<_> = messages_stream.try_collect().await?;

        if messages.len() == 0 {
            log::debug!("No more messages");
            break;
        } else {
            for message in messages {
                let body = message.body().context("message did not have a body!")?;
                let body = std::str::from_utf8(body)
                    .context("message was not valid utf-8")?
                    .as_bytes();
                out_file.write(body).context("error writing data to file")?;
                out_file.write("\r\n\r\n".as_bytes()).context("error writing data to file")?;
                log::debug!("{} bytes message added", body.len());
            }
        }

        message_id += 100;
    }

    out_file.flush().context("error flushing file")?;
    imap_session.logout().await?;

    log::debug!("Done");

    Ok(())
}