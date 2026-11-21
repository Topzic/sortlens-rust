//! XMP sidecars: read, merge and write, preserving fields other applications own.
//!
//! Sortlens never rewrites a sidecar from scratch. It edits the `rdf:Description` element in place,
//! so a rating or a crop made in another application survives a Sortlens cull.
//!
//! The contract is normative in `docs/BEHAVIOUR.md` §4: `IMG_1234.CR3` → `IMG_1234.xmp` beside it,
//! UTF-8 with an `xpacket` wrapper, one description, atomic replacement through a temp file.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

use sortlens_core::{ColorLabel, Error, Flag, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// The XMP namespace Sortlens owns for its own fields.
pub const SORTLENS_NS: &str = "https://sortlens.app/ns/1.0/";

/// The review state a sidecar carries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sidecar {
    /// Stars, 0–5. Written as `xmp:Rating`.
    pub rating: u8,
    /// The cull decision. Written as `sortlens:Decision`.
    pub flag: Flag,
    /// Colour label. Written as `xmp:Label`.
    pub label: Option<ColorLabel>,
    /// Keywords. Written as a `dc:subject` bag.
    pub keywords: Vec<String>,
}

impl Sidecar {
    /// Build from the catalog's view of a photo.
    pub fn new(rating: u8, flag: Flag, label: Option<ColorLabel>, mut keywords: Vec<String>) -> Self {
        keywords.sort_unstable();
        keywords.dedup();
        Self {
            rating: rating.min(5),
            flag,
            label,
            keywords,
        }
    }
}

/// Whether a sidecar carries edit settings from another application.
///
/// A sidecar alone means keywords or a rating; `crs:` (Camera Raw settings), a history block or a
/// develop recipe mean the pixels were edited somewhere. That is the difference the "edited" filter
/// needs, and it is a property of the sidecar rather than of any one field Sortlens writes.
pub fn has_edits(text: &str) -> bool {
    const MARKERS: &[&str] = &[
        "crs:",
        "xmpMM:History",
        "<xmp:History>",
        "xmp:History",
        "darktable:",
        "photoshop:History",
    ];
    MARKERS.iter().any(|marker| text.contains(marker))
}

/// Where the sidecar for an image lives: same folder, same stem, `.xmp`.
pub fn path_for(image: &Path) -> PathBuf {
    image.with_extension("xmp")
}

/// Read the sidecar text beside an image, for checks that need the whole document.
pub fn read_text(image: &Path) -> Option<String> {
    fs::read_to_string(path_for(image)).ok()
}

/// Read the sidecar beside an image. `Ok(None)` when there is none.
pub fn read(image: &Path) -> Result<Option<Sidecar>> {
    let path = path_for(image);
    match fs::read_to_string(&path) {
        Ok(text) => Ok(Some(parse(&text))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Error::io(path.display().to_string(), error)),
    }
}

/// Write the sidecar beside an image, merging into an existing one.
///
/// The file is only touched when its content would change, so re-saving one photo does not churn
/// the disk for a whole selection.
pub fn write(image: &Path, sidecar: &Sidecar) -> Result<bool> {
    let path = path_for(image);
    let existing = fs::read_to_string(&path).ok();
    let text = merge(existing.as_deref(), sidecar);
    if existing.as_deref() == Some(text.as_str()) {
        return Ok(false);
    }
    atomic_write(&path, &text)?;
    Ok(true)
}

fn atomic_write(path: &Path, text: &str) -> Result<()> {
    let temp = path.with_extension("xmp.tmp");
    let fail = |error: std::io::Error| Error::io(temp.display().to_string(), error);
    fs::write(&temp, text).map_err(fail)?;
    fs::rename(&temp, path).map_err(|error| {
        let _ = fs::remove_file(&temp);
        Error::io(path.display().to_string(), error)
    })
}

/// Read a sidecar's review state out of XMP text.
///
/// Unknown fields are ignored; the standard attribute form and the less common element form are
/// both understood, because other applications emit both.
pub fn parse(text: &str) -> Sidecar {
    let rating = value_of(text, "xmp:Rating")
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(0)
        .min(5);
    let label = value_of(text, "xmp:Label").and_then(|value| label_from_xmp(&value));
    let flag = match value_of(text, "sortlens:Decision").as_deref() {
        Some("pick") => Flag::Pick,
        Some("reject") => Flag::Reject,
        _ => Flag::None,
    };
    Sidecar {
        rating,
        flag,
        label,
        keywords: keywords_of(text),
    }
}

