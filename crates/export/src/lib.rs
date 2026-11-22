//! Exporting picks out of the library.
//!
//! Two things happen here, and only one of them writes bytes: **planning** decides every destination
//! path before a single file is touched, so the collision rules are testable without a disk, and
//! **writing** carries them out. That split is why an export can say "17 written, 3 skipped as
//! duplicates of names already there" before it starts.
//!
//! What this deliberately does not do: it never modifies, moves or deletes the original, and it never
//! edits the library. An export is a read of the library and a write somewhere else.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

use sortlens_core::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// The only formats M8 ships.
///
/// PNG, TIFF and WebP were planned and are not here: the suite has one encoder, so promising a format
/// this build cannot write would be a lie in the UI. `Original` covers "give me the raw files", which
/// is what most exports actually want.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    /// Copy the file as it is: no re-encode, no loss, no resize.
    #[default]
    Original,
    /// Decode and re-encode as JPEG with the requested quality.
    Jpeg,
}

impl Format {
    /// Every format this build can write.
    pub const ALL: [Format; 2] = [Format::Original, Format::Jpeg];

    /// The extension written for this format, or `None` when it depends on the source.
    #[must_use]
    pub fn extension(&self) -> Option<&'static str> {
        match self {
            Format::Original => None,
            Format::Jpeg => Some("jpg"),
        }
    }

    /// A short name for a command line or a menu.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Format::Original => "Original",
            Format::Jpeg => "JPEG",
        }
    }

    /// Parse a name, case-insensitively.
    #[must_use]
    pub fn parse(text: &str) -> Option<Format> {
        match text.trim().to_ascii_lowercase().as_str() {
            "original" | "copy" => Some(Format::Original),
            "jpeg" | "jpg" => Some(Format::Jpeg),
            _ => None,
        }
    }

    /// The id used by the command registry and the CLI.
    #[must_use]
    pub fn id(&self) -> &'static str {
        match self {
            Format::Original => "original",
            Format::Jpeg => "jpeg",
        }
    }
}

/// What to do when the destination already has a file with that name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Collision {
    /// Leave the existing file alone and do not write this one.
    #[default]
    Skip,
    /// Write `name-2.jpg`, `name-3.jpg`, … — never overwriting, never losing the new file.
    Rename,
    /// Replace the existing file.
    Overwrite,
}

impl Collision {
    /// Every policy.
    pub const ALL: [Collision; 3] = [Collision::Skip, Collision::Rename, Collision::Overwrite];

    /// A short name for a menu.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Collision::Skip => "Skip",
            Collision::Rename => "Keep both",
            Collision::Overwrite => "Replace",
        }
    }

    /// Parse a name, case-insensitively.
    #[must_use]
    pub fn parse(text: &str) -> Option<Collision> {
        match text.trim().to_ascii_lowercase().as_str() {
            "skip" => Some(Collision::Skip),
            "rename" | "keep" | "keep-both" => Some(Collision::Rename),
            "overwrite" | "replace" => Some(Collision::Overwrite),
            _ => None,
        }
    }

    /// The id used by the command registry and the CLI.
    #[must_use]
    pub fn id(&self) -> &'static str {
        match self {
            Collision::Skip => "skip",
            Collision::Rename => "rename",
            Collision::Overwrite => "overwrite",
        }
    }
}

/// Everything an export needs to know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    /// What to write.
    pub format: Format,
    /// Longest edge in pixels; `None` keeps the source size. Ignored by [`Format::Original`], because
    /// copying bytes cannot resize them — the UI says so rather than pretending.
    pub max_edge: Option<u32>,
    /// JPEG quality, 1–100.
    pub quality: u8,
    /// Carry the source's EXIF and IPTC blocks into a re-encoded JPEG.
    pub metadata: bool,
    /// What to do about a name that is already there.
    pub collision: Collision,
}

impl Default for Spec {
    fn default() -> Self {
        Self {
            format: Format::Original,
            max_edge: None,
            quality: 90,
            metadata: true,
            collision: Collision::Skip,
        }
    }
}

