use std::{borrow::Cow, collections::HashMap, fmt::Debug, ops::Deref, path::Path};

use anyhow::Context;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};

use crate::{
    pkg_info::{Arch, Digest, VersionContent, VersionedArchEntry},
    version::RawVersion,
    PkgOption,
};

use super::{ModeGetLatestVersion, VersionComponent};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseHandler<'a> {
    pub repository_path: &'a str,
    pub arch_asset_patterns: ArchAssetPattern,
}

impl<'a> ModeGetLatestVersion for ReleaseHandler<'a> {
    async fn get_latest_version(
        &self,
        _option: &PkgOption,
        tmp_dir: &Path,
        in_test_mode: bool,
    ) -> anyhow::Result<VersionComponent> {
        let github_token = std::env::var("GITHUB_TOKEN")
            .context("Cannot retrieve github token from env value `GITHUB_TOKEN`")?;
        let http_client = build_http_client(&github_token)?;

        log::info!("Fetching latest release ...");
        let raw_body = get_raw_latest_release(&http_client, self.repository_path).await?;
        if in_test_mode {
            let path = tmp_dir.join("latest-release.json");
            log::trace!("Dump release json to {}", path.display());
            std::fs::write(path, &raw_body).unwrap();
        }
        let latest_release = serde_json::from_str::<GithubRelease>(&raw_body)?;
        log::info!(
            "Latest release {}, found {} asset(s)",
            latest_release.name,
            latest_release.assets.len()
        );

        let assets = latest_release
            .assets
            .into_iter()
            .filter_map(|asset| {
                self.arch_asset_patterns
                    .iter()
                    .find(|(_arch, pattern)| pattern.is_match(&asset.name))
                    .map(|(arch, _pattern)| (*arch, asset))
            })
            .collect::<HashMap<Arch, GithubAsset>>();
        log::debug!("Collected assets: {assets:#?}");
        log::info!("Calculating checksum for {} asset(s) ...", assets.len());
        let assets_with_checksum = get_checksum_for_assets(&http_client, assets).await?;
        log::trace!("Calculated checksums: {assets_with_checksum:#?}");

        Ok((
            RawVersion::from(Cow::Owned(latest_release.name.into())),
            VersionContent(assets_with_checksum),
        ))
    }
}

fn build_http_client(github_token: &str) -> anyhow::Result<reqwest::Client> {
    crate::reqwest_utils::prepare_http_client_json()
        .default_headers(HeaderMap::from_iter([
            (
                AUTHORIZATION,
                format!("Bearer {github_token}")
                    .parse()
                    .context("Cannot convert github token into header value")?,
            ),
            (
                HeaderName::from_static("x-github-api-version"),
                HeaderValue::from_static("2022-11-28"),
            ),
        ]))
        .build()
        .context("Failed to build HTTP client")
}

async fn get_checksum_for_assets<'a>(
    http_client: &reqwest::Client,
    assets: HashMap<Arch, GithubAsset<'a>>,
) -> anyhow::Result<HashMap<Arch, VersionedArchEntry<'a>>> {
    use futures::{FutureExt, TryStreamExt};
    use sha2::Digest as Sha2Digest;

    let responses_to_collect = assets.into_iter().map(|(arch, asset)| {
        http_client
            .get(asset.browser_download_url.clone())
            .send()
            .map(move |res_resp| {
                res_resp.map_err(anyhow::Error::from).and_then(|response| {
                    anyhow::ensure!(
                        response.status() == reqwest::StatusCode::OK,
                        "Invalid response status code for asset {}",
                        asset.name
                    );

                    Ok((arch, asset, response))
                })
            })
    });
    let responses = futures::future::try_join_all(responses_to_collect).await?;
    let checksum_to_collect = responses.into_iter().map(|(arch, asset, response)| {
        response
            .bytes_stream()
            .try_fold(
                (0_usize, sha2::Sha512::new()),
                |(size, mut hash), chunk| async move {
                    hash.update(&chunk);
                    Ok((size + chunk.len(), hash))
                },
            )
            .map(move |res| {
                res.map_err(anyhow::Error::from)
                    .and_then(|(dl_size, hash)| {
                        anyhow::ensure!(
                            dl_size == asset.size,
                            "Invalid download size for asset {}",
                            asset.name
                        );
                        let res = hash.finalize();
                        Ok((
                            arch,
                            VersionedArchEntry {
                                filename: asset.name,
                                download_url: asset.browser_download_url,
                                digest: Digest::Sha512(Cow::Owned(bytes_to_hex_str(&res))),
                            },
                        ))
                    })
            })
    });
    let assets_with_checksums = futures::future::try_join_all(checksum_to_collect).await?;
    Ok(assets_with_checksums.into_iter().collect())
}

