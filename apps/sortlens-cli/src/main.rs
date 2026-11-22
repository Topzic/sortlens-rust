//! `sortlens-cli`: the library from a terminal.
//!
//! Same engine, same catalog, same commands as the window. This is the fastest way to check that
//! a change to the catalog survived a restart, and the base for the MCP tools later.

#![forbid(unsafe_code)]

use serde_json::Value;
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
  date <id> <YYYY-MM-DD|->  set (or clear, with -) the capture date
  tag <id> <keyword>        add a keyword
  untag <id> <keyword>      remove a keyword
  keywords                  list keywords with photo counts
  rename <old> <new>        rename a keyword everywhere, merging if the name exists
  mcp                       serve the command registry over MCP on stdio, then exit
  reveal <id>               show the file in the desktop's file manager
  open <id>                 hand the file to the desktop's default application
  page [--offset N] [--limit N] [--sort KEY]
                            one page of photos as JSON, for scripts
  memories [--today D]      resurfacing cards; D is YYYY-MM-DD, default today
  shortcuts                 every command that has a keyboard shortcut
  registry                  keywords, the names they swallowed, and orphaned aliases
  alias <name> [canonical]  link a keyword to another, or forget the link
  folders [--root R] [--in PREFIX]
                            list subfolders with recursive counts
  check                     verify every indexed file is still on disk
  relocate                  find missing files elsewhere under their root
  list [--tag KW]           filter the listing by keyword
  undo                      undo the last change
  remap <old> <new>         repoint a moved or renamed root, sample checked
  history [--limit N]       recent changes, newest first
  retry [--all]             re-read metadata for photos that failed, or for all without EXIF
  move <dest> <id...>       move photos into a folder, keeping their review state
  moves                     list recent moves with their ids
  unmove <id>               put a moved batch back where it came from
  export <dest> [ids...]    write picks out; no ids means the whole library
                            --format original|jpeg --max N --quality N
                            --collision skip|rename|overwrite --no-metadata
  changed                   list files whose size or time no longer matches the index
  refresh                   re-read exactly the changed files
  audit                     check the catalog files read back cleanly
  compact                   snapshot and empty the log
  backup <file>             write a verified backup of the index
  restore <file> <folder>   verify a backup and unpack it into a new folder
  report                    support report
";

/// Star, flag, capture date or keyword, on one photo.
fn edit_photo(engine: &mut Engine, command: &str, rest: &[String]) -> Result<(), String> {
    let id = parse_id(rest.first())?;
    match command {
        "rate" => {
            let stars: u8 = rest.get(1).ok_or("stars are required")?.parse().map_err(|_| "stars must be 0-5")?;
            engine.rate(&[id], stars).map_err(|error| error.to_string())?;
            println!("rated {id} at {stars}");
        }
        "flag" => {
            let flag = match rest.get(1).map(String::as_str).unwrap_or("none") {
                "pick" => Flag::Pick,
                "reject" => Flag::Reject,
                "none" => Flag::None,
                other => return Err(format!("unknown flag: {other}")),
            };
            engine.flag(&[id], flag).map_err(|error| error.to_string())?;
            println!("flagged {id} as {}", flag.id());
        }
        "date" => {
            let text = rest.get(1).ok_or("a date or - is required")?;
            let captured = if text == "-" { None } else { Some(parse_day(text)?) };
            let changed = engine.set_captured(&[(id, captured)]).map_err(|error| error.to_string())?;
            match (captured, changed) {
                (Some(at), 1) => println!("{id}: captured {at}"),
                (None, 1) => println!("{id}: capture date cleared"),
                _ => println!("{id}: no change"),
            }
        }
        _ => {
            let keyword = rest.get(1).ok_or("a keyword is required")?;
            apply_keyword(engine, id, keyword, command == "tag")?;
        }
    }
    Ok(())
}

