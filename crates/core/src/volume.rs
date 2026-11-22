//! Which volume a folder lives on, and whether that volume is still mounted.
//!
//! A library can hold roots on a removable drive, and the difference between "this folder is empty" and
//! "the drive is not plugged in" decides whether the app should show 5,000 missing photos or one line saying
//! the drive is away. Answering it per file costs a `stat` per file; answering it per root costs one check.
//!
//! The probe is deliberately textual and table-driven:
//!
//! * Linux reads `/proc/mounts`, which is a real file with a documented format;
//! * macOS uses the `/Volumes/<name>` convention, because there is no `/proc` and `statfs` would need
//!   `libc` (this crate has no dependencies and no `unsafe`);
//! * Windows takes the drive letter from the path.
//!
//! The table parser is pure and tested against all three conventions, so the parts that can be wrong on a
//! machine this project does not run on are still tested here — and [`probe`] says on each platform what it
//! is assuming rather than pretending to a syscall it does not make.

use std::path::{Path, PathBuf};

/// A mounted volume, as far as this project needs to know it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    /// Stable identity of the volume. On Linux the device node, on macOS the mount point, on Windows the
    /// drive letter — the thing that names the same disk from one run to the next.
    pub key: String,
    /// Where it is mounted, so "is it there" can be answered with one check.
    pub mount: PathBuf,
    /// Whether this looks like removable media, so the app can offer a friendlier line than "missing".
    pub removable: bool,
}

/// One line of a mount table: where it is mounted, and what device backs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountPoint {
    /// The device, as the kernel names it.
    pub device: String,
    /// The mount point, with escapes resolved.
    pub mount: String,
}

/// Parse a mount table (`/proc/mounts` and `mount` output both fit).
///
/// Fields are space-separated and a literal space in a path is written `\040`; a backslash is `\134`, a tab
/// `\011` and a newline `\012`. Only the first two fields are read, because the rest describe filesystem
/// options that this crate has no opinion about.
#[must_use]
pub fn parse_mounts(text: &str) -> Vec<MountPoint> {
    let mut mounts = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let (Some(device), Some(mount)) = (fields.next(), fields.next()) else {
            continue;
        };
        mounts.push(MountPoint {
            device: unescape_field(device),
            mount: unescape_field(mount),
        });
    }
    mounts
}

/// Resolve the four escapes the mount table uses for characters that would otherwise break its fields.
fn unescape_field(field: &str) -> String {
    let bytes = field.as_bytes();
    let mut out = String::with_capacity(field.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\'
            && index + 3 < bytes.len()
            && let Ok(value) = u8::from_str_radix(&field[index + 1..index + 4], 8)
            && let Some(character) = char::from_u32(u32::from(value))
        {
            // Only the four documented escapes are resolved; anything else keeps its backslash, because a
            // device name may legitimately contain one on some filesystems.
            if matches!(value, 0o040 | 0o134 | 0o011 | 0o012) {
                out.push(character);
                index += 4;
                continue;
            }
        }
        out.push(bytes[index] as char);
        index += 1;
    }
    out
}

/// The volume a path lives on, given a mount table and the platform's convention.
///
/// `mounts` is the table from [`parse_mounts`]; the longest mount point that prefixes the path wins, which is
/// what makes a bind mount or a nested volume behave.
#[must_use]
pub fn volume_of(path: &Path, mounts: &[MountPoint], windows: bool) -> Volume {
    if windows {
        return volume_of_windows(path);
    }
    let text = path.to_string_lossy();
    let mut best: Option<&MountPoint> = None;
    for mount in mounts {
        let prefixes = text == mount.mount
            || text.starts_with(&format!("{}/", mount.mount.trim_end_matches('/')))
            || (mount.mount == "/" && text.starts_with('/'));
        if prefixes && best.is_none_or(|current| mount.mount.len() > current.mount.len()) {
            best = Some(mount);
        }
    }
    if let Some(mount) = best {
        // `/media/...`, `/run/media/...` and `/mnt/...` are where desktop Linux puts removable media.
        let removable = ["/media/", "/run/media/", "/mnt/"]
            .iter()
            .any(|prefix| mount.mount.starts_with(prefix));
        return Volume {
            key: mount.device.clone(),
            mount: PathBuf::from(&mount.mount),
            removable,
        };
    }
    // macOS: everything under /Volumes is a separate volume, and the ones people unplug live there. The
    // boot volume is `/`, which is never removable.
    let mount = match text.strip_prefix("/Volumes/") {
        Some(rest) => match rest.split('/').next() {
            Some(name) if !name.is_empty() => PathBuf::from(format!("/Volumes/{name}")),
            _ => PathBuf::from("/"),
        },
        None => PathBuf::from("/"),
    };
    let removable = mount != Path::new("/");
    let key = match mount.to_str() {
        Some(mount) => mount.to_string(),
        None => String::from("/"),
    };
    Volume { key, mount, removable }
}

/// Windows: the drive letter is the volume, and there is nothing to read to learn it.
fn volume_of_windows(path: &Path) -> Volume {
    let text = path.to_string_lossy();
    let bytes = text.as_bytes();
    let mount = if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        PathBuf::from(format!("{}:\\", (bytes[0] as char).to_ascii_uppercase()))
    } else {
        PathBuf::from("\\")
    };
    let key = mount.to_string_lossy().to_string();
    // Without `GetDriveType` there is no portable answer; a UNC path is definitely not a local fixed disk,
    // and a letter drive is treated as fixed. Saying "not removable" is a smaller lie than guessing.
    Volume {
        key,
        mount,
        removable: text.starts_with("\\\\"),
    }
}

