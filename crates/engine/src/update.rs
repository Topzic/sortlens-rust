//! The opt-in update check (SL-I10, desktop only).
//!
//! Sortlens makes exactly two approved network calls, both user-initiated and both consent-gated:
//! downloading models ([`crate::models`]) and checking for an update. This module is the second
//! one's engine face: it asks GitHub's releases API for the app's own repository, reads the latest
//! published release through `craft-update`'s parsing (every field capped, links held to the app's
//! own repository), and compares it against the running version with `craft-update`'s semver
//! rules — including its local-build rule, so a build from source is never "downgraded" to a
//! release it already contains.
//!
//! Nothing here runs without consent, and nothing else in the crate can reach a socket.

use craft_update::http::Fetched;
use serde::Serialize;

/// The repository releases are read from — the same one this checkout pushes to. Release files
/// are only ever linked from this repository, and `craft-update`'s parser refuses assets whose
/// download URL points anywhere else.
pub const REPO: &str = "Topzic/sortlens-rust";

/// The GitHub releases URL for a repository: the endpoint `check` reads.
pub fn releases_url(repo: &str) -> String {
    format!("https://api.github.com/repos/{repo}/releases?per_page=30")
}

/// One small JSON read of GitHub's API: the seam that keeps the network out of the tests.
///
/// The real implementation wraps `craft-update`'s [`craft_update::Http`], which carries the
/// GitHub API headers, the `ETag` revalidation, the rate-limit reading and a bounded body.
pub trait Fetcher: Send + Sync {
    fn get_api(&self, url: &str, etag: Option<&str>) -> std::result::Result<Fetched, String>;
}

/// The real fetcher. Built once per check; a failure to construct (a broken TLS store, a broken
/// proxy setting) is a refusal, not a silence.
pub struct Http {
    inner: craft_update::Http,
}

impl Http {
    /// A client that names the product and its version, and nothing about the person using it.
    pub fn new() -> std::result::Result<Self, String> {
        let user_agent = concat!("Sortlens/", env!("CARGO_PKG_VERSION"));
        Ok(Self {
            inner: craft_update::Http::new(user_agent)?,
        })
    }
}

impl Fetcher for Http {
    fn get_api(&self, url: &str, etag: Option<&str>) -> std::result::Result<Fetched, String> {
        self.inner.get_api(url, etag)
    }
}

/// What one check found.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UpdateReport {
    /// The version this check ran from.
    pub current: String,
    /// The latest published release, when the API had one.
    pub latest: Option<craft_update::Release>,
    /// Whether that release is newer than the running one, by `craft-update`'s semver rules.
    pub offered: bool,
}

impl UpdateReport {
    /// The nothing-found answer: the API answered, and it held no release for this repository.
    pub fn none(current: &str) -> Self {
        Self {
            current: current.to_owned(),
            latest: None,
            offered: false,
        }
    }

    /// One line for a notice or an activity row.
    #[must_use]
    pub fn headline(&self) -> String {
        match self.latest.as_ref() {
            Some(release) if self.offered => format!(
                "Update available · {} · {}",
                release.version_label(),
                first_note_line(&release.notes),
            ),
            Some(release) => format!(
                "Sortlens {} is up to date · {} is the latest release",
                self.current,
                release.version_label()
            ),
            None => format!("Sortlens {} is up to date · no releases found", self.current),
        }
    }
}

/// The first non-empty line of release notes, capped — enough for a notice, not for a page.
fn first_note_line(notes: &str) -> String {
    notes
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("no notes")
        .chars()
        .take(80)
        .collect()
}

/// Run one check against `releases_url`. `repo` is held to the release's own repository by the
/// parser; `current` is the running version string.
pub fn check(fetcher: &dyn Fetcher, releases_url: &str, repo: &str, current: &str) -> std::result::Result<UpdateReport, String> {
    match fetcher.get_api(releases_url, None)? {
        Fetched::Body { text, .. } => parse(&text, repo, current),
        Fetched::RateLimited { reset: _ } => Err("GitHub's request limit was reached — try again later".into()),
        Fetched::NotModified | Fetched::NotFound => Ok(UpdateReport::none(current)),
    }
}

