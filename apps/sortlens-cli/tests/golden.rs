//! Golden outputs: what the tool says, compared byte for byte.
//!
//! A unit test asserts a function's result. This asserts the *published* output — the text an agent,
//! a script or a person reads — so a renamed JSON field, a reordered counter or a reworded summary
//! shows up as a diff instead of shipping quietly.
//!
//! The fixture is built here rather than committed as binaries: `sortlens-testkit` writes real JPEGs
//! from a real encoder, so a photo in these goldens decodes, hashes and has a resolution.
//!
//! Rewrite the goldens with `SORTLENS_UPDATE_GOLDEN=1` (or `cargo xtask snapshot --update`), then
//! **read the diff** — that is the whole point of the file.

use sortlens_testkit::Fixture;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const UPDATE_ENV: &str = "SORTLENS_UPDATE_GOLDEN";

/// One recorded invocation.
///
/// `%PHOTOS%` in an argument is replaced with the fixture folder at run time, so a step can name an
/// absolute destination without the golden depending on where the fixture landed.
struct Step {
    /// Also the golden file's name.
    name: &'static str,
    args: &'static [&'static str],
    /// When set, record this file from the library instead of the command's output.
    file: Option<&'static str>,
}

/// The script. Order matters: later steps see the library earlier steps left behind.
const STEPS: &[Step] = &[
    Step {
        name: "01 add photos",
        args: &["add", "Photos"],
        file: None,
    },
    Step {
        name: "02 add archive",
        args: &["add", "Archive"],
        file: None,
    },
    Step {
        name: "03 roots",
        args: &["roots"],
        file: None,
    },
    Step {
        name: "04 stats",
        args: &["stats"],
        file: None,
    },
    Step {
        name: "05 list",
        args: &["list", "--limit", "10"],
        file: None,
    },
    Step {
        name: "06 keywords",
        args: &["keywords"],
        file: None,
    },
    Step {
        name: "07 folders",
        args: &["folders"],
        file: None,
    },
    Step {
        name: "08 check",
        args: &["check"],
        file: None,
    },
    Step {
        name: "09 changed",
        args: &["changed"],
        file: None,
    },
    Step {
        name: "10 rate",
        args: &["rate", "1", "5"],
        file: None,
    },
    Step {
        name: "11 tag",
        args: &["tag", "1", "coast"],
        file: None,
    },
    Step {
        name: "12 rename",
        args: &["rename", "coast", "beach"],
        file: None,
    },
    Step {
        name: "13 registry",
        args: &["registry"],
        file: None,
    },
    Step {
        name: "14 alias",
        args: &["alias", "beach", "shore"],
        file: None,
    },
    Step {
        name: "15 registry again",
        args: &["registry"],
        file: None,
    },
    Step {
        name: "16 untag through the alias",
        args: &["untag", "2", "shore"],
        file: None,
    },
    Step {
        name: "17 date",
        args: &["date", "1", "2023-06-01"],
        file: None,
    },
    Step {
        name: "18 date again",
        args: &["date", "2", "2024-06-01"],
        file: None,
    },
    Step {
        name: "19 memories",
        args: &["memories", "--today", "2024-06-01"],
        file: None,
    },
    Step {
        name: "20 insights",
        args: &["insights"],
        file: None,
    },
    Step {
        name: "21 page",
        args: &["page", "--limit", "3"],
        file: None,
    },
    Step {
        name: "22 undo",
        args: &["undo"],
        file: None,
    },
    Step {
        name: "23 history",
        args: &["history", "--limit", "8"],
        file: None,
    },
    Step {
        name: "24 shortcuts",
        args: &["shortcuts"],
        file: None,
    },
    Step {
        name: "25 move",
        args: &["move", "Photos/Moved", "1", "2"],
        file: None,
    },
    Step {
        name: "26 moves",
        args: &["moves"],
        file: None,
    },
    Step {
        name: "27 unmove",
        args: &["unmove", "1"],
        file: None,
    },
    Step {
        name: "28 audit",
        args: &["audit"],
        file: None,
    },
    Step {
        name: "29 compact",
        args: &["compact"],
        file: None,
    },
    Step {
        name: "33 export",
        args: &["export", "%PHOTOS%/Export", "1", "2"],
        file: None,
    },
    Step {
        name: "34 export again",
        args: &[
            "export",
            "%PHOTOS%/Export",
            "1",
            "2",
            "--format",
            "jpeg",
            "--max",
            "24",
            "--collision",
            "rename",
        ],
        file: None,
    },
    Step {
        name: "30 suggest",
        args: &["suggest", "1"],
        file: None,
    },
    Step {
        name: "31 accept a suggestion",
        args: &["accept", "1", "low light"],
        file: None,
    },
    Step {
        name: "32 accept it again",
        args: &["accept", "1", "low light"],
        file: None,
    },
    Step {
        name: "35 snapshot",
        args: &[],
        file: Some("snapshot.json"),
    },
    Step {
        name: "36 packs",
        args: &["packs"],
        file: None,
    },
    Step {
        name: "37 batch suggest",
        args: &["batchsuggest"],
        file: None,
    },
    // The JSON schema is a published surface: a script depends on these keys, so a rename here is a
    // breaking change and the golden diff is where it gets noticed.
    Step {
        name: "38 stats as JSON",
        args: &["stats", "--json"],
        file: None,
    },
    Step {
        name: "39 list as JSON",
        args: &["--json", "list", "--limit", "2"],
        file: None,
    },
    Step {
        name: "40 roots as JSON",
        args: &["roots", "--json"],
        file: None,
    },
    Step {
        name: "41 keywords as JSON",
        args: &["keywords", "--json"],
        file: None,
    },
    // The diagnostics bundle is a published artefact: its section names are what a maintainer greps, and
    // the report must never contain the path this test library lives at.
    Step {
        name: "44 version",
        args: &["--version"],
        file: None,
    },
    Step {
        name: "43 diagnostics",
        args: &["diagnostics"],
        file: None,
    },
    Step {
        name: "42 a verb with no schema says so",
        args: &["undo", "--json"],
        file: None,
    },
];

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_sortlens-cli")
}

