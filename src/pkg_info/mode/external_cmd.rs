use std::{borrow::Cow, process::Output};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::{pkg_info::VersionContent, version::RawVersion, VersionedArchEntry};

use super::VersionComponent;

pub fn process_output(output: Output) -> anyhow::Result<VersionComponent> {
    let string = String::from_utf8(output.stdout).context("Invalid utf-8 in output")?;
    let info =
        serde_json::from_str::<OutputVersionInfo>(&string).context("Failed to parse output")?;

    Ok((
        RawVersion::from(Cow::Owned(info.version.to_owned())),
        info.assets
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
    ))
}

#[derive(Debug, Serialize, Deserialize)]
struct OutputVersionInfo<'a> {
    #[serde(borrow)]
    version: &'a str,
    #[serde(borrow)]
    assets: VersionContent<'a>,
}
