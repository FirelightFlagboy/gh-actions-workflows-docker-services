use std::{borrow::Cow, path::PathBuf};

use anyhow::Context;
use clap::Parser;

use pkg_info_updater::PkgInfo;

#[derive(Debug, Parser)]
struct Args {
    /// The path to the pkg info file.
    #[arg(long = "file", short = 'f', default_value = "pkg-info.json")]
    file: PathBuf,
    /// Path to a temporary folder.
    #[arg(long = "tmp-dir", default_value = "/tmp")]
    tmp_dir: PathBuf,
    /// Enable test mode (do not require a clean working env).
    #[arg(long = "test")]
    test: bool,
}

fn main() -> anyhow::Result<()> {
    init_log();
    let args = Args::parse();

    log::trace!("args={args:#?}");
    let raw_data = std::fs::read_to_string(&args.file).context("Reading the data")?;
    let pkg_info = serde_json::from_str::<PkgInfo>(&raw_data).context("Deserializing the data")?;

    log::trace!("pkg_info={pkg_info:#?}");

    let fut = pkg_info.mode.get_latest_version(&args.tmp_dir, args.test)?;

    let tokio_runtime = tokio::runtime::Runtime::new()?;
    let (version, content) = tokio_runtime.block_on(fut)?;
    let mut pkg_info = pkg_info;

    pkg_info.base.latest_version = Some(Cow::Borrowed(version.as_str()));
    let versions = pkg_info.base.versions.get_or_insert_with(Default::default);
    *versions.entry(Cow::Borrowed(&version)).or_default() = content;

    let raw_dump_data = serde_json::to_string_pretty(&pkg_info).context("Serializing the data")?;
    std::fs::write(&args.file, raw_dump_data)
        .and_then(|_| std::fs::write(&args.file, b"\n"))
        .context("Writing back the data")?;
    Ok(())
}

fn init_log() {
    use env_logger::{Builder, Env};

    const DEFAULT_FILTER: &str = "debug,reqwest=warn";

    Builder::from_env(Env::default().default_filter_or(DEFAULT_FILTER)).init();
}
