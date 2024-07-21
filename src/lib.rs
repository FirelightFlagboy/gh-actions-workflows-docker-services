pub mod pkg_info;
mod reqwest_utils;
mod serde_utils;
mod version;

pub use pkg_info::{
    Arch, Base as PkgInfoBase, BashCmdReleaseHandler, Digest, GithubReleaseHandler,
    JqScriptReleaseHandler, Mode as PkgInfoMode, ModeGetLatestVersion, PkgInfo, PkgOption,
    VersionedArchEntry,
};
