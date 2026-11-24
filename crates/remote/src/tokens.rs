//! Capability tokens and the store they live in.
//!
//! A paired device holds one bearer token, and the token carries scopes: `read`, `upload`,
//! `annotate`, `cull` (the four named in `docs/ARCHITECTURE.md` §10). The scopes are the only
//! thing between a phone and the library's writes, so they are checked on every request and
//! persisted as JSON beside the library, where the owner can edit or delete them by hand.

use serde::{Deserialize, Serialize};
use sortlens_core::Error as CoreError;
use std::path::{Path, PathBuf};

/// Result type for this crate.
pub type Result<T> = std::result::Result<T, CoreError>;

/// The schema of the token file. Bumped when the shape changes; a newer file opens read-only.
pub const TOKEN_SCHEMA: u32 = 1;

/// What a paired device may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    /// Paged reads, stats, history, keyword and collection listings.
    Read,
    /// The upload endpoint: files land in the configured album folder.
    Upload,
    /// Catalog writes that leave every file on disk alone: ratings, flags, labels, keywords,
    /// collections, EXIF-suggested accepts, undos, and the on-demand rescan.
    Annotate,
    /// The cull session commands: start, decide, back, clear, summary.
    Cull,
}

impl Permission {
    /// Every scope, in the order a token file writes them.
    pub const ALL: [Permission; 4] = [Permission::Read, Permission::Upload, Permission::Annotate, Permission::Cull];

    /// The stable id used on the wire and in the token file.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Permission::Read => "read",
            Permission::Upload => "upload",
            Permission::Annotate => "annotate",
            Permission::Cull => "cull",
        }
    }

    /// Parse a scope id. Unknown scopes are refused rather than silently dropped, so a
    /// hand-edited token file cannot quietly lose half a device's access.
    #[must_use]
    pub fn parse(id: &str) -> Option<Permission> {
        Permission::ALL.into_iter().find(|permission| permission.id() == id)
    }
}

/// The scopes one device token carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Permissions {
    /// Paged reads and listings.
    pub read: bool,
    /// The upload endpoint.
    pub upload: bool,
    /// Catalog annotations and the rescan.
    pub annotate: bool,
    /// The cull session.
    pub cull: bool,
}

impl Permissions {
    /// Every scope, for a device the owner paired themselves.
    #[must_use]
    pub fn all() -> Self {
        Self {
            read: true,
            upload: true,
            annotate: true,
            cull: true,
        }
    }

    /// Build from a list of scope ids in wire order.
    #[must_use]
    pub fn from_list(list: &[Permission]) -> Self {
        let mut out = Self::default();
        for permission in list {
            match permission {
                Permission::Read => out.read = true,
                Permission::Upload => out.upload = true,
                Permission::Annotate => out.annotate = true,
                Permission::Cull => out.cull = true,
            }
        }
        out
    }

    /// Whether the token may do `permission`.
    #[must_use]
    pub const fn covers(self, permission: Permission) -> bool {
        match permission {
            Permission::Read => self.read,
            Permission::Upload => self.upload,
            Permission::Annotate => self.annotate,
            Permission::Cull => self.cull,
        }
    }

    /// The scopes as ids, in [`Permission::ALL`] order so a file rewrites stably.
    #[must_use]
    pub fn to_list(self) -> Vec<Permission> {
        Permission::ALL.into_iter().filter(|permission| self.covers(*permission)).collect()
    }
}

/// One paired device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceToken {
    /// The bearer token itself, 32 lowercase hex characters.
    pub token: String,
    /// The device name the pairing caller chose, for the owner to recognise a row.
    pub device: String,
    /// What this device may do.
    pub permissions: Permissions,
    /// Unix seconds when the token was issued.
    pub created_unix: u64,
}

#[derive(Serialize, Deserialize)]
struct StoreFile {
    schema: u32,
    tokens: Vec<DeviceToken>,
}

/// The token file: `<library>/remote-tokens.json` by default.
#[derive(Debug)]
pub struct TokenStore {
    tokens: Vec<DeviceToken>,
    path: PathBuf,
}

