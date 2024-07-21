use std::{path::Path, process::Stdio};

use anyhow::Context;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::{ModeGetLatestVersion, PkgOption};

use super::VersionComponent;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct ReleaseHandler<'a> {
    pub document_url: url::Url,
    #[serde(borrow)]
    pub script_path: &'a Path,
}

impl<'a> ModeGetLatestVersion for ReleaseHandler<'a> {
    async fn get_latest_version(
        &self,
        option: &PkgOption,
        _tmp_dir: &Path,
        _in_test_mode: bool,
    ) -> anyhow::Result<VersionComponent> {
        let http_client = crate::reqwest_utils::prepare_http_client_json()
            .build()
            .context("Failed to build http client")?;

        let mut cmd = Command::new("jq");
        cmd.arg("--from-file")
            .arg(self.script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        log::info!("Requesting json document ...");
        let document_resp = http_client.get(self.document_url.clone()).send().await?;
        anyhow::ensure!(
            document_resp.status() == reqwest::StatusCode::OK,
            "Invalid response status"
        );
        let mut document_reader = tokio_util::io::StreamReader::new(
            document_resp
                .bytes_stream()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        );

        log::info!("Spawning jq command ...");
        let mut process = cmd.spawn()?;

        let mut stdin = process
            .stdin
            .take()
            .expect("Missing stdin to send document data");

        log::info!("Streaming json document to jq ...");
        let bytes_streamed = tokio::io::copy(&mut document_reader, &mut stdin)
            .await
            .context("Failed to stream json document to jq's stdin")?;
        drop(stdin);

        log::trace!("Streamed {} bytes to jq's stdin", bytes_streamed);

        log::info!("Waiting for jq to finish ...");
        let output = process.wait_with_output().await?;

        anyhow::ensure!(output.status.success(), "Jq command has failed");
        super::external_cmd::process_output(&option, output)
    }
}
