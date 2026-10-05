//! Self-update from GitHub releases, through fastframe-update.
//!
//! The crate checks for a newer release, refuses package-managed copies,
//! downloads and verifies the update against the publisher signature, and
//! hands it to a helper that installs it and rolls back if it does not start.
//! ZapFast keeps its names, its key, its proxy and its interface.

pub use fastframe_update::{
    CHECK_INTERVAL, DownloadState, Installation, Kind, Prepared, Release, Source, Unsupported,
    Updater,
};
use fastframe_update::{MacConfig, ReqwestTransport, UpdateConfig};

/// ZapFast's releases and the names its installations have had.
pub const CONFIG: UpdateConfig = UpdateConfig {
    // Cask and bundle names from before the rename. This also accepts
    // fastsapp-* marker files and `fastsapp <version>` answers, which no
    // release produces.
    legacy_names: &["fastsapp"],
    macos: MacConfig {
        bundle_ids: &["me.paolino.fastsapp"],
        executable_names: &[],
        legacy_bundle_names: &["FastsApp.app"],
    },
    publisher_key: Some(include_str!("../assets/update-public-key.hex")),
    // The next release key, backed up outside GitHub. Releases stay signed
    // with the current key until installs trust this one too.
    additional_publisher_keys: &[include_str!("../assets/update-public-key-next.hex")],
    ..UpdateConfig::new(
        "crmne/zapfast",
        "ZapFast",
        "zapfast",
        env!("CARGO_PKG_VERSION"),
    )
};

/// An updater on the proxy-aware reqwest client.
pub fn updater() -> anyhow::Result<Updater> {
    let mut builder = reqwest::blocking::Client::builder();
    if let Some(proxy) = crate::proxy::reqwest_proxy() {
        builder = builder.proxy(proxy);
    }
    Ok(Updater::new(CONFIG, ReqwestTransport::new(builder)?))
}

/// Fork: what the update dialog copies instead of installing upstream's
/// build, asking an assistant to rebase this checkout onto the release while
/// keeping the changes FORK.md lists. Edit `assets/fork-merge-prompt.md` to
/// change the wording.
pub fn merge_prompt(release: &Release) -> String {
    include_str!("../assets/fork-merge-prompt.md")
        .replace("{checkout}", env!("CARGO_MANIFEST_DIR"))
        .replace("{running}", env!("CARGO_PKG_VERSION"))
        .replace("{new}", &release.version)
        .replace("{url}", &release.url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_merge_prompt_names_the_release_this_build_and_the_ledger() {
        let prompt = merge_prompt(&Release {
            version: "9.9.9".into(),
            url: "https://example.invalid/release".into(),
        });
        assert!(prompt.contains("git rebase v9.9.9"));
        assert!(prompt.contains(env!("CARGO_PKG_VERSION")));
        assert!(prompt.contains(env!("CARGO_MANIFEST_DIR")));
        assert!(prompt.contains("https://example.invalid/release"));
        assert!(prompt.contains("FORK.md"));
        assert!(!prompt.contains('{'), "a placeholder is left: {prompt}");
    }

    #[test]
    fn update_config_is_valid() {
        CONFIG.validate().unwrap();
        assert_eq!(CONFIG.current_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(CONFIG.slug, "zapfast");
    }

    #[test]
    fn the_updater_starts_on_github() {
        assert!(updater().unwrap().source().is_github());
    }
}