/// Print the resurfacing cards, optionally for a given day.
fn print_memories(engine: &mut Engine, rest: &[String]) -> Result<(), String> {
    let today = rest
        .windows(2)
        .find(|pair| pair[0] == "--today")
        .map(|pair| parse_day(&pair[1]))
        .transpose()?;
    let value = engine
        .run("memories.list", &serde_json::json!({ "today": today }))
        .map_err(|error| error.to_string())?;
    println!("{}", value["headline"].as_str().unwrap_or_default());
    for card in value["cards"].as_array().into_iter().flatten() {
        println!(
            "  {}  ({}; {} matched)",
            card["title"].as_str().unwrap_or("-"),
            card["detail"].as_str().unwrap_or("-"),
            card["matched"]
        );
        let ids: Vec<String> = card["photos"].as_array().into_iter().flatten().map(|id| id.to_string()).collect();
        println!("    {}", ids.join(" "));
    }
    Ok(())
}

/// `YYYY-MM-DD` to the Unix seconds of that midnight UTC.
fn parse_day(text: &str) -> Result<i64, String> {
    let parts: Vec<&str> = text.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return Err(format!("{text} is not YYYY-MM-DD"));
    };
    let (Ok(year), Ok(month), Ok(day)) = (year.parse::<i64>(), month.parse::<u32>(), day.parse::<u32>()) else {
        return Err(format!("{text} is not YYYY-MM-DD"));
    };
    if month == 0 || month > 12 || day == 0 || day > sortlens_memories::days_in_month(year, month) {
        return Err(format!("{text} is not a real date"));
    }
    Ok(sortlens_memories::midnight(sortlens_memories::Date { year, month, day }))
}

/// Add or remove one keyword, naming the keyword that was really used.
fn apply_keyword(engine: &mut Engine, id: sortlens_core::PhotoId, keyword: &str, add: bool) -> Result<(), String> {
    let resolved = engine.resolve_tag(keyword);
    let changed = engine.set_tag(&[id], keyword, add).map_err(|error| error.to_string())?;
    let verb = if add { "tagged" } else { "untagged" };
    if changed == 0 {
        println!("{id}: no change");
    } else if resolved == keyword.trim().to_lowercase() {
        println!("{id}: {verb} {resolved}");
    } else {
        println!("{id}: {verb} {resolved} ({keyword} is an alias)");
    }
    Ok(())
}

/// Print the keyword registry: counts, swallowed names, and aliases pointing nowhere.
fn print_registry(engine: &Engine) {
    let registry = engine.tag_registry();
    println!("{}", registry.headline());
    for entry in &registry.entries {
        let aliases = if entry.aliases.is_empty() {
            String::new()
        } else {
            format!("  <- {}", entry.aliases.join(", "))
        };
        println!("  {:>6}  {}{aliases}", entry.count, entry.name);
    }
    for (alias, canonical) in &registry.orphans {
        println!("  orphan: {alias} -> {canonical}");
    }
}

