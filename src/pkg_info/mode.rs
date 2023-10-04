mod bash_command;
mod github;

use std::{fmt::Debug, future::Future, path::Path, pin::Pin};

use serde::{Deserialize, Serialize};

use super::Version;

pub use bash_command::ReleaseHandler as BashCmdReleaseHandler;
pub use github::ReleaseHandler as GithubReleaseHandler;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", tag = "mode")]
pub enum Mode<'a> {
    GithubRelease(#[serde(borrow)] github::ReleaseHandler<'a>),
    BashCommand(#[serde(borrow)] bash_command::ReleaseHandler<'a>),
}

pub type BoxedFuture<Output> = Pin<Box<dyn Future<Output = Output>>>;

impl<'a> Mode<'a> {
    pub fn get_latest_version(
        &self,
        tmp_dir: &Path,
        in_test_mode: bool,
    ) -> anyhow::Result<BoxedFuture<anyhow::Result<(String, Version<'static>)>>> {
        match self {
            Mode::GithubRelease(gh_release) => gh_release.get_latest_version(tmp_dir, in_test_mode),
            Mode::BashCommand(command) => command.get_latest_version(tmp_dir, in_test_mode),
        }
    }
}

pub trait ModeGetLatestVersion {
    fn get_latest_version(
        &self,
        tmp_dir: &Path,
        in_test_mode: bool,
    ) -> anyhow::Result<BoxedFuture<anyhow::Result<(String, Version<'static>)>>>;
}
