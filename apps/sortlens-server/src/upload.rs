//! Resumable uploads: a phone pushes a photograph into the album folder in pieces it can
//! repeat until the host confirms the whole file (SL-J04).
//!
//! The protocol is range-shaped, so a client can always discover where to continue:
//!
//! - `POST /upload` with `X-Filename`, `X-Upload-Id`, and either no `Content-Range`
//!   (one request, whole file) or `Content-Range: bytes <start>-<end>/<total>`.
//! - The partial bytes live at `<album>/.uploads/<id>.part`, so the file length *is* the
//!   received count, and a server restart cannot lose it.
//! - A chunk that does not start where the file ends answers `409` with `received`, and the
//!   client rewinds to there — resume without a session table.
//! - The final chunk verifies the declared size and, when the client sent
//!   `X-Content-Sha256`, the digest; only then does the partial file become
//!   `<album>/<filename>`. A failed digest leaves the partial in place, so the client can
//!   send the same bytes again rather than the whole photograph.

use serde_json::{Value, json};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// The staging folder inside the album.
const STAGING: &str = ".uploads";

/// One chunk processed.
#[derive(Debug)]
pub struct Outcome {
    /// The HTTP status the answer carries.
    pub status: u16,
    /// The JSON body.
    pub body: Value,
}

fn ok(status: u16, body: Value) -> Outcome {
    Outcome { status, body }
}

/// Whether an upload id is one the server will touch. The id names files on disk, so it is
/// restricted to a safe alphabet rather than sanitised on the way through.
#[must_use]
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The file name a photograph will land under: the leaf of whatever the client sent, with
/// the characters a Windows folder cannot hold removed and an empty name made honest.
#[must_use]
pub fn sanitise_name(raw: &str) -> String {
    let leaf = raw.rsplit(['\\', '/']).next().unwrap_or("");
    let cleaned: String = leaf
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
        .collect();
    let trimmed = cleaned.trim_end_matches(['.', ' ']);
    if trimmed.is_empty() {
        "upload.bin".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Where a chunk claims to sit: `bytes <start>-<end>/<total>`, or none for a one-shot upload.
pub fn parse_range(raw: Option<&str>) -> Result<Option<(u64, u64, u64)>, &'static str> {
    let Some(raw) = raw else { return Ok(None) };
    let Some(rest) = raw.strip_prefix("bytes ") else {
        return Err("Content-Range must be 'bytes <start>-<end>/<total>'");
    };
    let mut parts = rest.split('/');
    let span = parts.next().ok_or("Content-Range has no span")?;
    let total = parts.next().ok_or("Content-Range has no total")?;
    if parts.next().is_some() {
        return Err("Content-Range has too many parts");
    }
    let (start, end) = span
        .split_once('-')
        .ok_or("Content-Range span must be '<start>-<end>'")
        .and_then(|(a, b)| {
            let start = a.parse::<u64>().map_err(|_| "Content-Range span is not two numbers")?;
            let end = b.parse::<u64>().map_err(|_| "Content-Range span is not two numbers")?;
            Ok((start, end))
        })?;
    let total = total.parse::<u64>().map_err(|_| "Content-Range total is not a number")?;
    if total == 0 || start > end || end >= total {
        return Err("Content-Range span does not fit its total");
    }
    Ok(Some((start, end, total)))
}

struct Partial {
    part: PathBuf,
    meta: PathBuf,
    name: String,
    total: u64,
}

fn partial_for(album: &Path, id: &str) -> Partial {
    let staging = album.join(STAGING);
    Partial {
        part: staging.join(format!("{id}.part")),
        meta: staging.join(format!("{id}.meta.json")),
        name: String::new(),
        total: 0,
    }
}

/// One chunk of an upload, as the request headers carried it.
pub struct Chunk<'a> {
    /// The client-chosen id naming this transfer.
    pub id: &'a str,
    /// The file name the photograph will land under, before sanitising.
    pub raw_name: &'a str,
    /// The parsed `Content-Range`, or `None` for a one-shot upload.
    pub range: Result<Option<(u64, u64, u64)>, &'a str>,
    /// The client's `X-Content-Sha256`, verified when the file completes.
    pub expected_sha: Option<&'a str>,
    /// The chunk bytes.
    pub body: &'a [u8],
    /// The upload body cap.
    pub max_upload: u64,
}