fn label_from_xmp(value: &str) -> Option<ColorLabel> {
    ColorLabel::ALL
        .into_iter()
        .find(|label| label.label().eq_ignore_ascii_case(value.trim()))
}

/// Produce the sidecar document, editing `existing` when there is one.
pub fn merge(existing: Option<&str>, sidecar: &Sidecar) -> String {
    match existing.filter(|text| text.contains("<rdf:Description")) {
        Some(text) => patch(text, sidecar),
        None => fresh(sidecar),
    }
}

fn fresh(sidecar: &Sidecar) -> String {
    let mut attributes = vec![
        "xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"".to_string(),
        "xmlns:dc=\"http://purl.org/dc/elements/1.1/\"".to_string(),
        format!("xmlns:sortlens=\"{SORTLENS_NS}\""),
    ];
    attributes.extend(owned_attributes(sidecar));
    let children = keywords_block(sidecar);
    let mut text = String::from("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n");
    text.push_str("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"Sortlens\">\n");
    text.push_str(" <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n");
    text.push_str("  <rdf:Description rdf:about=\"\"\n");
    for attribute in &attributes {
        text.push_str("    ");
        text.push_str(attribute);
        text.push('\n');
    }
    text.push_str("  >\n");
    if !children.is_empty() {
        text.push_str(&children);
        text.push('\n');
    }
    text.push_str("  </rdf:Description>\n");
    text.push_str(" </rdf:RDF>\n");
    text.push_str("</x:xmpmeta>\n");
    text.push_str("<?xpacket end=\"w\"?>");
    text
}

fn owned_attributes(sidecar: &Sidecar) -> Vec<String> {
    let mut attributes = vec![format!("xmp:Rating=\"{}\"", sidecar.rating)];
    if let Some(label) = sidecar.label {
        attributes.push(format!("xmp:Label=\"{}\"", label.label()));
    }
    if sidecar.flag != Flag::None {
        attributes.push(format!("sortlens:Decision=\"{}\"", sidecar.flag.id()));
    }
    attributes
}

fn keywords_block(sidecar: &Sidecar) -> String {
    if sidecar.keywords.is_empty() {
        return String::new();
    }
    let mut block = String::from("   <dc:subject>\n    <rdf:Bag>\n");
    for keyword in &sidecar.keywords {
        block.push_str("     <rdf:li>");
        block.push_str(&escape(keyword));
        block.push_str("</rdf:li>\n");
    }
    block.push_str("    </rdf:Bag>\n   </dc:subject>");
    block
}

/// Edit the first `rdf:Description` element of an existing document.
fn patch(text: &str, sidecar: &Sidecar) -> String {
    let Some(start) = text.find("<rdf:Description") else {
        return fresh(sidecar);
    };
    let Some(end) = tag_end(text, start) else {
        return fresh(sidecar);
    };
    // Split the start tag into its content, the whitespace before the closing bracket and whether
    // it closes itself. The whitespace is preserved so a second write is byte-identical.
    let element = &text[start..end - 1];
    let without_slash = element.trim_end();
    let self_closing = without_slash.ends_with('/');
    let content = without_slash.trim_end_matches('/');
    let content = if self_closing { content } else { element };
    let trailing = &content[content.trim_end().len()..];
    let mut tag = content.trim_end().to_string();

    tag = set_attribute(tag, "xmp:Rating", Some(&sidecar.rating.to_string()));
    tag = set_attribute(tag, "xmp:Label", sidecar.label.map(ColorLabel::label));
    tag = set_attribute(tag, "sortlens:Decision", (sidecar.flag != Flag::None).then(|| sidecar.flag.id()));

    let mut body = String::new();
    let mut rest = text[end..].to_string();
    let existing_block = subject_range(text, start);
    let children = keywords_block(sidecar);

    let mut head = text[..start].to_string();
    match existing_block {
        // Replace the keyword bag in place, keeping the surrounding layout.
        Some((block_start, block_end)) => {
            // The block brings its own indentation; drop the whitespace that preceded the old one
            // so a second write produces byte-identical output.
            let mut middle = text[end..block_start].trim_end_matches(' ').to_string();
            middle.push_str(&children);
            middle.push_str(&text[block_end..]);
            rest = middle;
            head.push_str(&tag);
            head.push_str(trailing);
            head.push('>');
            return format!("{head}{rest}");
        }
        None if !children.is_empty() => {
            head.push_str(&tag);
            if self_closing {
                head.push_str(trailing);
                head.push_str(">\n");
                head.push_str(&children);
                head.push_str("\n  </rdf:Description>");
            } else {
                head.push_str(trailing);
                head.push_str(">\n");
                head.push_str(&children);
                head.push_str(&rest);
                return head;
            }
        }
        None => {
            head.push_str(&tag);
            head.push_str(trailing);
            head.push_str(if self_closing { "/>" } else { ">" });
        }
    }
    body.push_str(&rest);
    head.push_str(&body);
    head
}