impl Spec {
    /// Validate the numbers a caller typed.
    pub fn check(&self) -> Result<()> {
        if self.quality == 0 || self.quality > 100 {
            return Err(Error::Invalid(format!("quality {} is outside 1–100", self.quality)));
        }
        if let Some(edge) = self.max_edge
            && edge < 16
        {
            return Err(Error::Invalid(format!("a longest edge of {edge} px is too small")));
        }
        Ok(())
    }

    /// One line describing what this will produce, for a dialog and a receipt.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut text = self.format.label().to_string();
        if self.format == Format::Jpeg {
            text.push_str(&format!(" · quality {}", self.quality));
            if let Some(edge) = self.max_edge {
                text.push_str(&format!(" · {edge} px longest edge"));
            }
            if !self.metadata {
                text.push_str(" · without metadata");
            }
        }
        text
    }
}

/// One file to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    /// Where the photo is now.
    pub source: PathBuf,
    /// Where it will be written.
    pub dest: PathBuf,
    /// Why this one will not be written, when it will not be.
    pub skip: Option<SkipReason>,
}

/// Why a file was not written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// A file with that name is already there and the policy is to skip.
    NameTaken,
    /// The plan ran out of names: 1000 files called `photo.jpg`.
    NoFreeName,
}

impl SkipReason {
    /// What to tell the user.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            SkipReason::NameTaken => "a file with that name is already there",
            SkipReason::NoFreeName => "no free name: 1000 files already end in that number",
        }
    }
}

/// How many suffixed names a rename will try before giving up.
pub const RENAME_LIMIT: u32 = 1000;

/// The file name without its extension, for building a new one.
fn stem_of(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "photo".to_string())
}

/// The extension a source keeps when it is copied, in the case the source used: `IMG_0001.CR3` copied
/// on a case-sensitive filesystem should still look like the file the photographer's camera wrote.
fn extension_of(path: &Path) -> Option<String> {
    path.extension().map(|extension| extension.to_string_lossy().to_string())
}

/// Decide every destination path up front.
///
/// `existing` is the set of names the destination already holds; the caller reads it once so the plan
/// is deterministic and needs no disk access per file. Names are generated in the order given, so two
/// photos with the same name in one export also collide with each other and not only with the folder.
#[must_use]
pub fn plan(spec: &Spec, sources: &[PathBuf], dest: &Path, existing: &[String]) -> Vec<Job> {
    let mut taken: Vec<String> = existing.iter().map(|name| name.to_ascii_lowercase()).collect();
    let mut jobs = Vec::with_capacity(sources.len());
    for source in sources {
        let stem = stem_of(source);
        let extension = match spec.format.extension() {
            Some(extension) => extension.to_string(),
            None => extension_of(source).unwrap_or_else(|| "bin".to_string()),
        };
        let base = format!("{stem}.{extension}");
        let free = !taken.contains(&base.to_ascii_lowercase());
        let (name, skip) = if free {
            (base, None)
        } else {
            match spec.collision {
                Collision::Skip => (base, Some(SkipReason::NameTaken)),
                Collision::Overwrite => (base, None),
                Collision::Rename => {
                    let mut chosen = None;
                    for tick in 2..=RENAME_LIMIT {
                        let candidate = format!("{stem}-{tick}.{extension}");
                        if !taken.contains(&candidate.to_ascii_lowercase()) {
                            chosen = Some(candidate);
                            break;
                        }
                    }
                    match chosen {
                        Some(candidate) => (candidate, None),
                        None => (base, Some(SkipReason::NoFreeName)),
                    }
                }
            }
        };
        if skip.is_none() {
            taken.push(name.to_ascii_lowercase());
        }
        jobs.push(Job {
            source: source.clone(),
            dest: dest.join(name),
            skip,
        });
    }
    jobs
}