/// Replace everything that legitimately differs between runs.
///
/// Only three things are allowed to vary: where the fixture lives, how long a scan took, and a
/// file's modification time. Everything
/// else in the output is the thing under test, so keeping this list short is the point.
fn normalise(text: &str, fixture: &Path, library: &Path) -> String {
    let mut out = text.replace(&library.display().to_string(), "<library>");
    out = out.replace(&fixture.display().to_string(), "<photos>");
    // `… in 0.0s` and `… in 2.4s` are the same fact measured twice.
    let mut cleaned = String::new();
    let mut rest = out.as_str();
    while let Some(index) = rest.find(" in ") {
        cleaned.push_str(&rest[..index]);
        let tail = &rest[index + 4..];
        match tail.find('s') {
            Some(end) if tail[..end].chars().all(|c| c.is_ascii_digit() || c == '.') && end > 0 => {
                cleaned.push_str(" in <elapsed>s");
                rest = &tail[end + 1..];
            }
            _ => {
                cleaned.push_str(" in ");
                rest = tail;
            }
        }
    }
    cleaned.push_str(rest);
    let mut stable = String::new();
    let mut rest = cleaned.as_str();
    // A file's modification time is a property of the machine, not of the library. JSON printed by
    // scripts has a space after the colon, so both spellings are normalised.
    while let Some(index) = rest.find("\"modified\":") {
        stable.push_str(&rest[..index]);
        let tail = &rest[index + "\"modified\":".len()..];
        let offset = tail.len() - tail.trim_start().len();
        let spaced = offset > 0;
        let digits = tail[offset..].chars().take_while(char::is_ascii_digit).count();
        if digits > 0 {
            stable.push_str(if spaced { "\"modified\": <mtime>" } else { "\"modified\":<mtime>" });
            rest = &tail[offset + digits..];
        } else {
            stable.push_str("\"modified\":");
            rest = tail;
        }
    }
    stable.push_str(rest);
    // The diagnostics report records what the machine is, not what the library is: a golden that pinned
    // "os = macos" would fail on the ubuntu job for a reason that says nothing about the code.
    let mut portable = String::new();
    for line in stable.split_inclusive('\n') {
        let host = ["os = ", "arch = "].iter().find(|prefix| line.starts_with(**prefix)).copied();
        let version = ["sortlens = ", "engine = "]
            .iter()
            .find(|prefix| line.starts_with(**prefix))
            .copied();
        match (host, version) {
            (Some(prefix), _) => {
                portable.push_str(prefix);
                portable.push_str("<host>\n");
            }
            (None, Some(prefix)) => {
                portable.push_str(prefix);
                portable.push_str("<version>\n");
            }
            (None, None) => portable.push_str(line),
        }
    }
    stable = portable;

    let mut cleaned = stable;
    // A trailing newline is not worth a diff.
    while cleaned.ends_with('\n') || cleaned.ends_with(' ') {
        cleaned.pop();
    }
    cleaned.push('\n');
    cleaned
}

fn build_fixture() -> Fixture {
    let fixture = Fixture::new("golden");
    // A small tree with a nested folder, so `folders` and subfolder drill-down have something real.
    fixture.jpeg("Photos/Trip/a.jpg", [210, 60, 40], 48, 32);
    fixture.jpeg("Photos/Trip/b.jpg", [210, 60, 40], 48, 32);
    fixture.jpeg("Photos/Trip/Day2/c.jpg", [40, 90, 210], 40, 40);
    fixture.jpeg("Photos/Studio/d.jpg", [120, 120, 120], 32, 48);
    // A second folder outside the first root, because roots are the unit of a scan.
    fixture.jpeg("Archive/e.jpg", [90, 200, 90], 24, 24);
    // A raw whose contents are really a JPEG preview: the scan routes it by its header and says so in the
    // `add` line, so the golden pins both the correction and the fact that it is reported.
    fixture.jpeg("Photos/Trip/preview.cr2", [200, 120, 40], 48, 32);
    // A sidecar the scan should import: a rating and a keyword written by another application.
    fixture.sidecar(
        "Photos/Trip/a.jpg",
        r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description xmlns:xmp="http://ns.adobe.com/xap/1.0/"
                     xmlns:dc="http://purl.org/dc/elements/1.1/"
                     xmp:Rating="3">
      <dc:subject>
        <rdf:Bag>
          <rdf:li>harbour</rdf:li>
        </rdf:Bag>
      </dc:subject>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#,
    );
    fixture
}

