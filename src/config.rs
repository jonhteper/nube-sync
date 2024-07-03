use std::{env, fs::File, io::Read, path::PathBuf};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::result::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host: Url,
    pub username: String,
    pub password: String,
    pub out_dir: PathBuf,
    pub black_list: Vec<String>,
}

impl Config {
    pub fn load_from_file(path: Option<PathBuf>) -> AppResult<Self> {
        let path = path.unwrap_or(env::current_dir()?.join("nube-sync.config.toml"));
        let mut file_content = String::new();
        let _ = File::open(path)?.read_to_string(&mut file_content)?;
        let config = toml::from_str(&file_content)?;

        Ok(config)
    }
}