/// The names in a folder, lowercased, or an empty list when it does not exist yet.
///
/// A folder that cannot be read is not an error here: the export reports it when it tries to write,
/// which is where the user can do something about it.
#[must_use]
pub fn existing_names(dest: &Path) -> Vec<String> {
    match fs::read_dir(dest) {
        Ok(entries) => entries
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().to_lowercase())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// The JPEG markers whose payload is metadata rather than image data.
const APP1_EXIF: u8 = 0xE1;
const APP13_IPTC: u8 = 0xED;

/// Collect the metadata segments a JPEG carries so a re-encode can keep them.
///
/// Only the marker payloads are copied, and only the two that carry EXIF and IPTC: an application
/// segment can hold anything, and blindly forwarding every one of them would carry another program's
/// editing state into an exported file.
#[must_use]
pub fn metadata_segments(jpeg: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut out = Vec::new();
    if jpeg.len() < 4 || jpeg[0] != 0xFF || jpeg[1] != 0xD8 {
        return out;
    }
    let mut index = 2usize;
    while index + 4 <= jpeg.len() {
        if jpeg[index] != 0xFF {
            break;
        }
        let marker = jpeg[index + 1];
        // Entropy-coded data starts here; nothing after it is a metadata segment.
        if marker == 0xDA || marker == 0xD9 {
            break;
        }
        let length = usize::from(u16::from_be_bytes([jpeg[index + 2], jpeg[index + 3]]));
        if length < 2 || index + 2 + length > jpeg.len() {
            break;
        }
        let payload = &jpeg[index + 4..index + 2 + length];
        if marker == APP1_EXIF || marker == APP13_IPTC {
            out.push((marker, payload.to_vec()));
        }
        index += 2 + length;
    }
    out
}

/// Write one file.
///
/// Returns the number of bytes written. A skipped job writes nothing and returns `0`.
pub fn export_one(spec: &Spec, job: &Job) -> Result<u64> {
    if job.skip.is_some() {
        return Ok(0);
    }
    if let Some(parent) = job.dest.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| Error::io(parent.display().to_string(), error))?;
    }
    let original = fs::read(&job.source).map_err(|error| Error::io(job.source.display().to_string(), error))?;
    let bytes = match spec.format {
        Format::Original => original,
        Format::Jpeg => {
            let edge = spec.max_edge.unwrap_or(u32::MAX);
            let decoded = sortlens_thumbs::decode_file(&job.source, edge).or_else(|_| sortlens_thumbs::decode_jpeg(&original, edge))?;
            let rgb: Vec<u8> = decoded
                .rgba
                .chunks_exact(4)
                .flat_map(|pixel| [pixel[0], pixel[1], pixel[2]])
                .collect();
            let segments = if spec.metadata { metadata_segments(&original) } else { Vec::new() };
            craft_jpeg::encode_baseline(
                &rgb,
                decoded.width as usize,
                decoded.height as usize,
                3,
                spec.quality,
                craft_jpeg::ChromaSubsampling::S444,
                &segments,
            )
            .map_err(|error| Error::Invalid(format!("could not encode {}: {error:?}", job.dest.display())))?
        }
    };
    fs::write(&job.dest, &bytes).map_err(|error| Error::io(job.dest.display().to_string(), error))?;
    Ok(bytes.len() as u64)
}

/// What an export produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Files written.
    pub written: usize,
    /// Files deliberately not written.
    pub skipped: usize,
    /// Files that failed, with the reason.
    pub failures: Vec<String>,
    /// Bytes written.
    pub bytes: u64,
    /// Where they went.
    pub dest: PathBuf,
}