/// Record the chunk.
pub fn ingest(album: &Path, chunk: &Chunk) -> Outcome {
    let Chunk {
        id,
        raw_name,
        range,
        expected_sha,
        body,
        max_upload,
    } = *chunk;
    if !valid_id(id) {
        return ok(
            400,
            json!({ "ok": false, "error": "X-Upload-Id is 1-64 characters of A-Z a-z 0-9 - _" }),
        );
    }
    if let Err(reason) = range {
        return ok(400, json!({ "ok": false, "error": reason }));
    }
    let range = range.ok().unwrap_or(None);
    let staging = album.join(STAGING);
    if let Err(error) = std::fs::create_dir_all(&staging) {
        return ok(
            500,
            json!({ "ok": false, "error": format!("could not create the upload staging folder: {error}") }),
        );
    }
    if body.is_empty() {
        return ok(400, json!({ "ok": false, "error": "the chunk is empty" }));
    }

    let mut partial = partial_for(album, id);
    let received_now = std::fs::metadata(&partial.part).map(|meta| meta.len()).unwrap_or(0);
    let (start, end, total) = match range {
        Some((start, end, total)) => (start, end, total),
        None => (0, body.len() as u64 - 1, body.len() as u64),
    };

    // The declared total is part of the file's identity: a client that resumes with a
    // different total is describing a different photograph.
    if !partial.meta.exists() && range.is_none() {
        // One-shot upload, first contact: fine.
    } else if let Ok(text) = std::fs::read_to_string(&partial.meta)
        && let Ok(stored) = serde_json::from_str::<Value>(&text)
    {
        let stored_total = stored.get("total").and_then(Value::as_u64).unwrap_or(0);
        if stored_total != total {
            return ok(
                409,
                json!({ "ok": false, "error": "the total does not match the upload in progress", "received": received_now, "total": stored_total }),
            );
        }
        partial.name = stored.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        partial.total = stored_total;
    }
    if start != received_now {
        return ok(
            409,
            json!({ "ok": false, "error": "the chunk does not start where the file ends", "received": received_now }),
        );
    }
    if total > max_upload {
        return ok(
            413,
            json!({ "ok": false, "error": format!("the upload is past the {max_upload}-byte limit") }),
        );
    }
    if end + 1 - start != body.len() as u64 {
        return ok(
            400,
            json!({ "ok": false, "error": "the body length does not match the Content-Range span" }),
        );
    }

    // A chunk that completes the file is verified *before* it is appended: a digest
    // mismatch must leave the partial exactly where it was, so the client can send the
    // chunk again instead of starting over. Hash what is already on disk, feed the chunk
    // through, and compare.
    let completing = start + body.len() as u64 == total;
    let mut digest_hex = String::new();
    if completing && expected_sha.is_some() {
        let mut hasher = sortlens_ai::sha256::Hasher::new();
        let existing = match std::fs::File::open(&partial.part) {
            Ok(file) => file,
            Err(error) => {
                return ok(
                    500,
                    json!({ "ok": false, "error": format!("could not read the partial file: {error}") }),
                );
            }
        };
        let mut reader = std::io::BufReader::new(existing);
        let mut buffer = [0u8; 64 * 1024];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => hasher.update(&buffer[..read]),
                Err(error) => {
                    return ok(
                        500,
                        json!({ "ok": false, "error": format!("could not read the partial file: {error}") }),
                    );
                }
            }
        }
        hasher.update(body);
        digest_hex = sortlens_ai::sha256::to_hex(&hasher.finish());
        let expected = expected_sha.unwrap_or("");
        if !digest_hex.eq_ignore_ascii_case(expected) {
            return ok(
                422,
                json!({ "ok": false, "error": "the digest does not match the bytes that arrived", "sha256": digest_hex, "received": received_now }),
            );
        }
    }

    let mut file = match open_append(&partial.part) {
        Ok(file) => file,
        Err(error) => {
            return ok(
                500,
                json!({ "ok": false, "error": format!("could not open the partial file: {error}") }),
            );
        }
    };
    if let Err(error) = file.write_all(body) {
        return ok(500, json!({ "ok": false, "error": format!("could not write the chunk: {error}") }));
    }
    drop(file);
    if partial.name.is_empty() && partial.total == 0 {
        let meta = json!({ "name": sanitise_name(raw_name), "total": total });
        if let Err(error) = std::fs::write(&partial.meta, meta.to_string()) {
            return ok(
                500,
                json!({ "ok": false, "error": format!("could not record the upload metadata: {error}") }),
            );
        }
    }

    let received = std::fs::metadata(&partial.part).map(|meta| meta.len()).unwrap_or(0);
    if received < total {
        return ok(200, json!({ "ok": true, "received": received, "total": total }));
    }

    // Complete: publish. The digest was verified before the final chunk landed, when the
    // client declared one; without a declared digest the computed one is recorded.
    let bytes = match std::fs::read(&partial.part) {
        Ok(bytes) => bytes,
        Err(error) => {
            return ok(
                500,
                json!({ "ok": false, "error": format!("could not read the completed upload: {error}") }),
            );
        }
    };
    if bytes.len() as u64 != total {
        return ok(
            500,
            json!({ "ok": false, "error": format!("the partial file is {} bytes, not the promised {total}", bytes.len()) }),
        );
    }
    let digest = if digest_hex.is_empty() {
        sortlens_ai::sha256::to_hex(&sortlens_ai::sha256::digest(&bytes))
    } else {
        digest_hex
    };
    let name = if partial.name.is_empty() {
        sanitise_name(raw_name)
    } else {
        partial.name.clone()
    };
    let mut target = album.join(&name);
    let mut final_name = name.clone();
    // Collision policy: never overwrite. The second photograph with a camera's repeated
    // name gets ` (1)` beside the stem, the way the desktop already names exports.
    let mut counter = 0u32;
    while target.exists() {
        counter += 1;
        let stem = name.rsplit_once('.').filter(|(stem, _)| !stem.is_empty());
        final_name = match stem {
            Some((stem, ext)) => format!("{stem} ({counter}).{ext}"),
            None => format!("{name} ({counter})"),
        };
        target = album.join(&final_name);
    }
    if let Err(error) = std::fs::rename(&partial.part, &target) {
        return ok(
            500,
            json!({ "ok": false, "error": format!("could not move the upload into the album: {error}") }),
        );
    }
    let _ = std::fs::remove_file(&partial.meta);
    ok(
        200,
        json!({ "ok": true, "received": received, "saved": final_name, "sha256": digest }),
    )
}