fn record(step: &Step, library: &Path, fixture_root: &Path) -> String {
    if let Some(file) = step.file {
        let path = library.join(file);
        return match fs::read_to_string(&path) {
            Ok(text) => normalise(&text, fixture_root, library),
            Err(error) => format!("<missing: {file}: {error}>\n"),
        };
    }
    let args: Vec<String> = step
        .args
        .iter()
        .map(|arg| arg.replace("%PHOTOS%", &fixture_root.display().to_string()))
        .collect();
    let output = Command::new(binary())
        .arg("--library")
        .arg(library)
        .args(&args)
        .current_dir(fixture_root)
        .output();
    match output {
        Ok(output) => {
            let mut text = String::new();
            text.push_str(&format!("exit {}\n", output.status.code().unwrap_or(-1)));
            text.push_str("--- stdout\n");
            text.push_str(&String::from_utf8_lossy(&output.stdout));
            text.push_str("--- stderr\n");
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            normalise(&text, fixture_root, library)
        }
        Err(error) => format!("<could not run {}: {error}>\n", binary()),
    }
}

/// `--version` has to answer on a machine that has never run the app, so it must not open — let alone
/// create — a library on the way. The golden pins the text; this pins the side effect.
#[test]
fn asking_for_the_version_does_not_create_a_library() {
    let folder = Fixture::new("golden-version");
    let library = folder.path("never-created");
    assert!(!library.exists());
    let output = Command::new(binary())
        .env("SORTLENS_LIBRARY_DIR", &library)
        .arg("--version")
        .output()
        .expect("run the binary");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert_eq!(text.trim(), format!("sortlens-cli {}", sortlens_core::VERSION));
    assert!(!library.exists(), "asking what version this is created {}", library.display());
}

/// The golden normaliser replaces the library and fixture paths so a diff is about the tool, not the
/// machine — which means a *leak* of those paths would be hidden from the golden diff. This test reads the
/// diagnostics output before any normalisation and says the redaction actually happened.
#[test]
fn the_diagnostics_bundle_contains_no_absolute_paths() {
    let fixture = build_fixture();
    let library = Fixture::new("golden-redaction");
    let output = Command::new(binary())
        .arg("--library")
        .arg(library.root())
        .arg("diagnostics")
        .current_dir(fixture.root())
        .output()
        .expect("run the binary");
    assert!(output.status.success(), "diagnostics should succeed");
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    for path in [library.root(), fixture.root()] {
        let path = path.display().to_string();
        assert!(!text.contains(&path), "the report leaked {path}:\n{text}");
    }
    assert!(text.contains("<library>"), "the library is named by its placeholder");
    assert!(text.contains("<root-"), "each root is named by its placeholder");
    assert!(text.contains("[health]"), "the report has its sections");
}

#[test]
fn every_published_output_is_unchanged() {
    let fixture = build_fixture();
    let library = Fixture::new("golden-library");
    let goldens = golden_dir();
    let update = std::env::var_os(UPDATE_ENV).is_some();
    if update {
        fs::create_dir_all(&goldens).expect("golden folder");
    }

    let mut failures = Vec::new();
    for step in STEPS {
        let actual = record(step, library.root(), fixture.root());
        let path = goldens.join(format!("{}.txt", step.name));
        if update {
            fs::write(&path, &actual).expect("write golden");
            continue;
        }
        match fs::read_to_string(&path) {
            Ok(expected) => {
                if expected != actual {
                    failures.push(format!("── {} ──\n--- expected\n{}\n--- actual\n{}\n", step.name, expected, actual));
                }
            }
            Err(error) => failures.push(format!("── {} ──\nno golden file at {}: {error}\n", step.name, path.display())),
        }
    }

    // Both directions: a golden whose step was deleted is a stale claim about the CLI, and it would
    // otherwise sit there passing forever.
    if !update {
        let known: Vec<String> = STEPS.iter().map(|step| format!("{}.txt", step.name)).collect();
        if let Ok(entries) = fs::read_dir(&goldens) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".txt") && !known.contains(&name) {
                    failures.push(format!(
                        "── {name} ──\nno step records this golden any more; delete it or restore the step\n"
                    ));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "\n{} golden output(s) changed. If the change is intended, run\n  \
         cargo xtask snapshot --update\nthen read the diff before committing it.\n{}",
        failures.len(),
        failures.join("")
    );
}
