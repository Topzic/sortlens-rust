//! The desktop's side of serving the library: spawn `sortlens-server` beside the app, follow its
//! startup lines, read the library's token file, and stop the process again.
//!
//! The shell paints what this module feeds it (`UiState::server`); the process, the binary and
//! the token file are all host business. Revoking is refused while the process runs, because a
//! running server holds its own token store and would write the row straight back.

use sortlens_remote::supervise::{ServerState, Supervisor};
use sortlens_remote::tokens::TokenStore;
use sortlens_ui::{DeviceRow, ServerSurface};
use std::path::{Path, PathBuf};

/// The library's sync server, owned by the desktop.
pub struct Sync {
    supervisor: Option<Supervisor>,
}

impl Sync {
    /// Not serving. The server process only exists while someone asked for it: opt-in by
    /// construction, like the server's own loopback bind.
    #[must_use]
    pub fn new() -> Self {
        Self { supervisor: None }
    }

    /// Spawn the server binary that sits beside this app, against `library`.
    pub fn start(&mut self, library: &Path) -> Result<(), String> {
        if self.supervisor.is_some() {
            return Err("The server is already running".to_owned());
        }
        let exe = locate()?;
        match Supervisor::spawn(&exe, library) {
            Ok(supervisor) => {
                self.supervisor = Some(supervisor);
                Ok(())
            }
            Err(error) => Err(error.to_string()),
        }
    }

    /// Stop the process and wait for it, so the library's write lock is free when this returns.
    pub fn stop(&mut self) {
        if let Some(mut supervisor) = self.supervisor.take() {
            supervisor.stop();
        }
    }

    /// What the shell paints this frame. The device rows are read from the token file only when
    /// the Settings dialog is open — a per-frame file read for a dialog nobody is looking at
    /// would be work pretending to be state.
    #[must_use]
    pub fn surface(&mut self, library: &Path, with_devices: bool) -> ServerSurface {
        let state: ServerState = match &mut self.supervisor {
            Some(supervisor) => supervisor.state().clone(),
            None => ServerState::default(),
        };
        let devices = if with_devices { read_devices(library) } else { Vec::new() };
        ServerSurface {
            running: self.supervisor.is_some() && !state.exited,
            port: state.port,
            code: state.code,
            devices,
        }
    }

    /// Revoke a device by token. While the server runs this is refused, not silently swallowed:
    /// the running server would resurrect the row on its next write-through.
    pub fn revoke(&self, library: &Path, token: &str) -> Result<(), String> {
        if self.supervisor.is_some() {
            return Err("Stop the server before revoking a device".to_owned());
        }
        let path = tokens_path(library);
        let mut store = TokenStore::load(&path).map_err(|error| error.to_string())?;
        match store.revoke(token) {
            Ok(true) => Ok(()),
            Ok(false) => Err("That device is not paired any more".to_owned()),
            Err(error) => Err(error.to_string()),
        }
    }
}

impl Drop for Sync {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The token file, at the same default the server itself uses.
fn tokens_path(library: &Path) -> PathBuf {
    library.join("remote-tokens.json")
}

/// The server binary, beside this app. `cargo run` and the packaged bundle both keep the
/// binaries together; a missing one is a build problem worth naming, not a silent absence.
fn locate() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|error| format!("could not find this app: {error}"))?;
    let mut path = exe
        .parent()
        .ok_or_else(|| "the app has no containing directory".to_owned())?
        .to_path_buf();
    path.push(if cfg!(windows) { "sortlens-server.exe" } else { "sortlens-server" });
    if path.exists() {
        Ok(path)
    } else {
        Err(format!(
            "{} was not found — build it with the rest of the workspace",
            path.display()
        ))
    }
}

/// The device rows for the Settings list: name, scopes, issue date.
fn read_devices(library: &Path) -> Vec<DeviceRow> {
    let Ok(store) = TokenStore::load(tokens_path(library)) else {
        // A missing file is a first run; an unreadable one is the server's problem to report.
        return Vec::new();
    };
    store
        .all()
        .iter()
        .map(|token| DeviceRow {
            token: token.token.clone(),
            device: token.device.clone(),
            scopes: token
                .permissions
                .to_list()
                .iter()
                .map(|permission| permission.id())
                .collect::<Vec<_>>()
                .join(","),
            since: format_day(i64::try_from(token.created_unix).unwrap_or(0)),
        })
        .collect()
}

