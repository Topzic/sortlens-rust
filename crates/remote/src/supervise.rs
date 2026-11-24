//! The desktop's own handle on the server process.
//!
//! The desktop application does not open the port itself: it spawns the `sortlens-server`
//! binary that sits beside it and reads the startup lines the process prints —
//! `listening http://host:port` and `pairing NNNNNN`. Those lines are a contract between two
//! processes (the loopback harness reads them the same way), so the parsing lives here, beside
//! the protocol it belongs to, and the desktop stays a thin shell over it.
//!
//! The server reprints `pairing NNNNNN` whenever a code expires and a fresh one is drawn, so a
//! desktop that keeps draining the pipe always shows the code that is live right now.

use sortlens_core::Error as CoreError;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// Result type for this module.
pub type Result<T> = std::result::Result<T, CoreError>;

/// One fact the server announced on its stdout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Startup {
    /// `listening http://127.0.0.1:52114` — the process is serving on this port. The host part
    /// is loopback or `0.0.0.0`; only the port matters to a supervisor that spawned it.
    Listening(u16),
    /// `pairing 448821` — the six-digit code that is live right now.
    Pairing(String),
}

/// Parse one startup line. Anything else — the version banner, the album and token paths, a
/// blank line — is `None`, not an error: a supervisor reacts only to what it knows, and an
/// unknown line from a newer server is news, not a crash.
#[must_use]
pub fn parse_line(line: &str) -> Option<Startup> {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix("listening http://") {
        // `host:port`; the host may itself contain no colon here (an address or `0.0.0.0`), so
        // the port is whatever follows the last one. An IPv6 host would break this, and the
        // server does not bind one.
        return rest
            .rsplit(':')
            .next()
            .and_then(|port| port.parse::<u16>().ok())
            .map(Startup::Listening);
    }
    if let Some(rest) = line.strip_prefix("pairing ") {
        let code = rest.trim();
        // A well-formed code is exactly six digits; anything else is not a code the pairing
        // gate would have issued, so it is not a fact worth reporting.
        if code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit()) {
            return Some(Startup::Pairing(code.to_owned()));
        }
        return None;
    }
    None
}

/// What the supervised process has announced and whether it is still alive.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServerState {
    /// The port the server is serving on, once its startup line has arrived.
    pub port: Option<u16>,
    /// The pairing code that is live right now, refreshed on every reprint.
    pub code: Option<String>,
    /// The process is gone — it exited, was killed, or never started serving.
    pub exited: bool,
}

/// One spawned server process, watched from a thread that drains its stdout.
///
/// The desktop owns zero or one of these; it is created by [`Supervisor::spawn`], asked for
/// [`Self::state`] every frame and stopped when the window closes. Dropping it kills the child:
/// a desktop that exits without tidying up must not leave a server holding the library's lock.
pub struct Supervisor {
    child: Child,
    events: std::sync::mpsc::Receiver<Startup>,
    state: ServerState,
}

impl Supervisor {
    /// Spawn the server binary against `library`, loopback only, and start draining its stdout.
    ///
    /// The port is left at the server's ephemeral default on purpose: the desktop learns it from
    /// the startup line rather than choosing one, so two libraries served at once never argue.
    pub fn spawn(server_exe: impl AsRef<Path>, library: impl AsRef<Path>) -> Result<Self> {
        let library = library.as_ref().to_path_buf();
        let mut command = Command::new(server_exe.as_ref());
        command
            .arg("--library")
            .arg(&library)
            .arg("--port")
            .arg("0")
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let child = command
            .spawn()
            .map_err(|error| CoreError::io(server_exe.as_ref().display().to_string(), error))?;
        Self::watch(child)
    }

    /// Watch an already-spawned child: drain its stdout and report whether it is alive.
    /// [`Self::spawn`] uses it; a test that stands in a stub process for the server uses it too.
    pub fn watch(mut child: Child) -> Result<Self> {
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CoreError::Invalid("the server's stdout is not readable".to_owned()))?;
        let (sender, events) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("sortlens-server-stdout".to_owned())
            .spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(std::result::Result::ok) {
                    if let Some(startup) = parse_line(&line)
                        && sender.send(startup).is_err()
                    {
                        break;
                    }
                }
            })
            .map_err(|error| CoreError::Invalid(format!("could not start the stdout reader: {error}")))?;
        Ok(Self {
            child,
            events,
            state: ServerState::default(),
        })
    }

    /// Fold everything the process has announced and whether it is still alive into the state,
    /// and hand it back. Called every frame, so a reprint replaces the expired code and a crash
    /// is visible on the next repaint.
    pub fn state(&mut self) -> &ServerState {
        while let Ok(event) = self.events.try_recv() {
            match event {
                Startup::Listening(port) => self.state.port = Some(port),
                Startup::Pairing(code) => self.state.code = Some(code),
            }
        }
        if !self.state.exited && matches!(self.child.try_wait(), Ok(Some(_)) | Err(_)) {
            self.state.exited = true;
        }
        &self.state
    }

    /// Stop the process and wait for it, so the library's write lock is free the moment this
    /// returns. A supervisor that was asked to stop is not resurrected.
    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.state.exited = true;
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The supervisor's own tests: the line contract, and the state a burst of reprints folds into.
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// The two startup lines the server prints are the contract; nothing else parses.
    #[test]
    fn the_startup_lines_parse_and_nothing_else_does() {
        assert_eq!(parse_line("listening http://127.0.0.1:52114"), Some(Startup::Listening(52_114)));
        assert_eq!(parse_line("listening http://0.0.0.0:8080"), Some(Startup::Listening(8_080)));
        assert_eq!(parse_line("pairing 448821"), Some(Startup::Pairing("448821".to_owned())));
        // The banner and the paths the server also prints are not facts the desktop reacts to.
        assert_eq!(parse_line("sortlens-server 0.1.0"), None);
        assert_eq!(parse_line("album /tmp/inbox"), None);
        assert_eq!(parse_line("tokens /library/remote-tokens.json"), None);
        assert_eq!(parse_line(""), None);
        // A malformed code is not a code: six digits exactly.
        assert_eq!(parse_line("pairing 44882"), None);
        assert_eq!(parse_line("pairing 44882a"), None);
        assert_eq!(parse_line("pairing"), None);
    }

    /// A reprint of the pairing line replaces the expired code, not appends to a history.
    #[test]
    fn a_reprinted_code_replaces_the_old_one() {
        let script = if cfg!(windows) {
            "echo pairing 111111 & echo listening http://127.0.0.1:59999 & echo pairing 222222".to_owned()
        } else {
            "printf 'pairing 111111\\nlistening http://127.0.0.1:59999\\npairing 222222\\n'".to_owned()
        };
        let mut command = Command::new(if cfg!(windows) { "cmd" } else { "sh" });
        if cfg!(windows) {
            command.arg("/C");
        } else {
            command.arg("-c");
        }
        command.arg(script).stdout(Stdio::piped()).stderr(Stdio::null());
        let child = command.spawn().unwrap();
        let mut supervisor = Supervisor::watch(child).unwrap();
        // Poll until the last line has been read, so the test does not race the reader thread:
        // the first code is superseded by the reprint, so the wait is on the *pair*.
        for _ in 0..200 {
            let state = supervisor.state();
            if state.port.is_some() && state.code.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let state = supervisor.state().clone();
        assert_eq!(state.port, Some(59_999), "the port arrived");
        assert_eq!(state.code.as_deref(), Some("222222"), "the reprint replaced the first code");
        supervisor.stop();
        assert!(supervisor.state().exited, "a stopped supervisor reports itself gone");
    }
}
