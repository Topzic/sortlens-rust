//! The diagnostics bundle: one text a person can attach to a bug report.
//!
//! A support report is only useful if it can be read by the person sending it, so this is plain text
//! with labelled sections rather than a zip of JSON and a database. It is also only *safe* to send if it
//! does not leak where someone keeps their photos, so every absolute path is replaced before it is
//! written: each indexed root becomes `<root-N>`, the library folder becomes `<library>`, and the home
//! directory becomes `<home>`. What is left — counts, versions, change summaries, settings, pack names —
//! is what a maintainer needs and nothing that says who the person is.
//!
//! Nothing here is a decision: no op is written, and the health check that it runs is read-only.

use sortlens_core::VERSION as CORE_VERSION;

use crate::Engine;

/// How many recent changes the bundle carries. Enough to see a pattern, short enough to read.
pub const HISTORY_LINES: usize = 25;

/// Replace every absolute path the library knows about with a stable placeholder.
///
/// The longest match wins, so a root inside the library folder is redacted as a root rather than as
/// `<library>`, and the numbering is by position in the list, so two runs of the same library produce the
/// same report.
#[must_use]
pub fn redact(text: &str, library: Option<&str>, roots: &[String]) -> String {
    redact_with_homes(text, library, roots, [std::env::var_os("HOME"), std::env::var_os("USERPROFILE")])
}

fn redact_with_homes(
    text: &str,
    library: Option<&str>,
    roots: &[String],
    homes: impl IntoIterator<Item = Option<std::ffi::OsString>>,
) -> String {
    let mut pairs: Vec<(String, String)> = Vec::new();
    if let Some(library) = library
        && !library.is_empty()
    {
        pairs.push((library.to_string(), "<library>".to_string()));
    }
    for (index, root) in roots.iter().enumerate() {
        if !root.is_empty() {
            pairs.push((root.clone(), format!("<root-{}>", index + 1)));
        }
    }
    for home in homes.into_iter().flatten() {
        let home = home.to_string_lossy();
        if home.len() > 1 {
            pairs.push((home.into_owned(), "<home>".to_string()));
        }
    }
    // Longest first: a root under the library must not be half-replaced by the library's own prefix.
    pairs.sort_by_key(|pair| std::cmp::Reverse(pair.0.len()));
    let mut out = text.to_string();
    for (path, placeholder) in pairs {
        out = out.replace(&path, &placeholder);
    }
    out
}

