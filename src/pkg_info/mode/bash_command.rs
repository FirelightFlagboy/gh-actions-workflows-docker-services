//! Mode when executing a bash command to retrieve the latest version.
//!
//! The command will be provided with those environment variable:
//!
//! | Name      | Description                         |
//! | --------- | ----------------------------------- |
//! | `TEST`    | The script is executed in test mode |
//! | `TMP_DIR` | Path to a temporary folder          |

use std::{
    borrow::Cow,
    ffi::OsStr,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::ModeGetLatestVersion;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct ReleaseHandler<'a> {
    #[serde(borrow)]
    pub command: Cow<'a, str>,
}

impl<'a> ModeGetLatestVersion for ReleaseHandler<'a> {
    fn get_latest_version(
        &self,
        tmp_dir: &Path,
        in_test_mode: bool,
    ) -> anyhow::Result<
        super::BoxedFuture<anyhow::Result<(String, crate::pkg_info::Version<'static>)>>,
    > {
        let mut cmd = Command::new("bash");

        cmd.args(["-c", &self.command])
            .envs([
                (
                    "TEST",
                    OsStr::new(if in_test_mode { "true" } else { "false" }),
                ),
                ("TMP_DIR", tmp_dir.as_os_str()),
            ])
            .env_remove("GITHUB_TOKEN")
            .stderr(Stdio::inherit());
        log::trace!("Configure the command: {cmd:#?}");

        log::info!("Executing the command ...");
        let output = cmd.output().context("Spawning the process")?;

        anyhow::ensure!(
            output.status.success(),
            "Bash command {} has failed",
            self.command
        );

        let res = super::external_cmd::process_output(output);

        Ok(Box::pin(futures::future::ready(res)))
    }
}
