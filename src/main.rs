use std::path::PathBuf;

use clap::Parser;
use config::Config;
use reqwest_dav::re_exports::tokio;

use result::AppResult;
use sync_service::SyncService;

mod cli;
mod config;
mod conn_retry;
mod result;
mod sync_service;
mod versions;

#[cfg(feature = "version_migration")]
mod old_version;

#[tokio::main]
async fn main() {
    let cmd_options = cli::NubeSyncCommand::parse();

    match cmd_options.cmd {
        cli::SubCommand::Sync(cmd) => sync(cmd).await,
        cli::SubCommand::Clear(cmd) => clear(&cmd.out),

        #[cfg(feature = "version_migration")]
        cli::SubCommand::Migrate(cmd) => migrate(cmd).await,
    }
}

fn sync_service(config_path: PathBuf, out_dir: Option<&PathBuf>) -> AppResult<SyncService> {
    let mut config = Config::load_from_file(config_path)?;

    if let Some(out_dir) = out_dir {
        config.out_dir.clone_from(out_dir);
    }

    SyncService::init(config)
}

async fn sync(cmd: cli::SyncSubCommand) {
    let mut sync =
        sync_service(cmd.config_location(), cmd.out_dir()).expect("Error starting sync service");

    sync.sync(&cmd.remote_location())
        .await
        .expect("Error syncing");
}

fn clear(out_dir: &PathBuf) {
    SyncService::clear_out_dir(out_dir).expect("Error clearing dir");
}

#[cfg(feature = "version_migration")]
async fn migrate(cmd: cli::SyncSubCommand) {
    let mut config =
        Config::load_from_file(cmd.config_location()).expect("Error loading config file");

    if let Some(out_dir) = cmd.out_dir() {
        config.out_dir.clone_from(out_dir);
    }

    let mut sync = SyncService::init_with_empty_db(config).expect("Error starting sync service");

    println!("Local db migration...");

    sync.migrate_db(&cmd.remote_location())
        .await
        .expect("Error migrating db");
}
