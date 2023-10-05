use std::borrow::Cow;

use regex::Regex;
use rstest::rstest;

use pkg_info_update::{
    Arch, BashCmdReleaseHandler, Digest, GithubReleaseHandler, JqScriptReleaseHandler, PkgInfo,
    PkgInfoBase, PkgInfoMode, VersionedArchEntry,
};

#[rstest]
#[case::minimal_github_release(
    std::include_str!("samples/minimal-github-release.json"),
    PkgInfo {
        base: PkgInfoBase {
            name: "Gohugo",
            latest_version: None,
            versions: None
        },
        mode: PkgInfoMode::GithubRelease(GithubReleaseHandler {
            repository_path: &"gohugoio/hugo",
            arch_asset_patterns: [
                (Arch::Amd64, Regex::new("^hugo_([0-9]+(\\.[0-9]+)+)_linux-amd64.tar.gz$").unwrap()),
                (Arch::Arm64, Regex::new("^hugo_([0-9]+(\\.[0-9]+)+)_linux-arm64.tar.gz$").unwrap())
            ].into_iter().collect()
        })
    }
)]
#[case::minimal_bash_command(
    std::include_str!("samples/minimal-bash-command.json"),
    PkgInfo {
        base: PkgInfoBase { name: "Foobar", latest_version: None, versions: None },
        mode: PkgInfoMode::BashCommand(BashCmdReleaseHandler {
            command: Cow::Borrowed(r#"echo '{ "version": "0.1.0", "assets": { "amd64": { "filename": "foobar", "download_url": "https://google.com", "digest": "sha256:e8bf04349572f90e569c5bd46be3f7101e1e289125adb8b9eaba94badba1c43a" } } }'"#)
        })
    }
)]
#[case::minimal_jq_script(
    std::include_str!("samples/minimal-jq-script.json"),
    PkgInfo {
        base: PkgInfoBase {
            name: "Sonarr",
            latest_version: None,
            versions: None
        },
        mode: PkgInfoMode::JqScript(JqScriptReleaseHandler {
            document_url: url::Url::parse("https://services.sonarr.tv/v1/releases").unwrap(),
            script_path: "sonarr.jq".as_ref()
        })
    }
)]
#[case::single_version_github_release(
    std::include_str!("samples/single-version-github-release.json"),
    PkgInfo {
        base: PkgInfoBase {
            name: "Gohugo",
            latest_version: Some(Cow::Borrowed("v0.119.0")),
            versions: Some([
                (Cow::Borrowed("v0.119.0"), [
                    (
                        Arch::Amd64,
                        VersionedArchEntry {
                            filename: Cow::Borrowed("hugo_0.119.0_linux-amd64.tar.gz"),
                            download_url: url::Url::parse("https://github.com/gohugoio/hugo/releases/download/v0.119.0/hugo_0.119.0_linux-amd64.tar.gz").unwrap(),
                            digest: Digest::Sha512(Cow::Borrowed("01781c4162da4788a98b5d704222ca007ad020dbe3dfbdc18858ee2eafa115ba2792593370a905aa7c8f2a9f07170721f2de44fca191b29dc01f1108ea1af631"))
                        }
                    ),
                    (
                        Arch::Arm64,
                        VersionedArchEntry {
                            filename: Cow::Borrowed("hugo_0.119.0_linux-arm64.tar.gz"),
                            download_url: url::Url::parse("https://github.com/gohugoio/hugo/releases/download/v0.119.0/hugo_0.119.0_linux-arm64.tar.gz").unwrap(),
                            digest: Digest::Sha512(Cow::Borrowed("18db0f2d55ec94eb8555af16964d40863c60aaa89498a45d50c4644cc9b018a46744a323d1ce7f4af59b0c0bd665a97d5b212c231f7f368e3ac5ac81aa9a55ec"))
                        }
                    )
                ].into_iter().collect())
            ].into_iter().collect())
        },
        mode: PkgInfoMode::GithubRelease(GithubReleaseHandler {
            repository_path: &"gohugoio/hugo",
            arch_asset_patterns: [
                (Arch::Amd64, Regex::new("^hugo_([0-9]+(\\.[0-9]+)+)_linux-amd64.tar.gz$").unwrap()),
                (Arch::Arm64, Regex::new("^hugo_([0-9]+(\\.[0-9]+)+)_linux-arm64.tar.gz$").unwrap())
            ].into_iter().collect()
        })
    }
)]
fn schema(#[case] input: &str, #[case] expected: PkgInfo) {
    let got = serde_json::from_str::<PkgInfo>(input).unwrap();
    assert_eq!(got, expected);
}
