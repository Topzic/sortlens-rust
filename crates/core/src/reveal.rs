//! Showing a file to the operating system: the two commands, and nothing else.
//!
//! This module **builds** commands; it never runs them. That split is deliberate:
//!
//! - the engine and the wasm client can both ask "what would I run?" without a process table,
//! - the decision is pure data, so the exact argv for each platform can be asserted in a test,
//! - spawning stays in the two desktop hosts (the app and the CLI), which is the only place an OS
//!   process belongs.
//!
//! Arguments are always a list, never one shell string: a photo called `$HOME; rm -rf ~.jpg` must
//! mean a file with an awkward name, not a command.

#![forbid(unsafe_code)]

use std::path::Path;

/// The desktop the command is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Desktop {
    /// `open`, with `-R` to select rather than launch.
    MacOs,
    /// `explorer /select,` — reliable for revealing, awkward for everything else.
    Windows,
    /// `xdg-open`, which can open a file but cannot select one in its folder.
    Linux,
}

impl Desktop {
    /// The desktop this binary was built for, or `None` when there is none (wasm, or an OS with no
    /// integration yet). A caller that gets `None` says so rather than guessing.
    pub const fn current() -> Option<Self> {
        if cfg!(target_os = "macos") {
            Some(Self::MacOs)
        } else if cfg!(target_os = "windows") {
            Some(Self::Windows)
        } else if cfg!(target_os = "linux") || cfg!(target_os = "freebsd") {
            Some(Self::Linux)
        } else {
            None
        }
    }

    /// A stable name for the command registry and for error messages.
    pub const fn id(self) -> &'static str {
        match self {
            Self::MacOs => "macos",
            Self::Windows => "windows",
            Self::Linux => "linux",
        }
    }
}

/// A program and its arguments, ready for `Command::new(program).args(args)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// The program to run, as found on `PATH` or an absolute path.
    pub program: String,
    /// Arguments, in order, each passed as one argument.
    pub args: Vec<String>,
}

impl Command {
    /// Build one from parts.
    pub fn new(program: impl Into<String>, args: impl IntoIterator<Item = String>) -> Self {
        Self {
            program: program.into(),
            args: args.into_iter().collect(),
        }
    }

    /// The command as a line a person can read. Only for messages and tests: never parse it back.
    pub fn display(&self) -> String {
        let mut line = self.program.clone();
        for arg in &self.args {
            line.push(' ');
            if arg.contains(' ') || arg.contains('"') {
                line.push('"');
                line.push_str(&arg.replace('"', "\\\""));
                line.push('"');
            } else {
                line.push_str(arg);
            }
        }
        line
    }
}

/// Show the file in the desktop's file manager.
///
/// macOS and Windows select the file in its folder. Linux cannot: `xdg-open` opens a path, and file
/// managers differ on whether they accept a selection flag, so the containing folder is opened
/// instead. That is a deviation from the other two, and the caller should say so rather than pretend
/// the photo is selected.
pub fn reveal(path: &Path, desktop: Desktop) -> Command {
    let path = path.to_string_lossy().to_string();
    match desktop {
        Desktop::MacOs => Command::new("open", ["-R".to_string(), path]),
        Desktop::Windows => Command::new("explorer", [format!("/select,{path}")]),
        Desktop::Linux => Command::new("xdg-open", [containing_folder(&path)]),
    }
}

/// Whether [`reveal`] selects the file itself rather than opening its folder.
pub const fn reveal_selects(desktop: Desktop) -> bool {
    !matches!(desktop, Desktop::Linux)
}

/// Hand the file to the default application.
pub fn open_with_default(path: &Path, desktop: Desktop) -> Command {
    let path = path.to_string_lossy().to_string();
    match desktop {
        Desktop::MacOs => Command::new("open", [path]),
        // `start` needs an empty title argument first, or a quoted path is taken for the title.
        Desktop::Windows => Command::new("cmd", ["/c".to_string(), "start".to_string(), String::new(), path]),
        Desktop::Linux => Command::new("xdg-open", [path]),
    }
}