/// Probe the real machine, with the platform's own source of truth.
///
/// Returns `None` only when there is nothing to probe with — an unreadable mount table on Linux. Callers
/// treat that as "unknown", not as "offline": a missing answer must never be reported as a missing drive.
#[must_use]
pub fn probe(path: &Path) -> Option<Volume> {
    let windows = cfg!(windows);
    let mounts = if windows {
        Vec::new()
    } else if cfg!(target_os = "linux") {
        let text = std::fs::read_to_string("/proc/mounts").ok()?;
        parse_mounts(&text)
    } else {
        Vec::new()
    };
    Some(volume_of(path, &mounts, windows))
}

/// Whether a volume's mount point exists right now.
///
/// One `stat` for the whole root, instead of one per photo: this is the entire point of knowing the volume.
#[must_use]
pub fn mounted(volume: &Volume) -> bool {
    volume.mount.is_dir()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    const LINUX: &str = "\
/dev/sda2 / ext4 rw,relatime 0 0
proc /proc proc rw 0 0
/dev/sdb1 /media/ian/PHOTOS exfat rw,nosuid 0 0
/dev/sdc1 /run/media/ian/CARD\\040SLOT exfat rw 0 0
";

    #[test]
    fn a_mount_table_is_read_field_by_field() {
        let mounts = parse_mounts(LINUX);
        assert_eq!(mounts.len(), 4);
        assert_eq!(mounts[0].device, "/dev/sda2");
        assert_eq!(mounts[0].mount, "/");
        assert_eq!(mounts[2].mount, "/media/ian/PHOTOS");
    }

    /// A path with a space is escaped in the table, and must come back with the space restored.
    #[test]
    fn escaped_spaces_are_resolved_and_nothing_else_is() {
        let mounts = parse_mounts(LINUX);
        assert_eq!(mounts[3].mount, "/run/media/ian/CARD SLOT");
        assert_eq!(unescape_field("/a\\041b"), "/a\\041b", "only the four documented escapes");
        assert_eq!(unescape_field("/a\\134b"), "/a\\b");
        assert_eq!(unescape_field("/a\\04"), "/a\\04", "a short escape is left alone");
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let mounts = parse_mounts("# comment\n\n/dev/sda1 / ext4 rw 0 0\nrubbish\n");
        assert_eq!(mounts.len(), 1);
    }

    /// The longest matching mount wins, which is what makes a nested volume work.
    #[test]
    fn the_longest_mount_point_prefix_wins() {
        let mounts = parse_mounts(LINUX);
        let inside = volume_of(Path::new("/media/ian/PHOTOS/2026/a.jpg"), &mounts, false);
        assert_eq!(inside.key, "/dev/sdb1");
        assert_eq!(inside.mount, PathBuf::from("/media/ian/PHOTOS"));
        assert!(inside.removable, "/media is where desktop Linux puts removable media");

        let root = volume_of(Path::new("/home/ian/Pictures"), &mounts, false);
        assert_eq!(root.mount, PathBuf::from("/"));
        assert!(!root.removable);

        // A prefix only counts at a separator: `/media/ian/PHOTOS-old` is not on the PHOTOS volume, so it
        // falls back to the root volume — which is where it really is on Linux.
        let sibling = volume_of(Path::new("/media/ian/PHOTOS-old/a.jpg"), &mounts, false);
        assert_eq!(sibling.mount, PathBuf::from("/"));
        assert!(
            !sibling.removable,
            "a folder that merely starts like a removable one is not removable"
        );
    }

    /// Without a table, a path under `/Volumes` is its own volume — that is the macOS convention.
    #[test]
    fn the_macos_convention_needs_no_table() {
        let card = volume_of(Path::new("/Volumes/CARD SLOT/DCIM/a.jpg"), &[], false);
        assert_eq!(card.mount, PathBuf::from("/Volumes/CARD SLOT"));
        assert_eq!(card.key, "/Volumes/CARD SLOT");
        assert!(card.removable);

        let boot = volume_of(Path::new("/Users/ian/Pictures"), &[], false);
        assert_eq!(boot.mount, PathBuf::from("/"));
        assert!(!boot.removable);

        let bare = volume_of(Path::new("/Volumes"), &[], false);
        assert_eq!(bare.mount, PathBuf::from("/"), "the folder itself is not a volume");
    }

    #[test]
    fn a_windows_path_takes_its_drive_letter() {
        let drive = volume_of(Path::new("C:\\Photos\\2026\\a.jpg"), &[], true);
        assert_eq!(drive.key, "C:\\");
        assert!(!drive.removable);
        let share = volume_of(Path::new("\\\\nas\\photos\\a.jpg"), &[], true);
        assert!(share.removable, "a network share is not a fixed disk");
    }

    /// Probing the machine this test runs on answers something, and the mount point it names exists.
    #[test]
    fn the_real_probe_finds_a_mounted_volume() {
        let here = std::env::current_dir().expect("cwd");
        let volume = probe(&here).expect("a volume for the working directory");
        assert!(!volume.key.is_empty());
        assert!(volume.mount.is_absolute());
        assert!(mounted(&volume), "the directory we are standing in is on a mounted volume");
    }
}
