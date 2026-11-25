//! The craft-fonts manifest, parsed once and read by two consumers.
//!
//! `crates/ui/build.rs` turns the checkout's `fonts/manifest.txt` into the `CRAFT_FONTS` table
//! at build time, and `cargo xtask fonts` reports and refuses a set by the same rules. Both
//! read this module, so the two cannot disagree about what a line means or which faces a build
//! keeps.
//!
//! The phone's trimmed font set (roadmap 3b) is one of those rules: `CRAFT_FONTS_SCRIPTS` — a
//! comma- or space-separated list of ISO 15924 codes — narrows the embed to the faces the
//! shipped interface languages need. A phone binary then leaves out whole families it could
//! never paint (the manifest's Hangul face, for a build that ships English, Arabic and
//! Japanese), while a desktop build leaves the variable unset and embeds the whole manifest.

#![forbid(unsafe_code)]

/// Scripts the interface itself draws, whatever languages ship: Latin is the interface language
/// and `Zsym` is the symbol coverage behind the keycaps and the arrows. [`crate::fonts`]
/// re-exports this as its own [`crate::fonts::REQUIRED_SCRIPTS`].
pub const REQUIRED_SCRIPTS: &[&str] = &["Latn", "Zsym"];

/// The interface languages that ship with a translation, code by script: what
/// `crate::i18n::LANGUAGES` offers, mapped to the ISO 15924 script each is written in.
pub const SHIPPED_LANGUAGE_SCRIPTS: &[(&str, &str)] = &[("en", "Latn"), ("ar", "Arab"), ("ja", "Jpan"), ("zh-TW", "Hani")];

/// The scripts a phone build's filter must name: the interface's own two plus every shipped
/// language's. `cargo xtask mobile` passes exactly this list as `CRAFT_FONTS_SCRIPTS`, and the
/// test below holds it to that definition so adding a language updates both.
pub const PHONE_SCRIPTS: &[&str] = &["Latn", "Zsym", "Arab", "Jpan", "Hani"];

/// One non-comment line of craft-fonts' `fonts/manifest.txt`.
///
/// The format is craft-fonts': `family | style | file | scripts | …`, with `scripts`
/// comma-separated ISO 15924 codes. The remaining fields (weight, width, licence name) are
/// craft-fonts' own bookkeeping; nothing here needs them.
#[derive(Debug, PartialEq, Eq)]
pub struct Face<'a> {
    /// Family name, spelled as craft-fonts spells it (`NanumGothic`, one word).
    pub family: &'a str,
    /// Style within the family, e.g. `Regular`.
    pub style: &'a str,
    /// Font file, relative to the checkout root.
    pub file: &'a str,
    /// ISO 15924 scripts this face is for.
    pub scripts: Vec<&'a str>,
}

/// Parse every non-empty, non-comment line of a manifest.
pub fn parse(manifest: &str) -> Result<Vec<Face<'_>>, String> {
    let mut faces = Vec::new();
    for line in manifest
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let fields: Vec<&str> = line.split(" | ").map(str::trim).collect();
        let [family, style, file, scripts, ..] = fields.as_slice() else {
            return Err(format!("malformed manifest line: {line}"));
        };
        faces.push(Face {
            family,
            style,
            file,
            scripts: scripts.split(',').map(str::trim).filter(|script| !script.is_empty()).collect(),
        });
    }
    Ok(faces)
}