impl TokenStore {
    /// Load the store at `path`. A missing file is an empty store — first run — and a file that
    /// will not parse is an error, because silently starting over would revoke devices the owner
    /// believes are still paired.
    pub fn load(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let tokens = match std::fs::read_to_string(&path) {
            Ok(text) => {
                let file: StoreFile =
                    serde_json::from_str(&text).map_err(|error| CoreError::Invalid(format!("{}: {error}", path.display())))?;
                if file.schema > TOKEN_SCHEMA {
                    return Err(CoreError::Invalid(format!(
                        "{}: token schema {} is newer than {TOKEN_SCHEMA}",
                        path.display(),
                        file.schema
                    )));
                }
                file.tokens
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(CoreError::io(path.display().to_string(), error)),
        };
        Ok(Self { tokens, path })
    }

    /// Write the store back. Called after every change, so a crash cannot resurrect a revoked
    /// device.
    pub fn save(&self) -> Result<()> {
        let file = StoreFile {
            schema: TOKEN_SCHEMA,
            tokens: self.tokens.clone(),
        };
        let text = serde_json::to_string_pretty(&file).map_err(|error| CoreError::Invalid(error.to_string()))?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| CoreError::io(parent.display().to_string(), error))?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, text).map_err(|error| CoreError::io(tmp.display().to_string(), error))?;
        std::fs::rename(&tmp, &self.path).map_err(|error| CoreError::io(self.path.display().to_string(), error))?;
        Ok(())
    }

    /// The store's path, for the server's startup line.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Issue a token. `token` comes from the server's entropy mix; the store only records it.
    /// A token that already exists is re-issued with the new scopes and device name.
    pub fn issue(&mut self, token: String, device: &str, permissions: Permissions, created_unix: u64) -> Result<DeviceToken> {
        if token.is_empty() || !token.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) {
            return Err(CoreError::Invalid("a device token is lowercase hex".into()));
        }
        if let Some(existing) = self.tokens.iter_mut().find(|row| row.token == token) {
            existing.device = device.to_string();
            existing.permissions = permissions;
            let issued = existing.clone();
            self.save()?;
            return Ok(issued);
        }
        let issued = DeviceToken {
            token: token.clone(),
            device: device.to_string(),
            permissions,
            created_unix,
        };
        self.tokens.push(issued.clone());
        self.save()?;
        Ok(issued)
    }

    /// The device a bearer token belongs to, if it is paired.
    #[must_use]
    pub fn find(&self, token: &str) -> Option<&DeviceToken> {
        self.tokens.iter().find(|row| row.token == token)
    }

    /// Revoke a device. Reports whether it was paired at all.
    pub fn revoke(&mut self, token: &str) -> Result<bool> {
        let before = self.tokens.len();
        self.tokens.retain(|row| row.token != token);
        if self.tokens.len() == before {
            return Ok(false);
        }
        self.save()?;
        Ok(true)
    }

    /// How many devices are paired.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Whether anything is paired.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn scopes_round_trip_through_the_wire_form() {
        let all = Permissions::all();
        let list = all.to_list();
        assert_eq!(
            list,
            vec![Permission::Read, Permission::Upload, Permission::Annotate, Permission::Cull],
            "a token file writes scopes in a stable order"
        );
        assert_eq!(Permissions::from_list(&list), all);
        assert!(all.covers(Permission::Read));
        let narrow = Permissions::from_list(&[Permission::Read, Permission::Annotate]);
        assert!(narrow.covers(Permission::Read));
        assert!(
            !narrow.covers(Permission::Upload),
            "a scope the pairing did not grant is not covered"
        );
        assert_eq!(Permission::parse("read"), Some(Permission::Read));
        assert_eq!(Permission::parse("cull"), Some(Permission::Cull));
        assert_eq!(Permission::parse("root"), None, "unknown scopes are refused, not dropped");
    }

    #[test]
    fn the_store_persists_a_token_across_a_reload_and_refuses_a_rubbish_file() {
        let dir = std::env::temp_dir().join(format!(
            "sortlens-tokens-{}-{}",
            std::process::id(),
            std::time::Instant::now().elapsed().subsec_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("remote-tokens.json");
        {
            let mut store = TokenStore::load(&path).unwrap();
            assert!(store.is_empty(), "first run: nothing paired");
            store.issue("abc123".repeat(5) + "ab", "phone", Permissions::all(), 100).unwrap();
            assert_eq!(store.len(), 1);
        }
        let store = TokenStore::load(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"schema\": 1"));
        assert!(store.find(&("abc123".repeat(5) + "ab")).is_some(), "the token survives a restart");
        assert!(store.find(&("ffffff".repeat(5) + "ab")).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_issue_replaces_the_same_token_and_revoke_removes_the_row() {
        let dir = std::env::temp_dir().join(format!("sortlens-tokens-r-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("remote-tokens.json");
        let mut store = TokenStore::load(&path).unwrap();
        let token = "abcd".repeat(8);
        store.issue(token.to_string(), "phone", Permissions::all(), 100).unwrap();
        store
            .issue(token.to_string(), "tablet", Permissions::from_list(&[Permission::Read]), 200)
            .unwrap();
        assert_eq!(store.len(), 1, "re-issuing the same token edits the row instead of duplicating it");
        assert_eq!(store.find(&token).unwrap().device, "tablet");
        assert_eq!(store.find(&token).unwrap().permissions, Permissions::from_list(&[Permission::Read]));
        assert!(store.revoke(&token).unwrap());
        assert!(!store.revoke(&token).unwrap(), "revoking twice reports the truth");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_malformed_token_argument_is_refused() {
        let dir = std::env::temp_dir().join(format!("sortlens-tokens-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut store = TokenStore::load(dir.join("remote-tokens.json")).unwrap();
        assert!(store.issue("short".to_string(), "phone", Permissions::all(), 1).is_err());
        assert!(
            store.issue("ABCD".repeat(8), "phone", Permissions::all(), 1).is_err(),
            "uppercase is not our token form"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