/// The read half of the check: the release list through `craft-update`'s parser (drafts, error
/// bodies and foreign download links refused, every text capped), then the version comparison.
pub fn parse(json: &str, repo: &str, current: &str) -> std::result::Result<UpdateReport, String> {
    let latest = craft_update::parse_github_release_list(json, repo, false)?;
    let offered = latest
        .as_ref()
        .and_then(|release| release.version.as_ref())
        .zip(craft_update::parse_version(current))
        .is_some_and(|(latest, running)| craft_update::version::offers_update(latest, &running));
    Ok(UpdateReport {
        current: current.to_owned(),
        latest,
        offered,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// A GitHub releases-list answer with one stable release newer than the running 0.1.0.
    const NEWER: &str = r#"[
        {
            "tag_name": "v0.2.0",
            "name": "Sortlens v0.2.0",
            "draft": false,
            "prerelease": false,
            "published_at": "2026-11-20T10:00:00Z",
            "html_url": "https://github.com/Topzic/sortlens-rust/releases/tag/v0.2.0",
            "body": "A fix worth having",
            "assets": [
                { "name": "SHA256SUMS.txt", "browser_download_url": "https://github.com/Topzic/sortlens-rust/releases/download/v0.2.0/SHA256SUMS.txt", "size": 120 }
            ]
        }
    ]"#;

    #[test]
    fn a_newer_stable_release_is_offered_and_summarised() {
        let report = parse(NEWER, REPO, "0.1.0").expect("parses");
        assert!(report.offered);
        let release = report.latest.as_ref().expect("the release came through");
        assert_eq!(release.version_label(), "0.2.0");
        assert_eq!(release.repo, REPO, "the parser holds the release to its own repository");
        assert_eq!(report.headline(), "Update available · 0.2.0 · A fix worth having");
    }

    #[test]
    fn an_older_or_equal_release_is_not_offered() {
        let report = parse(NEWER, REPO, "9.9.9").expect("parses");
        assert!(!report.offered);
        assert!(
            report.headline().contains("is up to date"),
            "an answer is still an answer: {}",
            report.headline()
        );
    }

    #[test]
    fn a_prerelease_is_skipped_so_a_stable_running_stays_current() {
        let list = r#"[
            { "tag_name": "v0.3.0-rc.1", "name": "rc", "draft": false, "prerelease": true,
              "html_url": "https://github.com/Topzic/sortlens-rust", "body": "", "assets": [] }
        ]"#;
        let report = parse(list, REPO, "0.1.0").expect("parses");
        assert!(report.latest.is_none(), "prereleases are not offered to a stable build");
        assert!(!report.offered);
    }

    #[test]
    fn a_local_build_is_only_offered_a_higher_release_number() {
        // semver puts 0.1.0-integration… before 0.1.0, but replacing a build from source with the
        // release it already contains would be a downgrade: craft-update's rule refuses it.
        let list = NEWER.replace("v0.2.0", "v0.1.0").replace("0.2.0", "0.1.0");
        let report = parse(&list, REPO, "0.1.0-integration.20261124.f5b3f123").expect("parses");
        assert!(!report.offered, "0.1.0 is not newer than 0.1.0's main branch");
    }

    #[test]
    fn garbage_and_rate_limits_are_named_errors() {
        assert!(parse("not json", REPO, "0.1.0").is_err());
        struct Limited;
        impl Fetcher for Limited {
            fn get_api(&self, _url: &str, _etag: Option<&str>) -> std::result::Result<Fetched, String> {
                Ok(Fetched::RateLimited { reset: None })
            }
        }
        let error = check(&Limited, &releases_url(REPO), REPO, "0.1.0").expect_err("limited");
        assert!(error.contains("request limit"), "{error}");
    }

    #[test]
    fn an_empty_answer_reads_as_up_to_date_not_as_a_failure() {
        struct Empty;
        impl Fetcher for Empty {
            fn get_api(&self, _url: &str, _etag: Option<&str>) -> std::result::Result<Fetched, String> {
                Ok(Fetched::NotFound)
            }
        }
        let report = check(&Empty, &releases_url(REPO), REPO, "0.1.0").expect("empty");
        assert_eq!(report, UpdateReport::none("0.1.0"));
    }
}
