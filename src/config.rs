use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ImapServerConfig {
    pub server: String,
    pub port: Option<u16>,
    pub folder_delimiter: Option<char>,
    pub folder_name_mappings: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ImapConfig {
    pub pull: Option<ImapServerConfig>,
    pub push: Option<ImapServerConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CalDAVServerConfig {
    pub server: String,
    pub port: Option<u16>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CalDAVConfig {
    pub pull: Option<CalDAVServerConfig>,
    pub push: Option<CalDAVServerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub imap: Option<ImapConfig>,
    pub caldav: Option<CalDAVConfig>
}