/// How much of `id`'s file the host already holds — the question a resuming client asks.
pub fn status(album: &Path, id: &str) -> Outcome {
    if !valid_id(id) {
        return ok(
            400,
            json!({ "ok": false, "error": "an upload id is 1-64 characters of A-Z a-z 0-9 - _" }),
        );
    }
    let staging = album.join(STAGING);
    let part = staging.join(format!("{id}.part"));
    let meta_path = staging.join(format!("{id}.meta.json"));
    let received = std::fs::metadata(&part).map(|meta| meta.len()).unwrap_or(0);
    if received == 0 {
        return ok(404, json!({ "ok": false, "error": "no upload with that id" }));
    }
    let meta: Value = std::fs::read_to_string(&meta_path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null);
    let mut body = json!({ "ok": true, "received": received });
    if let Some(total) = meta.get("total").and_then(Value::as_u64) {
        body["total"] = json!(total);
    }
    if let Some(name) = meta.get("name").and_then(Value::as_str) {
        body["name"] = json!(name);
    }
    ok(200, body)
}

fn open_append(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new().create(true).append(true).open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_leaf_only_and_folder_safe() {
        assert_eq!(sanitise_name(r"C:\Users\me\IMG_0001.jpg"), "IMG_0001.jpg");
        assert_eq!(sanitise_name("../../etc/passwd"), "passwd", "the leaf of a path is all that lands");
        assert_eq!(sanitise_name("a<b>c|d?e"), "abcde");
        assert_eq!(sanitise_name("name."), "name", "a Windows folder refuses a trailing dot");
        assert_eq!(sanitise_name(""), "upload.bin");
    }

    #[test]
    fn ranges_parse_and_refuse_the_impossible() {
        assert_eq!(parse_range(Some("bytes 0-9/100")), Ok(Some((0, 9, 100))));
        assert_eq!(parse_range(Some("bytes 40-99/100")), Ok(Some((40, 99, 100))));
        assert_eq!(parse_range(None), Ok(None));
        assert!(parse_range(Some("chunks 0-9/100")).is_err());
        assert!(parse_range(Some("bytes 99-40/100")).is_err());
        assert!(parse_range(Some("bytes 0-100/100")).is_err());
        assert!(parse_range(Some("bytes 0-9/0")).is_err());
    }

    #[test]
    fn ids_are_a_safe_alphabet() {
        assert!(valid_id("phone-IMG_1"));
        assert!(!valid_id("../escape"));
        assert!(!valid_id(""));
        assert!(!valid_id(&"x".repeat(65)));
    }
}
