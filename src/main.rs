use std::{
    collections::HashMap,
    env,
    error::Error,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use ::tokio::fs::DirBuilder;
use reqwest_dav::{
    list_cmd::{ListEntity, ListFile},
    re_exports::tokio,
    Auth, Client, ClientBuilder, Depth,
};
use serde::{Deserialize, Serialize};
use url::Url;

#[tokio::main]
async fn main() {
    self_client().await;
}

pub type AppResult<T> = Result<T, Box<dyn Error>>;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalVersion {
    // <href, in_fs_path>
    paths: HashMap<String, PathBuf>,
}

impl LocalVersion {
    /// Search file named `.sync` in `parent_dir` to get the last version of files.
    pub fn load_from_file(parent_dir: PathBuf) -> AppResult<Self> {
        let mut file_content = String::new();
        let _ = File::open(parent_dir.join(".sync"))?.read_to_string(&mut file_content)?;
        let last_version = serde_json::from_str(&file_content)?;

        Ok(last_version)
    }

    pub fn save_in_file(&self, parent_dir: PathBuf) -> AppResult<()> {
        let path = parent_dir.join(".sync");
        let mut file = File::create(path)?;
        let json_version = serde_json::to_string(&self)?;

        file.write_all(json_version.as_bytes())?;

        Ok(())
    }

    pub fn add(&mut self, href: String, path: PathBuf) {
        self.paths.insert(href, path);
    }
}

pub struct SyncService {
    config: Config,
    client: Client,
    local_version: LocalVersion,
}

impl SyncService {
    pub fn init(config: Config) -> AppResult<SyncService> {
        let client = ClientBuilder::new()
            .set_host(config.host.to_string())
            .set_auth(Auth::Basic(
                config.username.clone(),
                config.password.clone(),
            ))
            .build()?;

        let service = SyncService {
            client,
            local_version: LocalVersion::load_from_file(config.out_dir.clone())?,
            config,
        };

        Ok(service)
    }

    pub async fn sync(&mut self, remote_dir: &str) -> AppResult<()> {
        println!("downloading location: {}...", remote_dir);
        let files = self.client.list(remote_dir, Depth::Infinity).await?;

        self.apply_sync(remote_dir, files).await
    }

    async fn apply_sync(&mut self, remote_dir: &str, files: Vec<ListEntity>) -> AppResult<()> {
        for f in files {
            match f {
                ListEntity::File(file) => {
                    if self.is_in_black_list(&file.href)? {
                        continue;
                    }
                    self.download_file(&file, remote_dir).await?;
                }
                ListEntity::Folder(folder) => {
                    if self.is_in_black_list(&folder.href)? {
                        continue;
                    }

                    let base_url = Url::parse(
                        format!("{}{}", self.config.host.to_string(), remote_dir).as_str(),
                    )?;
                    let url_path = base_url.path();
                    let remote_dir_path = &folder.href[url_path.len()..];
                    if remote_dir_path.is_empty() {
                        continue;
                    }
                    let remote_dir_path = urlencoding::decode(remote_dir_path)?;
                    println!("dir: {}", remote_dir_path);

                    let path = self.config.out_dir.clone().join(remote_dir_path.as_ref());

                    DirBuilder::new().create(&path).await?;

                    self.local_version.add(folder.href.clone(), path);
                }
            }
        }

        self.local_version
            .save_in_file(self.config.out_dir.clone())?;

        Ok(())
    }

    fn is_in_black_list(&self, href: &String) -> AppResult<bool> {
        let decoded_href = urlencoding::decode(href)?.to_string();
        if self.config.black_list.contains(&decoded_href) {
            return Ok(true);
        }

        for excluded_path in &self.config.black_list {
            if decoded_href.contains(excluded_path) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn download_file(&mut self, file: &ListFile, remote_dir: &str) -> AppResult<()> {
        let base_url =
            Url::parse(format!("{}{}", self.config.host.to_string(), remote_dir).as_str())?;
        let url_path = base_url.path();
        let remote_path = &file.href[url_path.len()..];
        let download_uri = &file.href[self.config.host.path().len()..];
        let dowloaded = self.client.get(download_uri).await?.bytes().await?;

        let decoded_remote_path = urlencoding::decode(remote_path)?;
        let path = PathBuf::from(decoded_remote_path.as_ref());
        println!("downloading: {decoded_remote_path}...");

        let local_path = self.config.out_dir.clone().join(path);
        let mut local_file = File::create(&local_path)?;
        local_file.write_all(&dowloaded)?;

        self.local_version.add(file.href.clone(), local_path);

        Ok(())
    }

    pub fn clear_out_dir(&self) -> AppResult<()> {
        let files = std::fs::read_dir(&self.config.out_dir)?;
        for file in files {
            let path = file?.path();
            if path.is_dir() {
                std::fs::remove_dir_all(&path)?;
            } else {
                std::fs::remove_file(&path)?;
            }
        }
        Ok(())
    }
}

async fn self_client() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("nube-sync.config.toml");
    let config = Config::load_from_file(Some(path)).expect("Error loading config");
    let mut sync = SyncService::init(config).expect("Error inicializando servicio");
}
