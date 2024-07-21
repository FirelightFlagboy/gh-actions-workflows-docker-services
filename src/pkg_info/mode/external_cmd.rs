use std::{borrow::Cow, process::Output};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::{pkg_info::VersionContent, version::Version, PkgOption, VersionedArchEntry};

pub fn process_output(
    option: &PkgOption,
    output: Output,
) -> anyhow::Result<(Version<'static>, VersionContent<'static>)> {
    let string = String::from_utf8(output.stdout).context("Invalid utf-8 in output")?;
    let info =
        serde_json::from_str::<OutputVersionInfo>(&string).context("Failed to parse output")?;

    Ok((
        Version::from_raw_str(Cow::Owned(info.version.to_owned()), option.strip_v_prefix),
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
