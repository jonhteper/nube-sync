use std::path::PathBuf;

use clap::Parser;
use config::Config;
use reqwest_dav::re_exports::tokio;

use sync_service::SyncService;

mod cli;
mod config;
mod result;
mod sync_service;
mod versions;

#[tokio::main]
async fn main() {
    let cmd_options = cli::NubeSyncCommand::parse();

    match cmd_options.cmd {
        cli::SubCommand::Sync(cmd) => sync(cmd).await,
        cli::SubCommand::Clear(cmd) => clear(&cmd.out),
    }
}

async fn sync(cmd: cli::SyncSubCommand) {
    let config_path = cmd.config_location();
    let mut config = Config::load_from_file(config_path).expect("Error loading config");

    if let Some(out_dir) = cmd.out_dir() {
        config.out_dir = out_dir.clone();
    }

    let mut sync = SyncService::init(config).expect("Error starting sync service");

    sync.sync(&cmd.remote_location())
        .await
        .expect("Error syncing");
}

fn clear(out_dir: &PathBuf) {
    SyncService::clear_out_dir(out_dir).expect("Error clearing dir");
}
