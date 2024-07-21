mod mode;

use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{Debug, Display, Write},
    ops::{Deref, DerefMut},
};

use serde::{Deserialize, Serialize};
use url::Url;

pub use mode::{
    BashCmdReleaseHandler, GithubReleaseHandler, JqScriptReleaseHandler, Mode, ModeGetLatestVersion,
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PkgInfo<'a> {
    #[serde(flatten, borrow)]
    pub base: Base<'a>,
    #[serde(flatten, borrow)]
    pub mode: Mode<'a>,
    #[serde(flatten)]
    pub option: PkgOption,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Default, Clone, Copy)]
pub struct PkgOption {
    /// Remove the `v` prefix from the version string.
    #[serde(default)]
    pub strip_v_prefix: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Base<'a> {
    #[serde(borrow, rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<&'a str>,
    #[serde(borrow)]
    pub name: &'a str,
    #[serde(borrow, skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<Cow<'a, str>>,
    #[serde(borrow, skip_serializing_if = "Option::is_none")]
    pub versions: Option<Versions<'a>>,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Versions<'a>(
    #[serde(borrow, serialize_with = "crate::serde_utils::ordered_map")]
    HashMap<Cow<'a, str>, VersionContent<'a>>,
);

impl<'a> Deref for Versions<'a> {
    type Target = HashMap<Cow<'a, str>, VersionContent<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for Versions<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> FromIterator<(Cow<'a, str>, VersionContent<'a>)> for Versions<'a> {
    fn from_iter<T: IntoIterator<Item = (Cow<'a, str>, VersionContent<'a>)>>(iter: T) -> Self {
        Self(HashMap::from_iter(iter))
    }
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct VersionContent<'a>(
    #[serde(borrow, serialize_with = "crate::serde_utils::ordered_map")]
    HashMap<Arch, VersionedArchEntry<'a>>,
);

impl<'a> FromIterator<(Arch, VersionedArchEntry<'a>)> for VersionContent<'a> {
    fn from_iter<T: IntoIterator<Item = (Arch, VersionedArchEntry<'a>)>>(iter: T) -> Self {
        Self(HashMap::from_iter(iter))
    }
}

impl<'a> Deref for VersionContent<'a> {
    type Target = HashMap<Arch, VersionedArchEntry<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    Amd64,
    Arm64,
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionedArchEntry<'a> {
    #[serde(borrow)]
    pub filename: Cow<'a, str>,
    pub download_url: Url,
    #[serde(borrow)]
    pub digest: Digest<'a>,
}

impl<'a> Debug for VersionedArchEntry<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VersionedArchEntry")
            .field("filename", &self.filename)
            .field("download_url", &self.download_url.as_str())
            .field("digest", &self.digest)
            .finish()
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(try_from = "&str")]
pub enum Digest<'a> {
    #[serde(borrow)]
    Sha512(Cow<'a, str>),
    #[serde(borrow)]
    Sha256(Cow<'a, str>),
}

impl<'a> Digest<'a> {
    pub fn to_owned(&self) -> Digest<'static> {
        match self {
            Digest::Sha512(v) => Digest::Sha512(Cow::Owned(v.to_string())),
            Digest::Sha256(v) => Digest::Sha256(Cow::Owned(v.to_string())),
        }
    }
}

impl<'a> TryFrom<&'a str> for Digest<'a> {
    type Error = ParseDigestError<'a>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let Some((digest_id, digest)) = value.split_once(':') else {
            return Err(ParseDigestError::MissingSeparator);
        };
        let digest_len = digest.len();
        match digest_id {
            "sha512" => {
                if digest_len == 128 {
                    Ok(Self::Sha512(Cow::Borrowed(digest)))
                } else {
                    Err(ParseDigestError::InvalidDigestSize {
                        got: digest_len,
                        expected: 128,
                    })
                }
            }
            "sha256" => {
                if digest_len == 64 {
                    Ok(Self::Sha256(Cow::Borrowed(digest)))
                } else {
                    Err(ParseDigestError::InvalidDigestSize {
                        got: digest_len,
                        expected: 64,
                    })
                }
            }
            _ => Err(ParseDigestError::UnknownDigest(digest_id)),
        }
    }
}

impl<'a> Display for Digest<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (id, digest) = match self {
            Digest::Sha512(digest) => ("sha512", digest),
            Digest::Sha256(digest) => ("sha256", digest),
        };
        f.write_str(id)?;
        f.write_char(':')?;
        f.write_str(digest)
    }
}

impl<'a> Serialize for Digest<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseDigestError<'a> {
    #[error("Invalid digest size, expected {} but got {}", .expected, .got)]
    InvalidDigestSize { got: usize, expected: usize },
    #[error("Unknown digest `{}`", .0)]
    UnknownDigest(&'a str),
    #[error("Missing separator `:`")]
    MissingSeparator,
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case::sha256(
        "sha256:25f5602ea53a18f4d64208c6d135690ace28cda7b89ef1eeccd2e60e6cce2e03",
        Ok(Digest::Sha256(Cow::Borrowed(
            "25f5602ea53a18f4d64208c6d135690ace28cda7b89ef1eeccd2e60e6cce2e03"
        )))
    )]
    #[case::sha512(
        "sha512:3743ae8538d0bdaa6e3838bc1098a021a1e7ebbd78ecad3e025970665e784b63b723363582948f102a30f2d9502c8a6314b35059826a1d5ea1c991f5b224e5fb",
        Ok(Digest::Sha512(Cow::Borrowed("3743ae8538d0bdaa6e3838bc1098a021a1e7ebbd78ecad3e025970665e784b63b723363582948f102a30f2d9502c8a6314b35059826a1d5ea1c991f5b224e5fb")))
    )]
    fn test_digest_from_to_str(
        #[case] input: &str,
        #[case] expected: Result<Digest, ParseDigestError>,
    ) {
        let res = Digest::try_from(input);

        assert_eq!(res, expected);

        if let Ok(digest) = res {
            assert_eq!(digest.to_string(), input);
        }
    }
}