/// The byte range of the first `dc:subject` element, when the document has one.
fn subject_range(text: &str, from: usize) -> Option<(usize, usize)> {
    let start = text[from..].find("<dc:subject")? + from;
    let open_end = tag_end(text, start)?;
    let close = text[open_end..].find("</dc:subject>")? + open_end;
    Some((start, close + "</dc:subject>".len()))
}

/// Index just past the `>` that ends the tag starting at `start`.
fn tag_end(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut quote = false;
    for (offset, byte) in bytes.iter().enumerate().skip(start) {
        match byte {
            b'"' => quote = !quote,
            b'>' if !quote => return Some(offset + 1),
            _ => {}
        }
    }
    None
}

/// Set or remove one attribute on an open element tag (text without its final `>`).
fn set_attribute(tag: String, name: &str, value: Option<&str>) -> String {
    let needle = format!("{name}=\"");
    if let Some(position) = tag.find(&needle) {
        let value_start = position + needle.len();
        let value_end = tag[value_start..].find('"').map(|offset| value_start + offset);
        return match (value, value_end) {
            (Some(value), Some(end)) => format!("{}{}{}", &tag[..value_start], escape(value), &tag[end..]),
            (Some(value), None) => format!("{}{value}\"", &tag[..value_start]),
            (None, Some(end)) => {
                let mut trimmed = tag[..position].trim_end().to_string();
                trimmed.push_str(&tag[end + 1..]);
                trimmed
            }
            (None, None) => tag[..position].trim_end().to_string(),
        };
    }
    match value {
        Some(value) => {
            let mut tag = ensure_namespace(tag, name);
            tag.push_str(&format!("\n    {name}=\"{}\"", escape(value)));
            tag
        }
        None => tag,
    }
}

/// Declare the namespace an attribute needs if the document does not declare it yet.
fn ensure_namespace(tag: String, attribute: &str) -> String {
    let (prefix, uri) = if attribute.starts_with("sortlens:") {
        ("sortlens", SORTLENS_NS)
    } else {
        ("xmp", "http://ns.adobe.com/xap/1.0/")
    };
    if tag.contains(&format!("xmlns:{prefix}=")) {
        return tag;
    }
    format!("{tag}\n    xmlns:{prefix}=\"{uri}\"")
}

fn value_of(text: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    if let Some(position) = text.find(&needle) {
        let start = position + needle.len();
        let end = text[start..].find('"')? + start;
        return Some(unescape(&text[start..end]));
    }
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(unescape(&text[start..end]))
}

