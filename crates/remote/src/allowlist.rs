//! The command-registry allow-list: which registry commands a remote device may call, and
//! which scope each one needs.
//!
//! `docs/ARCHITECTURE.md` §10 is the rule: *the remote API is the command registry with a
//! restricted allow-list, not a second API*. The table below names every registry id the wire
//! accepts; anything absent is refused without asking the engine. A test here fails the
//! moment the registry renames or retires one of these ids, and fails the moment an id in the
//! table is not `Status::Ready`.

use crate::tokens::Permission;
use sortlens_core::Result;
use sortlens_engine::Registry;

/// Every registry command the wire accepts, with the scope it needs.
///
/// A phone session is the same `photo.*` and `cull.*` commands the window runs, so the audit
/// trail stays one trail. Commands that reach outside the engine (open a file, quit the app,
/// install a model, check for updates, write to disk outside the album) have no row: a paired
/// device cannot ask the host to do them.
pub const TABLE: &[(&str, Permission)] = &[
    // Read: what a browse screen needs.
    ("library.page", Permission::Read),
    ("library.stats", Permission::Read),
    ("library.roots", Permission::Read),
    ("library.history", Permission::Read),
    ("library.jobs", Permission::Read),
    ("library.health", Permission::Read),
    ("library.changed", Permission::Read),
    ("library.deleted", Permission::Read),
    ("tag.keywords", Permission::Read),
    ("tag.registry", Permission::Read),
    ("collection.search", Permission::Read),
    ("memories.list", Permission::Read),
    ("library.audit", Permission::Read),
    // The snapshot is the state itself, so a client that cannot hold the op log can hold the
    // library: the wasm client opens it through `Engine::open_memory` (SL-J02).
    ("library.snapshot", Permission::Read),
    // Annotate: catalog writes that leave every file on disk alone.
    ("photo.rate", Permission::Annotate),
    ("photo.flag", Permission::Annotate),
    ("photo.label", Permission::Annotate),
    ("photo.keyword", Permission::Annotate),
    ("photo.setCaptured", Permission::Annotate),
    ("photo.tagFromPrev", Permission::Annotate),
    ("tag.alias", Permission::Annotate),
    ("tag.rename", Permission::Annotate),
    ("tag.delete", Permission::Annotate),
    ("collection.new", Permission::Annotate),
    ("collection.addTo", Permission::Annotate),
    ("collection.removeFrom", Permission::Annotate),
    ("collection.rename", Permission::Annotate),
    ("collection.delete", Permission::Annotate),
    ("collection.favourite", Permission::Annotate),
    ("collection.sort", Permission::Annotate),
    ("collection.colour", Permission::Annotate),
    ("suggest.accept", Permission::Annotate),
    ("suggest.dismiss", Permission::Annotate),
    ("library.retryMetadata", Permission::Annotate),
    ("edit.undo", Permission::Annotate),
    ("edit.redo", Permission::Annotate),
    // On-demand rescan (SL-J08): the host re-indexes because the phone asked. It writes the
    // catalog, so it is an annotate, not a read.
    ("library.rescan", Permission::Annotate),
    // Cull: the session a phone is actually for.
    ("cull.start", Permission::Cull),
    ("cull.decide", Permission::Cull),
    ("cull.apply", Permission::Cull),
    ("cull.back", Permission::Cull),
    ("cull.clear", Permission::Cull),
    ("cull.summary", Permission::Cull),
];

/// The scope `id` needs, or `None` when the wire does not accept it at all.
#[must_use]
pub fn permission_for(id: &str) -> Option<Permission> {
    TABLE
        .iter()
        .find(|(table_id, _)| *table_id == id)
        .map(|(_, permission)| *permission)
}

/// Whether the wire accepts `id` at all (a scope check still applies).
#[must_use]
pub fn accepts(id: &str) -> bool {
    permission_for(id).is_some()
}

