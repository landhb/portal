use directories::UserDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    pub relay_host: String,
    pub relay_port: u16,
    pub download_location: PathBuf,
}

impl ::std::default::Default for AppConfig {
    fn default() -> Self {
        let hdir = UserDirs::new();

        // select ~/Downloads or /tmp for downloads
        let ddir = match &hdir {
            Some(home) => home.download_dir().map_or("/tmp", |v| v.to_str().unwrap()),
            None => "/tmp",
        };

        Self {
            relay_host: String::from("portal-relay.landhb.dev"),
            relay_port: portal::DEFAULT_PORT,
            download_location: PathBuf::from(ddir),
        }
    }
}
