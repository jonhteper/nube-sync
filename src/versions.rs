use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use reqwest_dav::list_cmd::ListEntity;
use serde::{Deserialize, Serialize};

use crate::result::AppResult;

#[derive(Debug, Clone)]
pub struct VersionService {
    version: Version,
    entities: Vec<ListEntity>,
}

impl VersionService {
    pub fn init(local: LocalVersion, entities: Vec<ListEntity>) -> Self {
        let server_version = ServerVersion::from_entities(&entities);
        let version = Version::new(&server_version, &local);

        Self { version, entities }
    }

    pub fn entities_to_download(&self) -> Vec<ListEntity> {
        let to_download = self.version.files_to_download();

        self.entities
            .clone()
            .into_iter()
            .filter(|entity| {
                let href = match entity {
                    ListEntity::File(file) => &file.href,
                    ListEntity::Folder(folder) => &folder.href,
                };

                to_download.contains(href)
            })
            .collect()
    }

    pub fn version(&self) -> &Version {
        &self.version
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
        let file = File::open(parent_dir.join(".sync"));
        if let Err(err) = &file {
            if err.kind() == std::io::ErrorKind::NotFound {
                return Ok(LocalVersion {
                    paths: HashMap::new(),
                });
            }
        }

        let mut file_content = String::new();
        let _ = file.unwrap().read_to_string(&mut file_content)?;
        let last_version = serde_json::from_str(&file_content)?;

        Ok(last_version)
    }

    pub fn save_in_file(&self, parent_dir: &Path) -> AppResult<()> {
        let path = parent_dir.join(".sync");
        let mut file = File::create(path)?;
        let json_version = serde_json::to_string_pretty(&self)?;

        file.write_all(json_version.as_bytes())?;

        Ok(())
    }

    pub fn add(&mut self, href: String, path: PathBuf) {
        self.paths.insert(href, path);
    }

    pub fn remove(&mut self, href: &String) -> Option<PathBuf> {
        self.paths.remove(href)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Status {
    Local,
    Server,
    Both,
}

#[derive(Debug, Clone)]
pub struct ServerVersion {
    pub paths: Vec<String>,
}

impl ServerVersion {
    pub fn from_entities(files: &[ListEntity]) -> Self {
        let mut paths = Vec::new();
        for f in files {
            let href = match f {
                ListEntity::File(file) => file.href.clone(),
                ListEntity::Folder(folder) => folder.href.clone(),
            };

            paths.push(href);
        }

        ServerVersion { paths }
    }
}

#[derive(Debug, Clone)]
pub struct Version {
    paths: HashMap<String, Status>,
}

impl Version {
    pub fn new(server: &ServerVersion, local: &LocalVersion) -> Self {
        let mut paths = HashMap::new();
        for href in local.paths.keys() {
            paths.insert(href.clone(), Status::Local);
        }

        for href in &server.paths {
            match paths.get_mut(href) {
                Some(status) => *status = Status::Both,
                None => {
                    paths.insert(href.clone(), Status::Server);
                }
            }
        }

        Version { paths }
    }

    pub fn files_to_remove(&self) -> Vec<String> {
        let mut paths = Vec::new();
        for (href, status) in self.paths.iter() {
            if *status == Status::Local {
                paths.push(href.clone());
            }
        }

        paths
    }

    pub fn files_to_download(&self) -> Vec<String> {
        let mut paths = Vec::new();
        for (href, status) in self.paths.iter() {
            if *status == Status::Server {
                paths.push(href.clone());
            }
        }

        paths
    }
}
