//! L4: the command layer.
//!
//! The engine owns the registry and, from M1 on, the open catalog and the job pool. The UI, the
//! CLI and the MCP server all call [`Engine::run`] with a command id and a JSON argument object;
//! nothing above this layer touches the catalog directly.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

pub mod command;

pub use command::{CommandSpec, Group, Registry, Status};
pub use sortlens_core::{Error, Result};

use serde_json::{Value, json};

/// What a command asks the caller to do when the effect is not the engine's to perform.
///
/// `app.quit` and `app.settings` are real commands, but quitting is the window's job and the
/// settings dialog belongs to the shell, so the engine returns the intent instead of pretending
/// it did something.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Nothing further; the result value is the answer.
    None,
    /// Ask the host to open the settings dialog.
    OpenSettings,
    /// Ask the host to close the window.
    Quit,
}

/// The command engine.
#[derive(Debug)]
pub struct Engine {
    registry: Registry,
    last_effect: Effect,
}

impl Engine {
    /// Build an engine with the M0 command table and no catalog open.
    pub fn new() -> Self {
        Self {
            registry: Registry::v0(),
            last_effect: Effect::None,
        }
    }

    /// The command table, for menus and the palette.
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// The effect requested by the most recent [`Self::run`] call.
    pub fn last_effect(&self) -> Effect {
        self.last_effect
    }

    /// Run one command.
    ///
    /// Planned commands fail with [`Error::NotYetAvailable`] naming their milestone; the UI
    /// disables them, so reaching this path is a bug in the caller rather than a user action.
    pub fn run(&mut self, command: &str, args: &Value) -> Result<Value> {
        self.last_effect = Effect::None;
        let spec = *self.registry.expect(command)?;
        match spec.status {
            Status::Planned(_) => {
                return Err(Error::NotYetAvailable(match command {
                    "library.addFolder" => "folder indexing arrives in M1",
                    "library.rescan" => "rescanning arrives in M1",
                    "library.import" => "card import arrives in M1",
                    "photo.rate" | "photo.flag" | "photo.label" => "rating and flags arrive in M3",
                    "photo.keyword" => "keywords arrive in M3",
                    "collection.new" | "collection.addTo" => "collections arrive in M5",
                    "cull.start" | "cull.apply" | "cull.undo" => "culling arrives in M4",
                    "view.detail" => "the detail view arrives in M4",
                    "view.similar" => "similar search arrives in M6",
                    "view.places" => "the map arrives in M5",
                    "view.insights" => "insights arrive in M6",
                    _ => "this command is planned",
                }));
            }
            Status::Ready => {}
        }

        match command {
            "app.about" => Ok(json!({
                "name": "Sortlens",
                "version": sortlens_core::VERSION,
                "engine": VERSION,
                "commands": self.registry.all().len(),
                "readyCommands": self.registry.ready_count(),
            })),
            "app.supportReport" => Ok(json!({
                "sortlens": sortlens_core::VERSION,
                "engine": VERSION,
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "commands": self.registry.all().len(),
                "planned": self.registry.all().len() - self.registry.ready_count(),
                "reason": args.get("reason").and_then(Value::as_str).unwrap_or("user request"),
            })),
            "app.settings" => {
                self.last_effect = Effect::OpenSettings;
                Ok(json!({ "opened": "settings" }))
            }
            "app.quit" => {
                self.last_effect = Effect::Quit;
                Ok(json!({ "quitting": true }))
            }
            other => Err(Error::Invalid(format!("command has no handler: {other}"))),
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn about_reports_the_build() {
        let mut engine = Engine::new();
        let about = engine.run("app.about", &json!({})).expect("about is ready");
        assert_eq!(about["name"], "Sortlens");
        assert_eq!(about["version"], sortlens_core::VERSION);
    }

    #[test]
    fn planned_commands_say_when_they_arrive() {
        let mut engine = Engine::new();
        let error = engine.run("library.addFolder", &json!({})).expect_err("planned");
        let message = error.to_string();
        assert!(message.contains("M1"), "{message}");
    }

    #[test]
    fn unknown_commands_are_rejected() {
        let mut engine = Engine::new();
        assert!(matches!(engine.run("nope.nope", &json!({})), Err(Error::Invalid(_))));
    }

    #[test]
    fn settings_asks_the_host_to_open_the_dialog() {
        let mut engine = Engine::new();
        engine.run("app.settings", &json!({})).expect("settings is ready");
        assert_eq!(engine.last_effect(), Effect::OpenSettings);
    }

    #[test]
    fn quit_asks_the_host_to_close() {
        let mut engine = Engine::new();
        engine.run("app.quit", &json!({})).expect("quit is ready");
        assert_eq!(engine.last_effect(), Effect::Quit);
    }
}
