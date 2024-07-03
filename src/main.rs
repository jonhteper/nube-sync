use std::{env, path::PathBuf};

use config::Config;
use reqwest_dav::re_exports::tokio;

use sync_service::SyncService;

mod config;
mod result;
mod sync_service;
mod versions;

#[tokio::main]
async fn main() {
    self_client().await;
}

async fn self_client() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("nube-sync.config.toml");
    let config = Config::load_from_file(Some(path)).expect("Error loading config");
    let mut sync = SyncService::init(config).expect("Error inicializando servicio");
}
