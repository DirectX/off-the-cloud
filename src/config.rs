use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ImapServerConfig {
    pub server: String,
    pub port: Option<u16>,
    pub folder_delimiter: Option<char>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ImapConfig {
    pub pull: Option<ImapServerConfig>,
    pub push: Option<ImapServerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub imap: Option<ImapConfig>,
}