impl Engine {
    /// Build the diagnostics bundle as text.
    ///
    /// Runs the read-only health check, so a report carries what is actually on disk rather than what the
    /// index believes.
    pub fn diagnostics(&mut self) -> Result<String, sortlens_core::Error> {
        let stats = self.stats();
        let roots = self.roots().to_vec();
        let library = self.library_dir().map(|dir| dir.display().to_string());
        let health = self.check_health()?;
        let packs = self.packs();
        let history = self.history(HISTORY_LINES).unwrap_or_default();
        let warnings = self.warnings().to_vec();
        let read_only = self.read_only();
        let sidecars = self.write_sidecars();

        let mut text = String::new();
        text.push_str("# Sortlens diagnostics\n\n");
        text.push_str("This file is meant to be attached to a bug report. Absolute paths are replaced with\n");
        text.push_str("<library>, <root-N> and <home> before anything is written.\n\n");

        text.push_str("[versions]\n");
        text.push_str(&format!("sortlens = {CORE_VERSION}\n"));
        text.push_str(&format!("engine = {}\n", crate::VERSION));
        text.push_str(&format!("os = {}\n", std::env::consts::OS));
        text.push_str(&format!("arch = {}\n", std::env::consts::ARCH));
        text.push('\n');

        text.push_str("[library]\n");
        text.push_str(&format!("read_only = {read_only}\n"));
        text.push_str(&format!("write_sidecars = {sidecars}\n"));
        text.push_str(&format!("roots = {}\n", roots.len()));
        text.push_str(&format!("photos = {}\n", stats.total));
        text.push_str(&format!("missing = {}\n", stats.missing));
        text.push_str(&format!("rated = {}\n", stats.rated));
        text.push_str(&format!("picks = {}\n", stats.picks));
        text.push_str(&format!("rejects = {}\n", stats.rejects));
        text.push_str(&format!("tagged = {}\n", stats.tagged));
        text.push_str(&format!("bytes = {}\n", stats.bytes));
        text.push_str(&format!("warnings = {}\n", warnings.len()));
        for warning in &warnings {
            text.push_str(&format!("  warning: {warning}\n"));
        }
        text.push('\n');

        text.push_str("[health]\n");
        text.push_str(&format!("checked = {}\n", health.checked));
        text.push_str(&format!("present = {}\n", health.present));
        text.push_str(&format!("missing = {}\n", health.missing));
        text.push_str(&format!("changed = {}\n", health.changed));
        text.push('\n');

        text.push_str("[packs]\n");
        if packs.is_empty() {
            text.push_str("none installed\n");
        }
        for pack in &packs {
            let state = if pack.enabled { "on" } else { "off" };
            match &pack.problem {
                Some(problem) => text.push_str(&format!("{state} {} — {problem}\n", pack.file)),
                None => text.push_str(&format!(
                    "{state} {} · {} keyword(s) · {}\n",
                    pack.name, pack.keywords, pack.licence
                )),
            }
        }
        text.push('\n');

        text.push_str("[recent changes]\n");
        if history.is_empty() {
            text.push_str("none\n");
        }
        for entry in &history {
            text.push_str(&format!("{} {} · {}\n", entry.seq, entry.kind, entry.summary));
        }
        text.push('\n');

        // The roots are named by their placeholders only: the path itself is the thing being withheld, and
        // a maintainer matching `<root-2>` against the change list needs the list to be here.
        text.push_str("[roots]\n");
        text.push_str(&format!("{} indexed\n", roots.len()));
        for index in 0..roots.len() {
            text.push_str(&format!("<root-{}>\n", index + 1));
        }

        Ok(redact(&text, library.as_deref(), &roots))
    }

    /// Where the library lives, for the redaction and for the report header.
    pub fn library_dir(&self) -> Option<&std::path::Path> {
        self.dir.as_deref()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn redaction_replaces_every_root_the_home_and_the_library() {
        let roots = vec!["/photos".to_string(), "/photos/nested".to_string()];
        let text = "library /var/lib/sortlens\nroot /photos/nested/a.jpg\nother /photos/b.jpg\nhome /Users/someone/x";
        let redacted = redact(text, Some("/var/lib/sortlens"), &roots);
        assert!(redacted.contains("<library>"), "{redacted}");
        assert!(redacted.contains("<root-2>/a.jpg"), "the longer root wins: {redacted}");
        assert!(redacted.contains("<root-1>/b.jpg"), "{redacted}");
        assert!(!redacted.contains("/photos"), "{redacted}");
        assert!(!redacted.contains("/var/lib/sortlens"), "{redacted}");
    }

    #[test]
    fn redaction_without_a_library_or_roots_still_removes_the_home() {
        // Exercise HOME and USERPROFILE independently, without relying on the runner's environment.
        for homes in [[Some("/home/person".into()), None], [None, Some("/home/person".into())]] {
            let redacted = redact_with_homes("in /home/person/Pictures", None, &[], homes);
            assert_eq!(redacted, "in <home>/Pictures");
        }
        let homes = [Some("/home/person".into()), Some("C:\\Users\\person".into())];
        let redacted = redact_with_homes("/home/person/a C:\\Users\\person\\b", None, &[], homes);
        assert_eq!(redacted, "<home>/a <home>\\b");
    }

    #[test]
    fn redaction_leaves_relative_paths_alone() {
        // A root-relative path says nothing about the machine, and a maintainer needs it.
        let text = "Trips/Japan/DSC_0001.jpg · IMG_0002.CR3";
        assert_eq!(redact(text, None, &[]), text);
    }
}
