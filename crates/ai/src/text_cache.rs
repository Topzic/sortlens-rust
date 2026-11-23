//! Cached per-pack text embeddings — the expensive half of the pack match, computed once.
//!
//! Matching a photo against a pack's vocabulary by meaning costs one image embedding per photo and
//! one text embedding per keyword. The image side must run per photo; the text side never has to.
//! A pack is a file that rarely changes, and the text embedder is a pinned model that never
//! changes without its digest changing, so each pack's keyword embeddings are computed once and
//! kept in the model store, keyed by both. A changed pack file or a different installed embedder
//! is a different cache file by construction — nothing is ever invalidated, only superseded.
//!
//! The cache is a pure convenience layer: given the embedder it fills missing entries, and given
//! no embedder it can only serve what is already cached. It trusts its own files only after
//! re-checking the model digest and the pack's content hash recorded inside them.

use crate::sha256;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// The cache schema this build writes and reads.
pub const SCHEMA: u32 = 1;

/// One keyword's embedding, as the cache keeps it.
#[derive(Debug, Clone, PartialEq)]
pub struct KeywordEmbedding {
    /// The keyword, as the pack spells it (trimmed, lowercased before comparison by the caller).
    pub keyword: String,
    /// The embedding, straight from the text embedder.
    pub values: Vec<f32>,
}

/// What a cache file contains, exactly as serialised.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CacheFile {
    schema: u32,
    /// The text embedder's SHA-256, hex: embeddings from a different model are not this model's.
    model: String,
    /// The SHA-256 of the pack file's bytes: a changed pack is a different vocabulary.
    pack: String,
    keywords: Vec<CachedKeyword>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CachedKeyword {
    keyword: String,
    values: Vec<f32>,
}

/// Where a pack's cache file lives: `<cache>/<model>/<pack content hash>.json`.
fn cache_path(cache_root: &Path, model: &str, pack_hash: &str) -> PathBuf {
    cache_root.join(model).join(format!("{pack_hash}.json"))
}

/// Load a pack's keyword embeddings from the cache, or build them with `embed` and cache them.
///
/// `pack_bytes` are the pack file's bytes (their digest keys the file); `keywords` are the
/// keywords in pack order. `embed` is called only for keywords the cache does not know, and must
/// embed exactly the prompt [`crate::tokenize::pack_prompt`] shapes. Nothing is written when the
/// pack was fully cached, and a partial cache is completed before it is handed back.
pub fn load_or_build(
    cache_root: &Path,
    model: &str,
    pack_bytes: &[u8],
    keywords: &[String],
    embed: impl FnMut(&str) -> Result<Vec<f32>, String>,
) -> Result<Vec<KeywordEmbedding>, String> {
    let pack_hash = sha256::to_hex(&sha256::digest(pack_bytes));
    let path = cache_path(cache_root, model, &pack_hash);
    let known: Vec<(String, Vec<f32>)> = read_cache(&path)
        .filter(|cache| cache.model == model && cache.pack == pack_hash)
        .map(|cache| {
            cache
                .keywords
                .into_iter()
                .map(|keyword| (keyword.keyword, keyword.values))
                .collect()
        })
        .unwrap_or_default();
    let mut answer: Vec<KeywordEmbedding> = keywords
        .iter()
        .filter_map(|keyword| {
            known
                .iter()
                .find(|(known_keyword, _)| known_keyword == keyword)
                .map(|(_, values)| KeywordEmbedding {
                    keyword: keyword.clone(),
                    values: values.clone(),
                })
        })
        .collect();
    let mut embed = embed;
    for keyword in keywords {
        if answer.iter().any(|known| &known.keyword == keyword) {
            continue;
        }
        let values = embed(keyword)?;
        answer.push(KeywordEmbedding {
            keyword: keyword.clone(),
            values,
        });
    }
    // Write the complete cache back, even when the file already existed: a pack that gained a
    // keyword deserves a cache that knows it without another miss next session.
    let file = CacheFile {
        schema: SCHEMA,
        model: model.to_string(),
        pack: pack_hash,
        keywords: answer
            .iter()
            .map(|entry| CachedKeyword {
                keyword: entry.keyword.clone(),
                values: entry.values.clone(),
            })
            .collect(),
    };
    write_cache(&path, &file)?;
    Ok(answer)
}

