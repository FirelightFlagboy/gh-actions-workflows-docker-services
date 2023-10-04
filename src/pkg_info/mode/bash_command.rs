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

use crate::{ModeGetLatestVersion, VersionedArchEntry};

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
                ("TMP_DIR", tmp_dir.as_os_str().clone()),
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

        let stdout =
            String::from_utf8(output.stdout).context("Output contain invalid utf-8 char")?;
        let output =
            serde_json::from_str::<CommandVersionInfo>(&stdout).context("Fail to parsed output")?;

        Ok(Box::pin(futures::future::ready(Ok((
            output.version.to_string(),
            output
                .assets
                .iter()
                .map(|(k, v)| {
                    (
                        *k,
                        VersionedArchEntry {
                            filename: Cow::Owned(v.filename.to_string()),
                            download_url: v.download_url.clone(),
                            digest: v.digest.to_owned(),
                        },
                    )
                })
                .collect(),
        )))))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CommandVersionInfo<'a> {
    #[serde(borrow)]
    version: &'a str,
    #[serde(borrow)]
    assets: crate::pkg_info::Version<'a>,
}