fn keywords_of(text: &str) -> Vec<String> {
    let mut keywords = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("<rdf:li>") {
        let start = open + "<rdf:li>".len();
        let Some(close) = rest[start..].find("</rdf:li>") else {
            break;
        };
        keywords.push(unescape(&rest[start..start + close]));
        rest = &rest[start + close + "</rdf:li>".len()..];
    }
    keywords.sort_unstable();
    keywords.dedup();
    keywords
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn unescape(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sortlens-sidecar-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn sample() -> Sidecar {
        Sidecar::new(4, Flag::Pick, Some(ColorLabel::Green), vec!["wildlife".into(), "beach".into()])
    }

    #[test]
    fn a_fresh_sidecar_round_trips() {
        let text = merge(None, &sample());
        assert!(text.contains("xmp:Rating=\"4\""));
        assert!(text.contains("xmp:Label=\"Green\""));
        assert!(text.contains("sortlens:Decision=\"pick\""));
        assert!(text.contains("<rdf:li>beach</rdf:li>"));
        assert_eq!(parse(&text), sample());
    }

    #[test]
    fn a_foreign_sidecar_keeps_the_fields_we_do_not_own() {
        let existing = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
    xmp:Rating="2"
    crs:Exposure2012="+0.75"
    xmp:CreatorTool="Lightroom Classic">
   <dc:subject xmlns:dc="http://purl.org/dc/elements/1.1/">
    <rdf:Bag>
     <rdf:li>old-keyword</rdf:li>
    </rdf:Bag>
   </dc:subject>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;
        let text = merge(Some(existing), &sample());
        assert!(text.contains("crs:Exposure2012=\"+0.75\""), "crop settings survive: {text}");
        assert!(text.contains("xmp:CreatorTool=\"Lightroom Classic\""));
        assert!(text.contains("xmp:Rating=\"4\""));
        assert!(!text.contains("old-keyword"));
        assert!(text.contains("<rdf:li>wildlife</rdf:li>"));
        assert!(text.contains("sortlens:Decision=\"pick\""));
        assert!(text.contains(&format!("xmlns:sortlens=\"{SORTLENS_NS}\"")));

        let read_back = parse(&text);
        assert_eq!(read_back.rating, 4);
        assert_eq!(read_back.label, Some(ColorLabel::Green));
        assert_eq!(read_back.keywords, vec!["beach".to_string(), "wildlife".to_string()]);
    }

    #[test]
    fn a_self_closing_description_gains_children_correctly() {
        let existing = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmp:Rating="1"/></rdf:RDF></x:xmpmeta>"#;
        let text = merge(Some(existing), &sample());
        assert!(text.contains("xmp:Rating=\"4\""));
        assert!(text.contains("<rdf:li>beach</rdf:li>"));
        assert!(text.ends_with("</rdf:RDF></x:xmpmeta>"), "{text}");
        assert_eq!(parse(&text).keywords.len(), 2);
    }

    #[test]
    fn clearing_a_label_and_flag_removes_them() {
        let text = merge(None, &sample());
        let cleared = Sidecar::new(0, Flag::None, None, Vec::new());
        let merged = merge(Some(&text), &cleared);
        assert!(!merged.contains("xmp:Label"));
        assert!(!merged.contains("sortlens:Decision"));
        assert!(!merged.contains("<dc:subject"));
        assert!(merged.contains("xmp:Rating=\"0\""));
    }

    #[test]
    fn the_element_form_of_a_rating_is_understood() {
        let text = r#"<rdf:Description><xmp:Rating>5</xmp:Rating></rdf:Description>"#;
        assert_eq!(parse(text).rating, 5);
    }

    #[test]
    fn entities_are_escaped_and_unescaped() {
        let sidecar = Sidecar::new(3, Flag::None, None, vec!["smith & jones <studio>".into()]);
        let text = merge(None, &sidecar);
        assert!(text.contains("smith &amp; jones &lt;studio&gt;"));
        assert_eq!(parse(&text).keywords, vec!["smith & jones <studio>".to_string()]);
    }

    #[test]
    fn writing_is_atomic_and_idempotent() {
        let dir = temp_dir("write");
        let image = dir.join("IMG_0001.CR3");
        fs::write(&image, b"raw").expect("image");
        assert_eq!(path_for(&image), dir.join("IMG_0001.xmp"));

        assert!(write(&image, &sample()).expect("write"));
        let first = fs::read_to_string(path_for(&image)).expect("read");
        assert!(!write(&image, &sample()).expect("rewrite"), "unchanged content is not rewritten");
        assert_eq!(fs::read_to_string(path_for(&image)).expect("read"), first);
        assert!(!dir.join("IMG_0001.xmp.tmp").exists(), "no temp file is left behind");

        let read_back = read(&image).expect("read").expect("present");
        assert_eq!(read_back, sample());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_sidecar_reads_as_none() {
        let dir = temp_dir("missing");
        let image = dir.join("a.jpg");
        fs::write(&image, b"jpg").expect("image");
        assert!(read(&image).expect("read").is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sidecar_paths_replace_the_extension() {
        assert_eq!(path_for(Path::new("/photos/IMG_1.CR3")), PathBuf::from("/photos/IMG_1.xmp"));
        assert_eq!(path_for(Path::new("/photos/IMG_1.jpg")), PathBuf::from("/photos/IMG_1.xmp"));
    }

    #[test]
    fn edits_are_recognised_by_their_namespaces() {
        let plain = merge(None, &sample());
        assert!(!has_edits(&plain), "our own sidecar is not an edit");

        let edited = r#"<rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" crs:Exposure2012="+0.5"/>"#;
        assert!(has_edits(edited));
        assert!(has_edits("<xmpMM:History><rdf:Seq/></xmpMM:History>"));
        assert!(has_edits("<darktable:history_end>4</darktable:history_end>"));
    }

    #[test]
    fn keywords_are_sorted_and_deduplicated() {
        let sidecar = Sidecar::new(1, Flag::None, None, vec!["b".into(), "a".into(), "a".into()]);
        assert_eq!(sidecar.keywords, vec!["a".to_string(), "b".to_string()]);
    }
}