/// Read a cache file, refusing rather than trusting anything malformed: this build's schema, and
/// keywords whose embeddings are not all the same non-zero length.
fn read_cache(path: &Path) -> Option<CacheFile> {
    let text = fs::read_to_string(path).ok()?;
    let cache: CacheFile = serde_json::from_str(&text).ok()?;
    if cache.schema != SCHEMA {
        return None;
    }
    let first = cache.keywords.first()?;
    let dimension = first.values.len();
    if dimension == 0
        || cache
            .keywords
            .iter()
            .any(|keyword| keyword.values.len() != dimension || keyword.values.iter().any(|value| !value.is_finite()))
    {
        return None;
    }
    Some(cache)
}

/// Write a cache file atomically: temporary file first, rename into place, so a crash mid-write
/// never leaves a half-written cache where a reader would find one.
fn write_cache(path: &Path, cache: &CacheFile) -> Result<(), String> {
    let folder = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(folder).map_err(|error| format!("could not create {}: {error}", folder.display()))?;
    let text = serde_json::to_string(cache).map_err(|error| format!("the cache could not be serialised: {error}"))?;
    let temporary = path.with_extension("json.incoming");
    fs::write(&temporary, text).map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|error| format!("could not move the cache into place at {}: {error}", path.display()))
}

/// Serve a fully cached pack without any model: every requested keyword must be in the cache file
/// recorded for this model digest and pack content, or the call answers with an error and the
/// caller goes on without the pack. Nothing is written.
pub fn load_only(cache_root: &Path, model: &str, pack_bytes: &[u8], keywords: &[String]) -> Result<Vec<KeywordEmbedding>, String> {
    let pack_hash = sha256::to_hex(&sha256::digest(pack_bytes));
    let path = cache_path(cache_root, model, &pack_hash);
    let cache = read_cache(&path)
        .filter(|cache| cache.model == model && cache.pack == pack_hash)
        .ok_or_else(|| format!("no complete cache for this pack under this model at {}", path.display()))?;
    keywords
        .iter()
        .map(|keyword| {
            let values = &cache
                .keywords
                .iter()
                .find(|cached| &cached.keyword == keyword)
                .ok_or_else(|| format!("the cache does not know “{keyword}”"))?
                .values;
            Ok(KeywordEmbedding {
                keyword: keyword.clone(),
                values: values.clone(),
            })
        })
        .collect()
}