/// `YYYY-MM-DD` from unix seconds, without pulling in a date library — the same thirty lines of
/// civil-date arithmetic the change report uses, because one dependency for one column of one
/// dialog is not worth its advisories.
fn format_day(seconds: i64) -> String {
    if seconds <= 0 {
        return "unknown".to_owned();
    }
    let days = seconds.div_euclid(86_400);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    // January and February belong to the next March-based year, so the civil year is one past
    // the March-based one — the correction Hinnant's formula ends with.
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    /// The device list is the token file, mapped: names and scopes as ids, revocation by the
    /// exact token, and a refused revoke while a supervisor exists.
    #[test]
    fn the_device_list_is_the_token_file_and_revoke_needs_a_stopped_server() {
        let dir = std::env::temp_dir().join(format!("sortlens-sync-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let library = dir.join("library");
        std::fs::create_dir_all(&library).unwrap();

        let path = tokens_path(&library);
        let mut store = TokenStore::load(&path).unwrap();
        store
            .issue(
                "ab12cd34".to_owned(),
                "Ian's phone",
                sortlens_remote::tokens::Permissions::all(),
                1_764_000_000,
            )
            .unwrap();
        drop(store);

        let devices = read_devices(&library);
        assert_eq!(devices.len(), 1, "the paired device is listed");
        assert_eq!(devices[0].device, "Ian's phone");
        assert_eq!(devices[0].token, "ab12cd34");
        assert_eq!(devices[0].scopes, "read,upload,annotate,cull");
        assert_eq!(devices[0].since, "2025-11-24", "the issue date is a real calendar day");

        // A not-running Sync revokes the exact row; the file is rewritten without it.
        let sync = Sync::new();
        sync.revoke(&library, "ab12cd34").expect("a stopped server allows revocation");
        assert!(read_devices(&library).is_empty(), "the row is gone from the file");
        assert_eq!(
            sync.revoke(&library, "ab12cd34").expect_err("the row is already gone"),
            "That device is not paired any more"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The revoke-while-running refusal is the rule, not a nicety: a running server would bring
    /// the row back on its next write-through.
    #[test]
    fn revoke_while_the_server_runs_is_refused_by_name() {
        let dir = std::env::temp_dir().join(format!("sortlens-sync-running-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let library = dir.join("library");
        std::fs::create_dir_all(&library).unwrap();
        let mut store = TokenStore::load(tokens_path(&library)).unwrap();
        store
            .issue(
                "deadbeef".to_owned(),
                "tablet",
                sortlens_remote::tokens::Permissions::all(),
                1_764_000_000,
            )
            .unwrap();
        drop(store);

        let mut sync = Sync::new();
        // Stand in for the process: the stub prints the desktop contract's two lines and exits,
        // which is all a supervisor needs to believe a process is serving.
        let script = if cfg!(windows) {
            "echo listening http://127.0.0.1:59998 & echo pairing 135790".to_owned()
        } else {
            "printf 'listening http://127.0.0.1:59998\\npairing 135790\\n'".to_owned()
        };
        let mut command = Command::new(if cfg!(windows) { "cmd" } else { "sh" });
        command
            .arg(if cfg!(windows) { "/C" } else { "-c" })
            .arg(script)
            .stdout(Stdio::piped());
        let child = command.spawn().unwrap();
        sync.supervisor = Some(Supervisor::watch(child).unwrap());
        assert_eq!(
            sync.revoke(&library, "deadbeef").expect_err("running refuses"),
            "Stop the server before revoking a device"
        );
        assert!(
            TokenStore::load(tokens_path(&library)).unwrap().find("deadbeef").is_some(),
            "the row is untouched"
        );
        sync.stop();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The civil-date arithmetic: epoch day, a leap year, and the zero that reads as unknown.
    #[test]
    fn the_issue_date_is_a_calendar_day() {
        assert_eq!(format_day(0), "unknown");
        assert_eq!(format_day(86_400), "1970-01-02");
        assert_eq!(format_day(1_764_000_000), "2025-11-24");
        assert_eq!(format_day(951_782_400), "2000-02-29", "a leap February's last day");
        assert_eq!(format_day(951_868_800), "2000-03-01");
    }
}