impl Report {
    /// Whether every file was written.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.failures.is_empty()
    }

    /// One line for a receipt or a log.
    #[must_use]
    pub fn headline(&self) -> String {
        let mut text = format!(
            "{} written · {} skipped · {}",
            self.written,
            self.skipped,
            sortlens_core::format_bytes(self.bytes)
        );
        if !self.failures.is_empty() {
            text.push_str(&format!(" · {} failed", self.failures.len()));
        }
        text
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn sources(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn a_clean_folder_needs_no_collision_policy() {
        let spec = Spec::default();
        let jobs = plan(&spec, &sources(&["/a/one.jpg", "/b/two.jpg"]), Path::new("/out"), &[]);
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].dest, PathBuf::from("/out/one.jpg"));
        assert_eq!(jobs[1].dest, PathBuf::from("/out/two.jpg"));
        assert!(jobs.iter().all(|job| job.skip.is_none()));
    }

    #[test]
    fn the_same_name_in_one_export_collides_with_itself() {
        let spec = Spec {
            collision: Collision::Rename,
            ..Spec::default()
        };
        let jobs = plan(&spec, &sources(&["/a/photo.jpg", "/b/photo.jpg"]), Path::new("/out"), &[]);
        assert_eq!(jobs[0].dest, PathBuf::from("/out/photo.jpg"));
        assert_eq!(jobs[1].dest, PathBuf::from("/out/photo-2.jpg"));
    }

    #[test]
    fn skipping_leaves_the_existing_file_alone() {
        let spec = Spec::default();
        let jobs = plan(&spec, &sources(&["/a/photo.jpg"]), Path::new("/out"), &["photo.jpg".into()]);
        assert_eq!(jobs[0].skip, Some(SkipReason::NameTaken));
        assert_eq!(jobs[0].dest, PathBuf::from("/out/photo.jpg"));
    }

    #[test]
    fn renaming_never_picks_a_name_that_is_taken() {
        let spec = Spec {
            collision: Collision::Rename,
            ..Spec::default()
        };
        let existing = vec!["photo.jpg".to_string(), "photo-2.jpg".to_string()];
        let jobs = plan(&spec, &sources(&["/a/photo.jpg"]), Path::new("/out"), &existing);
        assert_eq!(jobs[0].dest, PathBuf::from("/out/photo-3.jpg"));
        assert!(jobs[0].skip.is_none());
    }

    #[test]
    fn a_folder_of_a_thousand_photo_two_files_is_reported_not_looped_over() {
        let spec = Spec {
            collision: Collision::Rename,
            ..Spec::default()
        };
        let mut existing = vec!["photo.jpg".to_string()];
        for tick in 2..=RENAME_LIMIT {
            existing.push(format!("photo-{tick}.jpg"));
        }
        let jobs = plan(&spec, &sources(&["/a/photo.jpg"]), Path::new("/out"), &existing);
        assert_eq!(jobs[0].skip, Some(SkipReason::NoFreeName));
    }

    #[test]
    fn overwriting_is_allowed_to_reuse_the_name() {
        let spec = Spec {
            collision: Collision::Overwrite,
            ..Spec::default()
        };
        let jobs = plan(&spec, &sources(&["/a/photo.jpg"]), Path::new("/out"), &["photo.jpg".into()]);
        assert!(jobs[0].skip.is_none());
        assert_eq!(jobs[0].dest, PathBuf::from("/out/photo.jpg"));
    }

    #[test]
    fn a_jpeg_export_switches_the_extension() {
        let spec = Spec {
            format: Format::Jpeg,
            ..Spec::default()
        };
        let jobs = plan(&spec, &sources(&["/a/IMG_0001.CR3"]), Path::new("/out"), &[]);
        assert_eq!(jobs[0].dest, PathBuf::from("/out/IMG_0001.jpg"));
    }

    #[test]
    fn an_original_export_keeps_the_source_extension() {
        let spec = Spec::default();
        let jobs = plan(&spec, &sources(&["/a/IMG_0001.CR3"]), Path::new("/out"), &[]);
        assert_eq!(jobs[0].dest, PathBuf::from("/out/IMG_0001.CR3"));
    }

    #[test]
    fn quality_and_size_are_validated_rather_than_clamped() {
        assert!(
            Spec {
                quality: 0,
                ..Spec::default()
            }
            .check()
            .is_err()
        );
        assert!(
            Spec {
                quality: 101,
                ..Spec::default()
            }
            .check()
            .is_err()
        );
        assert!(
            Spec {
                max_edge: Some(4),
                ..Spec::default()
            }
            .check()
            .is_err()
        );
        assert!(
            Spec {
                quality: 85,
                max_edge: Some(1600),
                ..Spec::default()
            }
            .check()
            .is_ok()
        );
    }

    #[test]
    fn a_summary_says_what_will_be_written() {
        let spec = Spec {
            format: Format::Jpeg,
            max_edge: Some(2048),
            quality: 85,
            metadata: false,
            ..Spec::default()
        };
        let text = spec.summary();
        assert!(text.contains("JPEG"));
        assert!(text.contains("quality 85"));
        assert!(text.contains("2048 px"));
        assert!(text.contains("without metadata"));
        assert_eq!(Spec::default().summary(), "Original", "a copy has nothing else to say");
    }

    #[test]
    fn an_export_writes_a_real_file() {
        let fixture = sortlens_testkit::Fixture::new("export");
        let source = fixture.jpeg("a.jpg", [120, 30, 200], 64, 48);
        let dest = fixture.root().join("out");
        let spec = Spec::default();
        let jobs = plan(&spec, &[source], &dest, &existing_names(&dest));
        assert_eq!(jobs.len(), 1);
        let written = export_one(&spec, &jobs[0]).expect("export");
        assert!(written > 0);
        let copied = fs::read(&jobs[0].dest).expect("read copy");
        assert_eq!(copied, fs::read(fixture.path("a.jpg")).expect("read source"), "a copy is a copy");
    }

    #[test]
    fn a_jpeg_export_resizes_and_re_encodes() {
        let fixture = sortlens_testkit::Fixture::new("export-resize");
        let source = fixture.jpeg("a.jpg", [200, 200, 40], 200, 100);
        let dest = fixture.root().join("out");
        let spec = Spec {
            format: Format::Jpeg,
            max_edge: Some(100),
            quality: 80,
            ..Spec::default()
        };
        let jobs = plan(&spec, &[source], &dest, &[]);
        export_one(&spec, &jobs[0]).expect("export");
        let decoded = sortlens_thumbs::decode_file(&jobs[0].dest, 4096).expect("decode");
        assert_eq!((decoded.width, decoded.height), (100, 50), "the long edge is the one clamped");
    }

    #[test]
    fn a_skipped_job_writes_nothing_and_keeps_the_original() {
        let fixture = sortlens_testkit::Fixture::new("export-skip");
        let source = fixture.jpeg("a.jpg", [10, 200, 10], 16, 16);
        let dest = fixture.root().join("out");
        fs::create_dir_all(&dest).expect("dest");
        fs::write(dest.join("a.jpg"), b"already here").expect("existing");
        let spec = Spec::default();
        let jobs = plan(&spec, &[source], &dest, &existing_names(&dest));
        assert_eq!(jobs[0].skip, Some(SkipReason::NameTaken));
        assert_eq!(export_one(&spec, &jobs[0]).expect("skip"), 0);
        assert_eq!(fs::read(dest.join("a.jpg")).expect("read"), b"already here");
    }

    #[test]
    fn metadata_segments_come_from_the_headers_only() {
        let pixels = vec![128u8; 32 * 32 * 3];
        let exif = vec![0x45u8, 0x78, 0x69, 0x66];
        let encoded = craft_jpeg::encode_baseline(
            &pixels,
            32,
            32,
            3,
            90,
            craft_jpeg::ChromaSubsampling::S444,
            &[(APP1_EXIF, exif.clone())],
        )
        .expect("encode");
        let found = metadata_segments(&encoded);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, APP1_EXIF);
        assert_eq!(found[0].1, exif);
    }

    #[test]
    fn a_file_that_is_not_a_jpeg_has_no_segments() {
        assert!(metadata_segments(b"not a jpeg at all").is_empty());
        assert!(metadata_segments(b"").is_empty());
    }

    #[test]
    fn a_report_says_what_happened() {
        let report = Report {
            written: 17,
            skipped: 3,
            failures: Vec::new(),
            bytes: 2048,
            dest: PathBuf::from("/out"),
        };
        let text = report.headline();
        assert!(text.contains("17 written"));
        assert!(text.contains("3 skipped"));
        assert!(text.contains("2.0 kB"));
        assert!(report.is_clean());
        let failed = Report {
            failures: vec!["a.jpg: permission denied".into()],
            ..report
        };
        assert!(!failed.is_clean());
        assert!(failed.headline().contains("1 failed"));
    }
}