/// Link a keyword to another, or forget the link when no target is given.
fn link_alias(engine: &mut Engine, rest: &[String]) -> Result<(), String> {
    let alias = rest.first().ok_or("a keyword is required")?;
    let canonical = rest.get(1).map(String::as_str);
    let changed = engine.set_alias(alias, canonical).map_err(|error| error.to_string())?;
    match (canonical, changed) {
        (Some(canonical), true) => println!("{alias} now means {canonical}"),
        (Some(canonical), false) => println!("{alias} already means {canonical}"),
        (None, true) => println!("forgot the alias {alias}"),
        (None, false) => println!("{alias} was not an alias"),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("sortlens-cli: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Write picks out of the library, and wait for the worker to finish.
///
/// Pulled out of `run` because the argument reading is longer than the other verbs and the command
/// table should stay a table.
fn export(engine: &mut Engine, rest: &[String]) -> Result<(), String> {
    let dest = rest.first().ok_or("a destination folder is required")?;
    // Values belong to flags, not to the photo list: `--max 24` is a size, and reading it as "photo
    // 24" was a real bug the golden job caught the moment the verb was recorded.
    let mut ids: Vec<sortlens_core::PhotoId> = Vec::new();
    let mut index = 1;
    while index < rest.len() {
        let arg = &rest[index];
        if let Some(flag) = arg.strip_prefix("--") {
            let takes_value = matches!(flag, "format" | "max" | "quality" | "collision" | "library");
            index += if takes_value { 2 } else { 1 };
            continue;
        }
        if let Ok(value) = arg.parse::<u64>() {
            ids.push(sortlens_core::PhotoId(value));
        }
        index += 1;
    }
    if ids.is_empty() {
        // No ids means "everything the filters would show" — the common case from a script.
        ids = engine.all_photos().iter().map(|photo| photo.id).collect();
    }
    let mut spec = sortlens_engine::ExportSpec::default();
    if let Some(format) = text_value(rest, "--format") {
        spec.format = sortlens_engine::ExportFormat::parse(&format).ok_or_else(|| format!("there is no {format} export"))?;
    }
    if let Some(edge) = flag_value(rest, "--max") {
        spec.max_edge = Some(u32::try_from(edge).map_err(|_| "that edge is out of range".to_string())?);
    }
    if let Some(quality) = flag_value(rest, "--quality") {
        spec.quality = u8::try_from(quality).map_err(|_| "that quality is out of range".to_string())?;
    }
    if rest.iter().any(|arg| arg == "--no-metadata") {
        spec.metadata = false;
    }
    if let Some(collision) = text_value(rest, "--collision") {
        spec.collision =
            sortlens_engine::Collision::parse(&collision).ok_or_else(|| format!("there is no {collision} collision policy"))?;
    }
    let planned = engine
        .begin_export(std::path::Path::new(dest), &ids, &spec)
        .map_err(|error| error.to_string())?;
    if planned == 0 {
        match engine.last_export() {
            Some(report) => println!("nothing to write · {}", report.headline()),
            None => println!("nothing to write"),
        }
        return Ok(());
    }
    for _ in 0..1200 {
        if let Some(report) = engine.poll_export() {
            for failure in &report.failures {
                eprintln!("  failed: {failure}");
            }
            println!("{} -> {}", report.headline(), report.dest.display());
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    Err("the export did not finish".into())
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
            if insights.by_month.iter().any(|count| *count > 0) {
                println!("months");
                for (index, count) in insights.by_month.iter().enumerate() {
                    if *count > 0 {
                        println!("  {:>6}  {}", count, sortlens_engine::month_name(index as u32 + 1));
                    }
                }
            }
            for highlight in insights.highlights() {
                println!("{:<18} {}", highlight.label.to_lowercase(), highlight.value);
            }
            Ok(())
        }
        // The photo-field verbs are all "read the id, read the value, write it" — one helper, so the
        // command table stays a table.
        "rate" | "flag" | "date" | "tag" | "untag" => edit_photo(&mut engine, &command, rest),
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
        "move" => {
            let dest = rest.first().ok_or("a destination folder is required")?;
            let ids: Vec<sortlens_core::PhotoId> = rest
                .iter()
                .skip(1)
                .filter_map(|value| value.parse::<u64>().ok())
                .map(sortlens_core::PhotoId)
                .collect();
            let count = engine.move_picks(dest, &ids).map_err(|error| error.to_string())?;
            println!("moved {count} photo(s) to {dest}");
            Ok(())
        }
        "export" => export(&mut engine, rest),
        "moves" => {
            if engine.move_receipts().is_empty() {
                println!("no moves recorded");
            }
            for receipt in engine.move_receipts() {
                println!("#{:<4} {:<40} {}", receipt.id, receipt.headline(), receipt.dest);
            }
            Ok(())
        }
        "unmove" => {
            let id = rest
                .first()
                .and_then(|value| value.parse::<u64>().ok())
                .ok_or("a move id is required")?;
            let count = engine.undo_move(id).map_err(|error| error.to_string())?;
            println!("put {count} photo(s) back");
            Ok(())
        }
        "changed" => {
            let health = engine.check_health().map_err(|error| error.to_string())?;
            println!(
                "{} checked, {} present, {} missing, {} changed",
                health.checked, health.present, health.missing, health.modified
            );
            for id in &health.modified_ids {
                if let Some(photo) = engine.photo(*id) {
                    println!("  {}  {}", photo.rel, photo.root);
                }
            }
            Ok(())
        }
        "refresh" => {
            let count = engine.refresh_changed().map_err(|error| error.to_string())?;
            println!("refreshed {count} photo(s)");
            Ok(())
        }
        "retry" => {
            let all = rest.iter().any(|arg| arg == "--all");
            let (updated, remaining) = engine.retry_metadata(all).map_err(|error| error.to_string())?;
            println!("updated {updated}, {remaining} still unreadable");
            Ok(())
        }
        "history" => {
            let limit = rest
                .iter()
                .position(|arg| arg == "--limit")
                .and_then(|index| rest.get(index + 1))
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(20);
            let entries = engine.history(limit).map_err(|error| error.to_string())?;
            if entries.is_empty() {
                println!("no changes since the last snapshot");
            }
            for entry in entries {
                println!("#{:<6} {:<16} {}", entry.seq, entry.kind, entry.summary);
            }
            Ok(())
        }
        "audit" => {
            let audit = engine.audit().map_err(|error| error.to_string())?;
            println!("{}", audit.headline());
            println!("{}", audit.detail());
            for note in &audit.notes {
                println!("note: {note}");
            }
            Ok(())
        }
        "compact" => {
            let bytes = engine.compact().map_err(|error| error.to_string())?;
            println!("compacted · {} reclaimed", sortlens_core::format_bytes(bytes));
            Ok(())
        }
        "remap" => {
            let from = rest.first().ok_or("the current folder is required")?;
            let to = rest.get(1).ok_or("the new folder is required")?;
            let count = engine.remap_root(from, to).map_err(|error| error.to_string())?;
            println!("repointed {count} photo(s) to {to}");
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
        "mcp" => {
            // One line in, one line out, until the client closes the pipe. Nothing is printed to
            // stdout except protocol messages: a stray println would corrupt the stream.
            let mut server = sortlens_mcp::Mcp::new(engine);
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            server.serve(stdin.lock(), stdout.lock()).map_err(|error| format!("mcp: {error}"))?;
            return Ok(());
        }
        "reveal" | "open" => {
            let id = parse_id(rest.first())?;
            let value = engine
                .run(
                    if command == "reveal" { "photo.reveal" } else { "photo.open" },
                    &serde_json::json!({ "id": id.0 }),
                )
                .map_err(|error| error.to_string())?;
            let program = value["program"].as_str().unwrap_or("open");
            let args: Vec<&str> = value["args"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|arg| arg.as_str())
                .collect();
            // The engine built the command; the CLI is a desktop host, so it runs it.
            match std::process::Command::new(program)
                .args(&args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
            {
                Ok(_) => println!("{}", value["line"].as_str().unwrap_or_default()),
                Err(error) => return Err(format!("could not run {program}: {error}")),
            }
            Ok(())
        }
        "page" => {
            let flag = |name: &str| rest.windows(2).find(|pair| pair[0] == name).map(|pair| pair[1].clone());
            let offset = flag("--offset").and_then(|value| value.parse::<u64>().ok()).unwrap_or(0);
            let limit = flag("--limit").and_then(|value| value.parse::<u64>().ok()).unwrap_or(20);
            let sort = flag("--sort");
            let value = engine
                .run(
                    "library.page",
                    &serde_json::json!({ "offset": offset, "limit": limit, "sort": sort }),
                )
                .map_err(|error| error.to_string())?;
            println!("{value}");
            Ok(())
        }
        "memories" => print_memories(&mut engine, rest),
        "shortcuts" => {
            let value = engine.run("app.shortcuts", &Value::Null).map_err(|error| error.to_string())?;
            println!("{} bound command(s)", value["count"]);
            for binding in value["bindings"].as_array().into_iter().flatten() {
                println!(
                    "  {:14} {:10} {}",
                    binding["keys"].as_str().unwrap_or("-"),
                    binding["group"].as_str().unwrap_or("-"),
                    binding["label"].as_str().unwrap_or("-")
                );
            }
            Ok(())
        }
        "registry" => {
            print_registry(&engine);
            Ok(())
        }
        "alias" => link_alias(&mut engine, rest),
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