/// The folder a path lives in, as a string; the path itself when it has no parent.
fn containing_folder(path: &str) -> String {
    Path::new(path)
        .parent()
        .map(|parent| {
            let text = parent.to_string_lossy().to_string();
            if text.is_empty() { ".".to_string() } else { text }
        })
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn reveal_selects_the_file_where_the_desktop_can() {
        let path = Path::new("/photos/2024/beach.jpg");
        assert_eq!(
            reveal(path, Desktop::MacOs),
            Command::new("open", ["-R".to_string(), "/photos/2024/beach.jpg".to_string()])
        );
        assert_eq!(
            reveal(Path::new(r"C:\photos\beach.jpg"), Desktop::Windows),
            Command::new("explorer", [r"/select,C:\photos\beach.jpg".to_string()])
        );
        // Linux opens the folder, and says so rather than implying a selection.
        assert_eq!(reveal(path, Desktop::Linux), Command::new("xdg-open", ["/photos/2024".to_string()]));
        assert!(reveal_selects(Desktop::MacOs));
        assert!(reveal_selects(Desktop::Windows));
        assert!(!reveal_selects(Desktop::Linux));
    }

    #[test]
    fn opening_hands_the_file_to_the_default_application() {
        let path = Path::new("/photos/beach.jpg");
        assert_eq!(
            open_with_default(path, Desktop::MacOs),
            Command::new("open", ["/photos/beach.jpg".to_string()])
        );
        assert_eq!(
            open_with_default(path, Desktop::Linux),
            Command::new("xdg-open", ["/photos/beach.jpg".to_string()])
        );
        assert_eq!(
            open_with_default(path, Desktop::Windows),
            Command::new(
                "cmd",
                [
                    "/c".to_string(),
                    "start".to_string(),
                    String::new(),
                    "/photos/beach.jpg".to_string()
                ]
            ),
            "start's empty first argument is the window title, not the file"
        );
    }

    #[test]
    fn an_awkward_filename_is_one_argument_not_a_command() {
        let path = Path::new("/photos/$(rm -rf ~).jpg");
        for desktop in [Desktop::MacOs, Desktop::Linux] {
            let command = open_with_default(path, desktop);
            assert_eq!(command.args.len(), 1);
            assert_eq!(command.args[0], "/photos/$(rm -rf ~).jpg");
        }
        let windows = reveal(Path::new(r"C:\my photos\a b.jpg"), Desktop::Windows);
        assert_eq!(windows.args, vec![r"/select,C:\my photos\a b.jpg".to_string()]);
    }

    #[test]
    fn a_display_line_quotes_but_is_only_for_reading() {
        let command = Command::new("open", ["/a b/c.jpg".to_string(), "/plain.jpg".to_string()]);
        assert_eq!(command.display(), "open \"/a b/c.jpg\" /plain.jpg");
        let quote = Command::new("open", ["/a\"b.jpg".to_string()]);
        assert_eq!(quote.display(), "open \"/a\\\"b.jpg\"");
    }

    #[test]
    fn a_relative_path_in_a_folder_still_reveals_something_openable() {
        assert_eq!(
            reveal(Path::new("beach.jpg"), Desktop::Linux),
            Command::new("xdg-open", [".".to_string()])
        );
        assert_eq!(containing_folder("/beach.jpg"), "/");
    }

    #[test]
    fn this_build_knows_which_desktop_it_is_on() {
        // The desktop hosts are macOS, Windows and Linux; wasm has none.
        if cfg!(target_arch = "wasm32") {
            assert!(Desktop::current().is_none());
        } else {
            let desktop = Desktop::current().expect("a desktop build");
            assert!(["macos", "windows", "linux"].contains(&desktop.id()));
        }
    }
}