/// Read a `CRAFT_FONTS_SCRIPTS` value: comma- or space-separated codes; unset or blank means
/// "no filter" — embed the whole manifest, as the desktop does.
pub fn parse_filter(value: Option<&str>) -> Vec<&str> {
    value
        .map(|value| {
            value
                .split([',', ' ', ';'])
                .map(str::trim)
                .filter(|script| !script.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// The faces a build embeds: all of them, or — when a script filter is set — only the faces
/// covering at least one requested script.
///
/// A requested script no face covers is refused by name, because it is almost always a typo
/// (`Lant` for `Latn`), and a filter that keeps nothing would ship a build with no face at all.
pub fn select<'a>(faces: &'a [Face<'a>], filter: &[&str]) -> Result<Vec<&'a Face<'a>>, String> {
    if filter.is_empty() {
        return Ok(faces.iter().collect());
    }
    for script in filter {
        if !faces.iter().any(|face| face.scripts.contains(script)) {
            return Err(format!("the manifest has no face for {script} (CRAFT_FONTS_SCRIPTS)"));
        }
    }
    // Every requested script matched at least one face, so at least one face is kept; no
    // further emptiness check is reachable.
    Ok(faces
        .iter()
        .filter(|face| face.scripts.iter().any(|script| filter.contains(script)))
        .collect())
}

/// Whether the kept faces cover a script.
fn covers(faces: &[&Face<'_>], script: &str) -> bool {
    faces.iter().any(|face| face.scripts.contains(&script))
}

/// What a build would ship, checked: the kept faces must cover the interface's own scripts and
/// every shipped language's. Returns the refusal sentences, empty when the set is whole.
///
/// `cargo xtask fonts` prints these and fails when there are any; `build.rs` has no second
/// reader to disagree with, but a release that embeds a set a test would refuse should never
/// leave the machine.
pub fn validate(manifest: &str, filter: &[&str]) -> Result<Vec<String>, String> {
    let faces = parse(manifest)?;
    let kept = select(&faces, filter)?;
    let mut problems = Vec::new();
    let mut required: Vec<&str> = REQUIRED_SCRIPTS.to_vec();
    for (_, script) in SHIPPED_LANGUAGE_SCRIPTS {
        if !required.contains(script) {
            required.push(script);
        }
    }
    for script in required {
        if !covers(&kept, script) {
            problems.push(format!("the embedded set does not cover {script}"));
        }
    }
    Ok(problems)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// The shape of the real manifest: Japanese with Latin, an Arabic face for the shipped
    /// translation, Hangul the shipped languages do not use, and the two symbol faces.
    const MANIFEST: &str = "BIZ UDPGothic | Regular | ja/BIZUDPGothic-Regular.ttf | Jpan,Latn\n\
                            BIZ UDPGothic | Bold | ja/BIZUDPGothic-Bold.ttf | Jpan,Latn\n\
                            Noto Naskh Arabic | Regular | ar/NotoNaskhArabic-Regular.ttf | Arab\n\
                            Noto Sans TC | Regular | zh-TW/NotoSansTC-Regular.ttf | Hani\n\
                            NanumGothic | Regular | ko/NanumGothic-Regular.ttf | Hang,Kore\n\
                            Noto Sans Symbols | Regular | symbols/NotoSansSymbols-Regular.ttf | Zsym\n\
                            Noto Sans Symbols 2 | Regular | symbols/NotoSansSymbols2-Regular.ttf | Zsym\n";

    #[test]
    fn parse_reads_every_face_and_skips_comments() {
        let faces = parse(MANIFEST).expect("a well-formed manifest parses");
        assert_eq!(faces.len(), 7);
        assert_eq!(faces[0].family, "BIZ UDPGothic");
        assert_eq!(faces[0].scripts, vec!["Jpan", "Latn"]);
        assert_eq!(faces[3].file, "zh-TW/NotoSansTC-Regular.ttf");
        assert_eq!(faces[4].file, "ko/NanumGothic-Regular.ttf");
        assert_eq!(parse("# only a comment\n").expect("comments are not faces"), Vec::new());
        assert!(parse("family | style\n").is_err(), "a line without scripts is malformed");
    }

    #[test]
    fn no_filter_keeps_the_whole_manifest() {
        let faces = parse(MANIFEST).expect("parses");
        let kept = select(&faces, &[]).expect("no filter keeps everything");
        assert_eq!(kept.len(), 7);
        assert!(kept.iter().any(|face| face.family == "NanumGothic"));
    }

    #[test]
    fn the_phone_filter_leaves_out_the_faces_no_shipped_language_uses() {
        let faces = parse(MANIFEST).expect("parses");
        let kept = select(&faces, PHONE_SCRIPTS).expect("the phone filter matches the manifest");
        let families: Vec<&str> = kept.iter().map(|face| face.family).collect();
        assert!(families.contains(&"BIZ UDPGothic"), "Japanese ships on the phone");
        assert!(families.contains(&"Noto Sans Symbols"), "Zsym is required");
        assert!(families.contains(&"Noto Sans TC"), "Traditional Chinese ships on the phone");
        assert!(!families.contains(&"NanumGothic"), "no shipped language is Korean");
        assert_eq!(kept.len(), 6);
    }

    #[test]
    fn a_typo_in_the_filter_is_refused_by_name() {
        let faces = parse(MANIFEST).expect("parses");
        let error = select(&faces, &["Latn", "Lant"]).expect_err("Lant is not a script the manifest has");
        assert!(error.contains("Lant"), "{error}");
    }

    #[test]
    fn a_script_no_face_covers_is_refused_even_when_others_survive() {
        let faces = parse("NanumGothic | Regular | ko/NanumGothic-Regular.ttf | Hang,Kore\n").expect("parses");
        let error = select(&faces, &["Hang", "Latn"]).expect_err("no face covers Latn here");
        assert!(error.contains("no face for Latn"), "{error}");
    }

    #[test]
    fn the_filter_value_is_comma_or_space_separated_and_blank_means_no_filter() {
        assert_eq!(parse_filter(None), Vec::<&str>::new());
        assert_eq!(parse_filter(Some("  ")), Vec::<&str>::new());
        assert_eq!(parse_filter(Some("Latn,Zsym")), vec!["Latn", "Zsym"]);
        assert_eq!(parse_filter(Some("Latn Zsym")), vec!["Latn", "Zsym"]);
        assert_eq!(parse_filter(Some(" Latn, Zsym ,Jpan")), vec!["Latn", "Zsym", "Jpan"]);
    }

    #[test]
    fn validation_refuses_a_set_that_drops_a_shipped_script() {
        assert_eq!(
            validate(MANIFEST, &[]).expect("the whole manifest covers everything"),
            Vec::<String>::new()
        );
        assert_eq!(
            validate(MANIFEST, PHONE_SCRIPTS).expect("the phone filter covers everything"),
            Vec::<String>::new()
        );
        let problems = validate(MANIFEST, &["Latn", "Zsym"]).expect("the filter itself is well-formed");
        // The Latin request keeps BIZ UDPGothic, which covers Jpan too; Arabic and Traditional
        // Chinese have no face left.
        assert_eq!(
            problems,
            vec!["the embedded set does not cover Arab", "the embedded set does not cover Hani"]
        );
        assert!(
            validate(MANIFEST, &["Latn", "Zsym", "Deva"]).is_err(),
            "a filter naming a script the manifest lacks is refused before coverage"
        );
    }

    #[test]
    fn the_phone_scripts_are_the_required_ones_plus_every_shipped_language() {
        let mut expected: Vec<&str> = REQUIRED_SCRIPTS.to_vec();
        for (_, script) in SHIPPED_LANGUAGE_SCRIPTS {
            if !expected.contains(script) {
                expected.push(*script);
            }
        }
        assert_eq!(PHONE_SCRIPTS, expected.as_slice());
    }
}
