//! `sortlens-cli`: the library from a terminal.
//!
//! Same engine, same catalog, same commands as the window. This is the fastest way to check that
//! a change to the catalog survived a restart, and the base for the MCP tools later.

#![forbid(unsafe_code)]

use sortlens_core::Flag;
use sortlens_engine::{Engine, Filter, Sort};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
sortlens-cli [--library DIR] <command>

Commands:
  add <folder>              index a folder
  rescan <folder>           re-index an existing root
  roots                     list roots with photo counts
  list [--limit N]          list photos (default 20)
  stats                     library counters
  insights                  counters, cameras, lenses, keywords and years
  rate <id> <0-5>           set stars
  flag <id> pick|reject|none
  tag <id> <keyword>        add a keyword
  untag <id> <keyword>      remove a keyword
  keywords                  list keywords with photo counts
  rename <old> <new>        rename a keyword everywhere, merging if the name exists
  folders [--root R] [--in PREFIX]
                            list subfolders with recursive counts
  check                     verify every indexed file is still on disk
  relocate                  find missing files elsewhere under their root
  list [--tag KW]           filter the listing by keyword
  undo                      undo the last change
  backup <file>             write a verified backup of the index
  restore <file> <folder>   verify a backup and unpack it into a new folder
  report                    support report
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("sortlens-cli: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut library = library_dir();
    if let Some(index) = args.iter().position(|arg| arg == "--library") {
        if index + 1 >= args.len() {
            return Err("--library needs a folder".into());
        }
        library = PathBuf::from(args.remove(index + 1));
        args.remove(index);
    }
    let Some(command) = args.first().cloned() else {
        print!("{USAGE}");
        return Ok(());
    };
    let rest = &args[1..];

    let mut engine = Engine::open(&library).map_err(|error| error.to_string())?;
    let result = match command.as_str() {
        "add" | "rescan" => {
            let folder = rest.first().ok_or("a folder is required")?;
            engine.begin_scan(folder.clone()).map_err(|error| error.to_string())?;
            let done = wait(&mut engine)?;
            println!(
                "{}: {} added, {} updated, {} missing, {} restored in {:.1}s",
                done.root,
                done.added,
                done.updated,
                done.missing,
                done.restored,
                done.elapsed.as_secs_f64()
            );
            for error in &done.errors {
                eprintln!("  unreadable: {error}");
            }
            Ok(())
        }
        "roots" => {
            let counts = engine.counts_by_root();
            if counts.is_empty() {
                println!("no roots");
            }
            for (root, count) in counts {
                println!("{count:>6}  {root}");
            }
            Ok(())
        }
        "list" => {
            let limit = flag_value(rest, "--limit").unwrap_or(20);
            let filter = Filter {
                tags: text_value(rest, "--tag").into_iter().collect(),
                ..Filter::default()
            };
            let page = engine.page(&filter, Sort::Natural, 0, limit);
            for photo in &page.photos {
                let label = photo.label.map(|label| label.label()).unwrap_or("-");
                let size = if photo.width > 0 {
                    format!("{}x{}", photo.width, photo.height)
                } else {
                    "-".to_string()
                };
                println!(
                    "{:>6}  {}  {:>2}★  {:>6}  {:>9}  {:>9}  {}",
                    photo.id.0,
                    label,
                    photo.rating,
                    photo.flag.id(),
                    size,
                    sortlens_core::format_bytes(photo.size),
                    photo.rel
                );
            }
            println!("{} photos", page.total);
            Ok(())
        }
        "stats" => {
            let stats = engine.stats();
            println!("photos    {}", stats.total);
            println!("rated     {}", stats.rated);
            println!("picks     {}", stats.picks);
            println!("rejects   {}", stats.rejects);
            println!("labelled  {}", stats.labelled);
            println!("tagged    {}", stats.tagged);
            println!("missing   {}", stats.missing);
            println!("bytes     {}", sortlens_core::format_bytes(stats.bytes));
            Ok(())
        }
        "rename" => {
            let from = rest.first().ok_or("a keyword to rename is required")?;
            let to = rest.get(1).ok_or("a new name is required")?;
            let count = engine.rename_tag(from, to).map_err(|error| error.to_string())?;
            println!("renamed {from} to {to} on {count} photo(s)");
            Ok(())
        }
        "insights" => {
            let insights = engine.insights();
            println!("photos    {}", insights.stats.total);
            println!("bytes     {}", sortlens_core::format_bytes(insights.stats.bytes));
            println!("rated     {:.0}%", insights.rated_share() * 100.0);
            println!("keyworded {:.0}%", insights.tagged_share() * 100.0);
            println!("picks     {}", insights.stats.picks);
            println!("rejects   {}", insights.stats.rejects);
            for (title, rows) in [
                ("cameras", &insights.cameras),
                ("lenses", &insights.lenses),
                ("keywords", &insights.keywords),
            ] {
                if rows.is_empty() {
                    continue;
                }
                println!("{title}");
                for row in rows.iter() {
                    println!("  {:>6}  {}", row.count, row.name);
                }
            }
            if !insights.years.is_empty() {
                println!("years");
                for year in &insights.years {
                    println!("  {:>6}  {}", year.count, year.year);
                }
            }
            Ok(())
        }
        "rate" => {
            let id = parse_id(rest.first())?;
            let stars: u8 = rest.get(1).ok_or("stars are required")?.parse().map_err(|_| "stars must be 0-5")?;
            engine.rate(&[id], stars).map_err(|error| error.to_string())?;
            println!("rated {id} at {stars}");
            Ok(())
        }
        "flag" => {
            let id = parse_id(rest.first())?;
            let flag = match rest.get(1).map(String::as_str).unwrap_or("none") {
                "pick" => Flag::Pick,
                "reject" => Flag::Reject,
                "none" => Flag::None,
                other => return Err(format!("unknown flag: {other}")),
            };
            engine.flag(&[id], flag).map_err(|error| error.to_string())?;
            println!("flagged {id} as {}", flag.id());
            Ok(())
        }
        "tag" | "untag" => {
            let add = command == "tag";
            let id = parse_id(rest.first())?;
            let keyword = rest.get(1).ok_or("a keyword is required")?;
            let changed = engine.set_tag(&[id], keyword, add).map_err(|error| error.to_string())?;
            if changed == 0 {
                println!("{id}: no change");
            } else {
                println!("{id}: {} {keyword}", if add { "tagged" } else { "untagged" });
            }
            Ok(())
        }
        "keywords" => {
            let vocabulary = engine.tag_vocabulary();
            if vocabulary.is_empty() {
                println!("no keywords");
            }
            for (tag, count) in vocabulary {
                println!("{count:>6}  {tag}");
            }
            Ok(())
        }
        "folders" => {
            let roots = engine.counts_by_root();
            let root = text_value(rest, "--root").unwrap_or_else(|| roots.first().map(|(root, _)| root.clone()).unwrap_or_default());
            let prefix = text_value(rest, "--in").unwrap_or_default();
            let folders = engine.subfolders(&root, &prefix);
            if folders.is_empty() {
                println!("{root}/{prefix} has no subfolders");
            }
            for (name, count) in folders {
                println!("{count:>6}  {prefix}{name}/");
            }
            Ok(())
        }
        "check" => {
            let value = engine
                .run("library.health", &serde_json::json!({}))
                .map_err(|error| error.to_string())?;
            println!(
                "{} of {} files present, {} missing",
                value["present"], value["checked"], value["missing"]
            );
            Ok(())
        }
        "relocate" => {
            let value = engine
                .run("library.relocate", &serde_json::json!({}))
                .map_err(|error| error.to_string())?;
            println!("relocated {}", value["relocated"]);
            Ok(())
        }
        "backup" => {
            let path = rest.first().ok_or("a destination file is required")?;
            let files = engine
                .export_backup(std::path::Path::new(path))
                .map_err(|error| error.to_string())?;
            println!("backed up {files} file(s) to {path}");
            Ok(())
        }
        "restore" => {
            let path = rest.first().ok_or("a backup file is required")?;
            let dest = rest.get(1).ok_or("a destination folder is required")?;
            let files = Engine::import_backup(std::path::Path::new(path), std::path::Path::new(dest)).map_err(|error| error.to_string())?;
            println!("restored {files} file(s) into {dest}");
            Ok(())
        }
        "undo" => {
            let undone = engine.undo().map_err(|error| error.to_string())?;
            println!("{}", if undone { "undone" } else { "nothing to undo" });
            Ok(())
        }
        "report" => {
            let value = engine
                .run("app.supportReport", &serde_json::json!({}))
                .map_err(|error| error.to_string())?;
            println!("{}", serde_json::to_string_pretty(&value).unwrap_or_default());
            Ok(())
        }
        other => Err(format!("unknown command: {other}\n\n{USAGE}")),
    };

    let _ = engine.close();
    result
}

fn wait(engine: &mut Engine) -> Result<sortlens_engine::ScanFinished, String> {
    loop {
        if let Some(done) = engine.poll() {
            return Ok(done);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn parse_id(value: Option<&String>) -> Result<sortlens_core::PhotoId, String> {
    value
        .ok_or_else(|| "a photo id is required".to_string())?
        .parse::<u64>()
        .map(sortlens_core::PhotoId)
        .map_err(|_| "photo id must be a number".to_string())
}

fn text_value(args: &[String], name: &str) -> Option<String> {
    let index = args.iter().position(|arg| arg == name)?;
    args.get(index + 1).cloned()
}

fn flag_value(args: &[String], name: &str) -> Option<usize> {
    let index = args.iter().position(|arg| arg == name)?;
    args.get(index + 1)?.parse().ok()
}

fn library_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("SORTLENS_LIBRARY_DIR")
        && !dir.trim().is_empty()
    {
        return PathBuf::from(dir);
    }
    let base = if cfg!(target_os = "macos") {
        home().join("Library").join("Application Support")
    } else if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join("AppData").join("Roaming"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".local").join("share"))
    };
    base.join("Sortlens").join("Library")
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}
