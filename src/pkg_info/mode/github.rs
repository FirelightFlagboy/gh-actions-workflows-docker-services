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
        option: &PkgOption,
        tmp_dir: &Path,
        in_test_mode: bool,
    ) -> anyhow::Result<VersionComponent> {
        let github_token = std::env::var("GITHUB_TOKEN")
            .context("Cannot retrieve github token from env value `GITHUB_TOKEN`")?;
        let http_client = build_http_client(&github_token)?;

        log::info!("Fetching latest release ...");
        let release =
            get_release(&http_client, self.repository_path, option.allow_prerelease).await?;
        if in_test_mode {
            let path = tmp_dir.join("latest-release.json");
            log::trace!("Dump release json to {}", path.display());
            serde_json::to_writer(std::fs::File::create(path).unwrap(), &release).unwrap();
        }
        log::info!(
            "Latest release {}, found {} asset(s)",
            release.name,
            release.assets.len()
        );

        let assets = self.get_assets_for_arch(release.assets);
        log::debug!("Collected assets: {assets:#?}");
        log::info!("Calculating checksum for {} asset(s) ...", assets.len());
        let assets_with_checksum = get_checksum_for_assets(&http_client, assets).await?;
        log::trace!("Calculated checksums: {assets_with_checksum:#?}");

        Ok((
            RawVersion::from(release.name),
            VersionContent(assets_with_checksum),
        ))
    }
}

impl<'a> ReleaseHandler<'a> {
    fn get_assets_for_arch<'b>(
        &self,
        assets: Vec<GithubAsset<'b>>,
    ) -> HashMap<Arch, GithubAsset<'b>> {
        self.arch_asset_patterns
            .iter()
            .filter_map(|(arch, pattern)| {
                assets
                    .iter()
                    .find(|asset| pattern.is_match(&asset.name))
                    .map(|asset| (*arch, asset.clone()))
            })
            .collect()
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

async fn get_release(
    http_client: &reqwest::Client,
    repository_path: &str,
    allow_prerelease: bool,
) -> anyhow::Result<GithubRelease<'static>> {
    const RELEASE_PER_PAGE: usize = 10;

    if !allow_prerelease {
        return get_latest_release(http_client, repository_path).await;
    }
    let url = format!("https://api.github.com/repos/{repository_path}/releases");
    let mut page = 1;
    loop {
        let res = http_client
            .get(&url)
            .query(&[("per_page", RELEASE_PER_PAGE), ("page", page)])
            .send()
            .await?;

        anyhow::ensure!(
            res.status() == reqwest::StatusCode::OK,
            "Invalid response status: {}",
            res.status(),
        );

        let raw_body = res.text().await?;
        let releases = serde_json::from_str::<Vec<GithubRelease>>(&raw_body)?;
        for release in releases {
            if !release.draft && release.prerelease == allow_prerelease {
                return Ok(release.to_owned());
            }
        }
        page += 1;
    }
}

async fn get_latest_release(
    http_client: &reqwest::Client,
    repository_path: &str,
) -> anyhow::Result<GithubRelease<'static>> {
    let res = http_client
        .get(format!(
            "https://api.github.com/repos/{repository_path}/releases/latest",
        ))
        .send()
        .await?;

    anyhow::ensure!(
        res.status() == reqwest::StatusCode::OK,
        "Invalid response status: {}",
        res.status(),
    );

    let raw_body = res.text().await?;
    Ok(serde_json::from_str::<GithubRelease>(&raw_body)?.to_owned())
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
    #[serde(borrow)]
    name: Cow<'a, str>,
    #[serde(borrow)]
    tag_name: Cow<'a, str>,
    prerelease: bool,
    draft: bool,
    #[serde(borrow)]
    assets: Vec<GithubAsset<'a>>,
}

impl GithubRelease<'_> {
    fn to_owned(&self) -> GithubRelease<'static> {
        GithubRelease {
            name: Cow::Owned(self.name.clone().into()),
            tag_name: Cow::Owned(self.tag_name.clone().into()),
            prerelease: self.prerelease,
            draft: self.draft,
            assets: self.assets.iter().map(GithubAsset::to_owned).collect(),
        }
    }
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Serialize, Deserialize, Clone)]
pub struct GithubAsset<'a> {
    #[serde(borrow)]
    name: Cow<'a, str>,
    size: usize,
    browser_download_url: url::Url,
}

impl GithubAsset<'_> {
    fn to_owned(&self) -> GithubAsset<'static> {
        GithubAsset {
            name: Cow::Owned(self.name.clone().into()),
            size: self.size,
            browser_download_url: self.browser_download_url.clone(),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_get_asset_for_arch() {
        let wanted_asset = GithubAsset {
            name: "asset.foo".into(),
            size: 0,
            browser_download_url: "http://asset.com".parse().unwrap(),
        };
        let assets = vec![
            wanted_asset.clone(),
            GithubAsset {
                name: "asset.bar".into(),
                size: 0,
                browser_download_url: "http://asset.com".parse().unwrap(),
            },
        ];
        let handler = ReleaseHandler {
            repository_path: "",
            arch_asset_patterns: ArchAssetPattern::from_iter(vec![(
                Arch::Amd64,
                regex::Regex::new(r"asset\.foo").unwrap(),
            )]),
        };

        let got_asset = handler.get_assets_for_arch(assets);

        assert_eq!(got_asset.len(), 1);
        assert_eq!(got_asset.get(&Arch::Amd64), Some(&wanted_asset));
    }

    #[test]
    // https://github.com/FirelightFlagboy/gh-actions-workflows-docker-services/issues/53
    fn can_use_same_asset_on_different_arch() {
        let foo_asset = GithubAsset {
            name: "asset.foo".into(),
            size: 0,
            browser_download_url: "http://asset.com".parse().unwrap(),
        };
        let assets = vec![foo_asset.clone()];
        let handler = ReleaseHandler {
            repository_path: "",
            arch_asset_patterns: ArchAssetPattern::from_iter(vec![
                (Arch::Amd64, regex::Regex::new(r"asset\.foo").unwrap()),
                (Arch::Arm64, regex::Regex::new(r"asset\.foo").unwrap()),
            ]),
        };

        let got_asset = handler.get_assets_for_arch(assets);

        assert_eq!(got_asset.len(), 2);
        assert_eq!(
            got_asset,
            HashMap::from_iter([(Arch::Amd64, foo_asset.clone()), (Arch::Arm64, foo_asset),])
        );
    }
}
