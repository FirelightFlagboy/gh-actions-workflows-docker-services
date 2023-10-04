pub mod pkg_info;
mod serde_utils;

pub use pkg_info::{
    Arch, Base as PkgInfoBase, BashCmdReleaseHandler, Digest, GithubReleaseHandler,
    Mode as PkgInfoMode, ModeGetLatestVersion, PkgInfo, VersionedArchEntry,
};
