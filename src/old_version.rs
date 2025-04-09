use std::{collections::HashMap, fs::File, io::Read, path::PathBuf};

use reqwest_dav::{list_cmd::ListEntity, Auth, ClientBuilder, Depth};
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    result::AppResult,
    sync_service::{SyncService, _SyncService},
    versions::{self, Href, LocalFile, _LocalVersion},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalVersion {
    paths: HashMap<Href, PathBuf>,
}

impl LocalVersion {
    /// Search file named `.sync` in `parent_dir` to get the last version of files.
    pub fn load_from_file(parent_dir: &PathBuf) -> AppResult<Self> {
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
}

impl SyncService {
    pub fn init_with_empty_db(config: Config) -> AppResult<SyncService> {
        let reqwest_client = reqwest::ClientBuilder::new().use_rustls_tls().build()?;
        let client = ClientBuilder::new()
            .set_agent(reqwest_client)
            .set_host(config.host.to_string())
            .set_auth(Auth::Basic(
                config.username.clone(),
                config.password.clone(),
            ))
            .build()?;

        let local_version = versions::LocalVersion::from(_LocalVersion {
            files: HashMap::new(),
        });

        let service = SyncService::from(_SyncService {
            client,
            local_version,
            config,
        });

        Ok(service)
    }

    /// try to create the new version of local version db.
    pub async fn migrate_db(&mut self, remote_dir: &str) -> AppResult<()> {
        let old_db = LocalVersion::load_from_file(&self.config().out_dir)?;
        let server_files = self.client().list(remote_dir, Depth::Infinity).await?;
        let mut new_version_files = HashMap::new();

        for f in server_files {
            let (href, file) = match f {
                ListEntity::File(file) => {
                    if old_db.paths.get(&file.href).is_none() {
                        continue;
                    }

                    let paths = self.define_paths(remote_dir, &file.href)?;

                    let local = LocalFile {
                        path: paths.local,
                        is_dir: false,
                        last_modified: Some(file.last_modified),
                    };

                    (file.href.clone(), local)
                }
                ListEntity::Folder(folder) => {
                    if old_db.paths.get(&folder.href).is_none() {
                        continue;
                    }

                    let paths = self.define_paths(remote_dir, &folder.href)?;

                    let local = LocalFile {
                        path: paths.local,
                        is_dir: true,
                        last_modified: None,
                    };

                    (folder.href.clone(), local)
                }
            };

            new_version_files.insert(href, file);
        }

        versions::LocalVersion::from(_LocalVersion {
            files: new_version_files,
        })
        .save_in_file(&self.config().out_dir)
    }
}
