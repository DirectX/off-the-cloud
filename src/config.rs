use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ImapConfig {
    pub server: String,
    pub port: Option<u16>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub imap: Option<ImapConfig>,
}