fn bytes_to_hex_str(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let mut res = String::with_capacity(bytes.len() * 2);

    bytes
        .iter()
        .for_each(|byte| write!(&mut res, "{byte:02x}").unwrap());

    res
}

async fn get_raw_latest_release(
    http_client: &reqwest::Client,
    repository_path: &str,
) -> anyhow::Result<String> {
    let res = http_client
        .get(format!(
            "https://api.github.com/repos/{}/releases/latest",
            repository_path
        ))
        .send()
        .await?;

    anyhow::ensure!(
        res.status() == reqwest::StatusCode::OK,
        "Invalid response status: {}",
        res.status(),
    );

    Ok(res.text().await?)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ArchAssetPattern(#[serde(with = "arch_pattern_map")] HashMap<Arch, regex::Regex>);

impl PartialEq for ArchAssetPattern {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        self.iter().all(|(key, value)| {
            other
                .get(key)
                .map_or(false, |v| value.as_str() == v.as_str())
        })
    }
}

impl Eq for ArchAssetPattern {}

impl Deref for ArchAssetPattern {
    type Target = HashMap<Arch, regex::Regex>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for ArchAssetPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.0.iter().map(|(k, v)| (k, v.as_str())))
            .finish()
    }
}

impl FromIterator<(Arch, regex::Regex)> for ArchAssetPattern {
    fn from_iter<T: IntoIterator<Item = (Arch, regex::Regex)>>(iter: T) -> Self {
        Self(HashMap::from_iter(iter))
    }
}

mod arch_pattern_map {
    use std::{borrow::Cow, collections::HashMap};

    use itertools::Itertools;
    use serde::{
        de::{Deserializer, Error, Visitor},
        ser::Serializer,
    };

    use crate::pkg_info::Arch;

    pub fn serialize<S>(map: &HashMap<Arch, regex::Regex>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_map(
            map.iter()
                .sorted_by_key(|(k, _v)| *k)
                .map(|(k, v)| (k, v.as_str())),
        )
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<Arch, regex::Regex>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MapVisitor;

        impl<'de> Visitor<'de> for MapVisitor {
            type Value = HashMap<Arch, regex::Regex>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "A map of Arch as key and GlobPattern as value")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut res = Self::Value::default();
                while let Some((k, v)) = map.next_entry::<Arch, Cow<'_, str>>()? {
                    let v = regex::Regex::new(&v).map_err(A::Error::custom)?;
                    res.insert(k, v);
                }
                Ok(res)
            }
        }

        deserializer.deserialize_map(MapVisitor)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GithubRelease<'a> {
    name: Cow<'a, str>,
    assets: Vec<GithubAsset<'a>>,
}

#[derive(Serialize, Deserialize)]
pub struct GithubAsset<'a> {
    name: Cow<'a, str>,
    size: usize,
    browser_download_url: url::Url,
}

impl<'a> Debug for GithubAsset<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GithubAsset")
            .field("name", &self.name)
            .field("size", &self.size)
            .field("browser_download_url", &self.browser_download_url.as_str())
            .finish()
    }
}
