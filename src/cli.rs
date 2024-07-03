use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct NubeSyncCommand {
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

#[derive(Debug, Parser)]
pub enum SubCommand {
    Sync(SyncSubCommand),
    Clear(ClearSubCommand),
}

#[derive(Debug, Parser)]
pub struct SyncSubCommand {
    /// Remote location of the files in the host server.
    #[clap(value_parser)]
    remote_location: PathBuf,

    /// Directory where the files will be downloaded. If not set, will try to use the out dir in config.
    #[clap(long)]
    out: Option<PathBuf>,

    /// Location of the config file. If not set, try to load `./nube-sync.config.toml`
    #[clap(long)]
    config: Option<PathBuf>,
}

impl SyncSubCommand {
    pub fn remote_location(&self) -> String {
        let mut str_location = self.remote_location.display().to_string();
        if !str_location.ends_with('/') {
            str_location.push_str("/");
        }

        str_location
    }

    pub fn out_dir(&self) -> Option<&PathBuf> {
        self.out.as_ref()
    }

    pub fn config_location(&self) -> PathBuf {
        self.config.clone().unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("nube-sync.config.toml")
        })
    }
}

#[derive(Debug, Parser)]
pub struct ClearSubCommand {
    /// Delete all files in the out directory if finds a `.sync` file inside.
    #[clap(value_parser)]
    pub out: PathBuf,
}