/// Verify the table against the registry it names.
///
/// Called once at server start and from the tests: an allow-list that points at a command the
/// registry does not have is an invitation that answers "planned" forever, and a ready
/// command a phone should be able to call but cannot is a silent gap. The registry is the
/// source of truth; this check keeps the table in step with it.
pub fn audit() -> Result<()> {
    let registry = Registry::v0();
    for (id, _) in TABLE {
        let spec = registry
            .get(id)
            .ok_or_else(|| sortlens_core::Error::Invalid(format!("the remote allow-list names {id}, which the registry does not have")))?;
        if !spec.is_ready() {
            return Err(sortlens_core::Error::Invalid(format!(
                "the remote allow-list names {id}, which is not ready yet"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use sortlens_engine::Status;

    #[test]
    fn every_table_row_is_a_ready_registry_command() {
        audit().expect("the allow-list names only ready registry commands");
    }

    #[test]
    fn the_registry_has_no_ready_command_the_wire_silently_swallowed() {
        // The rule cuts both ways: a ready command the table forgot is a gap a maintainer
        // should see. Host-effect commands (open, reveal, quit, settings, views, exports that
        // touch the desktop, model and update network calls, diagnostics, backups, library
        // surgery) stay out on purpose; everything else that is ready must be named here or
        // named in this test's exclusion list with a reason.
        let registry = Registry::v0();
        let host_effect = [
            // The host does these; the engine only names the effect.
            "app.about",
            "app.quit",
            "app.settings",
            "app.shortcuts",
            "app.supportReport",
            "app.diagnostics",
            "photo.open",
            "photo.reveal",
            // Views are a window concern; the phone has its own screens.
            "view.detail",
            "view.compare",
            "view.places",
            "view.softFocus",
            "view.similar",
            "view.insights",
            // Desktop I/O and library surgery stay native-only.
            "library.addFolder",
            "library.removeRoot",
            "library.compact",
            "library.backup",
            "library.restoreBackup",
            "library.exportHistory",
            "library.clearDeleted",
            "library.forgetBatch",
            "library.restoreBatch",
            "library.relocate",
            "library.remapRoot",
            "library.removeMissing",
            "library.readOnly",
            "library.subfolders",
            "library.refreshChanged",
            "edit.moves",
            "edit.movePicks",
            "edit.export",
            "edit.cancelExport",
            "edit.undoMove",
            "folder.mark",
            "folder.marks",
            "photo.delete",
            // Jobs whose progress is a native screen.
            "insights.start",
            "insights.progress",
            "photo.sharpness",
            "photo.sharpnessProgress",
            "similar.start",
            "similar.groups",
            "suggest.start",
            "suggest.list",
            "suggest.results",
            "suggest.progress",
            "suggest.cancel",
            "suggest.clear",
            // Approved network calls, consent-gated at the desktop; a paired device gets neither.
            "packs.list",
            "packs.set",
            "model.consent",
            "model.install",
            "model.cancel",
            "model.remove",
            "model.status",
            "update.check",
            "update.consent",
            "update.status",
        ];
        for spec in registry.ready() {
            if accepts(spec.id) {
                continue;
            }
            assert!(
                host_effect.contains(&spec.id),
                "ready command {} is accepted neither by the allow-list nor by this exclusion list",
                spec.id
            );
        }
    }

    #[test]
    fn scopes_map_as_documented() {
        assert_eq!(permission_for("library.page"), Some(Permission::Read));
        assert_eq!(permission_for("photo.rate"), Some(Permission::Annotate));
        assert_eq!(permission_for("library.rescan"), Some(Permission::Annotate));
        assert_eq!(permission_for("cull.decide"), Some(Permission::Cull));
        assert_eq!(permission_for("app.quit"), None, "the wire never accepts a host command");
        assert_eq!(permission_for("photo.rate.nonsense"), None);
        assert!(TABLE.iter().all(|(id, _)| registry_status_ready(id)));
    }

    fn registry_status_ready(id: &str) -> bool {
        Registry::v0().get(id).is_some_and(|spec| spec.status == Status::Ready)
    }
}
