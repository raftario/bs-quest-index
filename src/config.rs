use serde::Deserialize;
use std::path::Path;
use tokio::fs;

#[derive(Deserialize)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub log_level: Option<String>,
}

impl Config {
    pub async fn read<P: AsRef<Path>>(path: P) -> anyhow::Result<&'static Self> {
        let contents = fs::read_to_string(path).await?;
        let config = serde_json::from_str(&contents)?;
        Ok(Box::leak(Box::new(config)))
    }
}
