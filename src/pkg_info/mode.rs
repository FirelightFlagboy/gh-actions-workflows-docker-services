mod bash_command;
mod external_cmd;
mod github;
mod jq_script;

use std::path::Path;

use futures::Future;
use serde::{Deserialize, Serialize};

use super::VersionContent;

pub use bash_command::ReleaseHandler as BashCmdReleaseHandler;
pub use github::ReleaseHandler as GithubReleaseHandler;
pub use jq_script::ReleaseHandler as JqScriptReleaseHandler;

use crate::{version::Version, PkgOption};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", tag = "mode")]
pub enum Mode<'a> {
    GithubRelease(#[serde(borrow)] github::ReleaseHandler<'a>),
    BashCommand(#[serde(borrow)] bash_command::ReleaseHandler<'a>),
    JqScript(#[serde(borrow)] jq_script::ReleaseHandler<'a>),
}

pub type VersionComponent = (Version<'static>, VersionContent<'static>);

impl<'a> Mode<'a> {
    pub async fn get_latest_version(
        &self,
        option: &PkgOption,
        tmp_dir: &Path,
        in_test_mode: bool,
    ) -> anyhow::Result<VersionComponent> {
        match self {
            Mode::GithubRelease(gh_release) => {
                gh_release
                    .get_latest_version(option, tmp_dir, in_test_mode)
                    .await
            }
            Mode::BashCommand(command) => {
                command
                    .get_latest_version(option, tmp_dir, in_test_mode)
                    .await
            }
            Mode::JqScript(script) => {
                script
                    .get_latest_version(option, tmp_dir, in_test_mode)
                    .await
            }
        }
    }
}

pub trait ModeGetLatestVersion {
    fn get_latest_version(
        &self,
        option: &PkgOption,
        tmp_dir: &Path,
        in_test_mode: bool,
    ) -> impl Future<Output = anyhow::Result<VersionComponent>>;
}