/// Remove every cache file recorded under one model digest — what removing that embedder from the
/// store should also do, so a re-install re-derives rather than trusting what an older session
/// wrote. Returns the bytes reclaimed.
pub fn clear(cache_root: &Path, model: &str) -> u64 {
    let folder = cache_root.join(model);
    let Ok(entries) = fs::read_dir(&folder) else {
        return 0;
    };
    let mut reclaimed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let extension = path.extension().and_then(|extension| extension.to_str());
        if (extension == Some("json") || extension == Some("incoming"))
            && let Ok(meta) = fs::metadata(&path)
            && fs::remove_file(&path).is_ok()
        {
            reclaimed += meta.len();
        }
    }
    let _ = fs::remove_dir(&folder);
    reclaimed
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sortlens-text-cache-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn pack_bytes(text: &[u8]) -> Vec<u8> {
        text.to_vec()
    }

    /// A fake embedder: the keyword's length, three times, as floats.
    fn embed_by_len() -> impl FnMut(&str) -> Result<Vec<f32>, String> {
        |keyword: &str| Ok(vec![keyword.len() as f32; 3])
    }

    #[test]
    fn a_first_build_embeds_every_keyword_and_writes_the_cache() {
        let root = temp("first");
        let pack = pack_bytes(b"pack");
        let keywords = vec!["dog".to_string(), "cat".to_string()];
        let built = load_or_build(&root, "digest", &pack, &keywords, embed_by_len()).expect("build");
        assert_eq!(built.len(), 2);
        assert_eq!(built[0].keyword, "dog");
        assert_eq!(built[0].values, vec![3.0; 3]);
        // The file exists under the model digest and the pack hash.
        let pack_hash = sha256::to_hex(&sha256::digest(&pack));
        assert!(cache_path(&root, "digest", &pack_hash).is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_cached_pack_is_served_without_embedding_anything() {
        let root = temp("served");
        let pack = pack_bytes(b"pack");
        let keywords = vec!["dog".to_string()];
        let _ = load_or_build(&root, "digest", &pack, &keywords, embed_by_len()).expect("build");
        // A second call must not reach the embedder: one that panics if asked.
        let served = load_or_build(&root, "digest", &pack, &keywords, |_| {
            panic!("the cache must answer, not the embedder")
        })
        .expect("served");
        assert_eq!(served[0].values, vec![3.0; 3]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_changed_pack_is_a_different_cache_not_a_stale_one() {
        let root = temp("changed");
        let old_pack = pack_bytes(b"pack v1");
        let new_pack = pack_bytes(b"pack v2");
        let _ = load_or_build(&root, "digest", &old_pack, &["dog".to_string()], embed_by_len()).expect("build");
        // The new pack content embeds afresh, even under the same model: the embedder is called.
        let mut calls = 0;
        let built = load_or_build(&root, "digest", &new_pack, &["dog".to_string()], |keyword| {
            calls += 1;
            embed_by_len()(keyword)
        })
        .expect("fresh build");
        assert_eq!(calls, 1, "the new pack was not served from the old pack's cache");
        assert_eq!(built[0].values, vec![3.0; 3]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_different_model_is_a_different_cache() {
        let root = temp("model");
        let pack = pack_bytes(b"pack");
        let _ = load_or_build(&root, "digest-a", &pack, &["dog".to_string()], embed_by_len()).expect("build");
        let served = load_or_build(&root, "digest-b", &pack, &["dog".to_string()], embed_by_len()).expect("other model");
        assert_eq!(served[0].values, vec![3.0; 3], "the other model's cache was built fresh");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_cache_that_gained_a_keyword_is_completed_and_written_back() {
        let root = temp("complete");
        let pack = pack_bytes(b"pack");
        let _ = load_or_build(&root, "digest", &pack, &["dog".to_string()], embed_by_len()).expect("build");
        let keywords = vec!["dog".to_string(), "cat".to_string()];
        let built = load_or_build(&root, "digest", &pack, &keywords, embed_by_len()).expect("complete");
        assert_eq!(built.len(), 2, "both keywords answer");
        // And now the cache knows both: the embedder is not needed again.
        let served = load_or_build(&root, "digest", &pack, &keywords, |_| panic!("cache must be complete now")).expect("served");
        assert_eq!(served.len(), 2);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_tampered_cache_is_rebuilt_not_trusted() {
        let root = temp("tampered");
        let pack = pack_bytes(b"pack");
        let _ = load_or_build(&root, "digest", &pack, &["dog".to_string()], embed_by_len()).expect("build");
        // Something rewrote the cache: a wrong model digest and a non-finite value.
        let pack_hash = sha256::to_hex(&sha256::digest(&pack));
        let path = cache_path(&root, "digest", &pack_hash);
        fs::write(
            &path,
            r#"{"schema":1,"model":"other","pack":"other","keywords":[{"keyword":"dog","values":[1.0,NaN,3.0]}]}"#,
        )
        .expect("tamper");
        let rebuilt = load_or_build(&root, "digest", &pack, &["dog".to_string()], embed_by_len()).expect("rebuilt");
        assert_eq!(rebuilt[0].values, vec![3.0; 3], "the wrong cache was not served");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_embedder_that_refuses_is_an_error_not_an_empty_cache() {
        let root = temp("refusing");
        let pack = pack_bytes(b"pack");
        let error = load_or_build(&root, "digest", &pack, &["dog".to_string()], |_| {
            Err("the text embedder is broken".into())
        })
        .expect_err("refused");
        assert!(error.contains("broken"), "{error}");
        let pack_hash = sha256::to_hex(&sha256::digest(&pack));
        assert!(!cache_path(&root, "digest", &pack_hash).exists(), "a failed build writes no cache");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn clearing_one_model_leaves_the_others() {
        let root = temp("clear");
        let pack = pack_bytes(b"pack");
        let _ = load_or_build(&root, "digest-a", &pack, &["dog".to_string()], embed_by_len()).expect("a");
        let _ = load_or_build(&root, "digest-b", &pack, &["dog".to_string()], embed_by_len()).expect("b");
        let reclaimed = clear(&root, "digest-a");
        assert!(reclaimed > 0, "the files were real");
        let pack_hash = sha256::to_hex(&sha256::digest(&pack));
        assert!(!cache_path(&root, "digest-a", &pack_hash).exists());
        assert!(
            cache_path(&root, "digest-b", &pack_hash).exists(),
            "the other model's cache survives"
        );
        // Clearing an unknown model is simply nothing.
        assert_eq!(clear(&root, "never-installed"), 0);
        let _ = fs::remove_dir_all(&root);
    }
}
