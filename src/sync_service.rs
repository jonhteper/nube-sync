use std::{fs::File, io::Write, path::PathBuf};

use ::tokio::fs::DirBuilder;
use reqwest_dav::{
    list_cmd::{ListEntity, ListFile, ListFolder},
    Auth, Client, ClientBuilder, Depth,
};
use url::Url;

#[cfg(feature = "version_migration")]
use getset::Getters;
#[cfg(feature = "version_migration")]
use named_ctor::NamedCtor;

use crate::{
    config::Config,
    conn_retry::DEFAULT_CONN_RETRY,
    result::AppResult,
    versions::{Href, LocalFile, LocalVersion, VersionService},
};

#[cfg_attr(feature = "version_migration", derive(Getters, NamedCtor))]
#[cfg_attr(feature = "version_migration", getset(get = "pub"))]
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
        println!("sync location: {}...", remote_dir);
        let server_files = DEFAULT_CONN_RETRY
            .execute_with_retries(|| self.client.list(remote_dir, Depth::Infinity))
            .await?;
        let version_service = VersionService::init(self.local_version.clone(), server_files);

        self.delete_locals(version_service.version().files_to_remove())?;

        let to_sycn_files = version_service.entities_to_download();

        self.apply_sync(remote_dir, to_sycn_files).await?;

        self.local_version.save_in_file(&self.config.out_dir)
    }

    /// Remove files deleted on the server.
    fn delete_locals(&mut self, to_detele: Vec<String>) -> AppResult<()> {
        let mut folders_to_delete = Vec::new();
        for href in to_detele {
            let file = self.local_version.remove(&href).unwrap();
            let path = &file.path;

            if path.is_dir() {
                folders_to_delete.push(path.clone());
                println!("deleting local folder: {}", path.display());
                std::fs::remove_dir_all(path)?;

                continue;
            }

            if path.is_file() {
                println!("deleting local file: {}", path.display());
                std::fs::remove_file(path)?;
            }
        }

        Ok(())
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

                    self.create_dir(&folder, remote_dir).await?;
                }
            }
        }

        Ok(())
    }

    async fn create_dir(&mut self, folder: &ListFolder, remote_dir: &str) -> AppResult<()> {
        let base_url = Url::parse(format!("{}{}", self.config.host, remote_dir).as_str())?;
        let url_path = base_url.path();
        let remote_dir_path = &folder.href[url_path.len()..];
        if remote_dir_path.is_empty() {
            return Ok(());
        }

        let remote_dir_path = urlencoding::decode(remote_dir_path)?;
        println!("dir: {}", remote_dir_path);

        let path = self.config.out_dir.clone().join(remote_dir_path.as_ref());

        DirBuilder::new().create(&path).await?;

        self.local_version.add(
            folder.href.clone(),
            LocalFile {
                path,
                is_dir: true,
                last_modified: None,
            },
        );

        Ok(())
    }

    fn is_in_black_list(&self, href: &str) -> AppResult<bool> {
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

    pub fn define_paths(&self, remote_dir: &str, file_href: &Href) -> AppResult<DavPaths> {
        let base_url = Url::parse(format!("{}{}", self.config.host, remote_dir).as_str())?;
        let url_path = base_url.path();
        let remote_path_str = &file_href[url_path.len()..];

        let decoded_remote_path = urlencoding::decode(remote_path_str)?;
        let remote_path = PathBuf::from(decoded_remote_path.as_ref());

        Ok(DavPaths {
            local: self.config.out_dir.join(&remote_path),
            remote: remote_path,
        })
    }

    async fn download_file(&mut self, file: &ListFile, remote_dir: &str) -> AppResult<()> {
        let download_uri = &file.href[self.config.host.path().len()..];
        let dowloaded = DEFAULT_CONN_RETRY
            .execute_with_retries(|| self.client.get(download_uri))
            .await?
            .bytes()
            .await?;
        //let dowloaded = self.client.get(download_uri).await?.bytes().await?;

        let paths = self.define_paths(remote_dir, &file.href)?;
        println!("downloading: {}...", paths.remote.display());

        let mut local_file = File::create(&paths.local)?;
        local_file.write_all(&dowloaded)?;

        self.local_version.add(
            file.href.clone(),
            LocalFile {
                path: paths.local,
                is_dir: false,
                last_modified: Some(file.last_modified),
            },
        );

        Ok(())
    }

    pub fn clear_out_dir(out_dir: &PathBuf) -> AppResult<()> {
        let db_file_location = out_dir.join(".sync");
        if !db_file_location.is_file() {
            return Ok(());
        }

        let files = std::fs::read_dir(out_dir)?;
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

#[derive(Debug, Clone)]
pub struct DavPaths {
    pub remote: PathBuf,
    pub local: PathBuf,
}
