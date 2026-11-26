//! `sortlens-cli`: the library from a terminal.
//!
//! Same engine, same catalog, same commands as the window. This is the fastest way to check that
//! a change to the catalog survived a restart, and the base for the MCP tools later.

#![forbid(unsafe_code)]

use serde_json::Value;
use sortlens_core::Flag;
use sortlens_engine::{Engine, Filter, Sort, TagSource};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
sortlens-cli [--library DIR] <command> [--json]
sortlens-cli --version | --help
sortlens-cli --json schemas        every verb with a JSON schema, and its contract

Commands:
  add <folder>              index a folder
  rescan <folder>           re-index an existing root
  roots                     list roots with photo counts
  list [--limit N] [--tag KW] [--source SRC]
                            list photos (default 20); filter by keyword, or by where a
                            keyword came from (manual, ai, xmp, unrecorded)
  page [--offset N] [--limit N] [--sort KEY] [--tag KW] [--source SRC]
                            one page of photos as JSON, for scripts
  stats                     library counters
  insights                  counters, cameras, lenses, keywords and years
  rate <id[,id...]> <0-5>   set stars
  flag <id> pick|reject|none
  label <id[,id...]> <colour|none>
                            set the colour label
  date <id> <YYYY-MM-DD|->  set (or clear, with -) the capture date
  tag <id> <keyword>        add a keyword
  untag <id> <keyword>      remove a keyword
  suggest <id>              what the file's own EXIF suggests
  accept <id> <keyword>     accept one of those suggestions
  dismiss <id[,id...]> <keyword>
                            stop proposing a keyword for these photos; one undo brings it back
  packs                     keyword packs found in the library
  pack <name> on|off        switch a pack on or off
  batchsuggest             propose keywords for the whole library (writes nothing)
  sharpen                  measure how sharp every photo is; unscored photos only
  delete <id[,id...]>      delete photos after the same safety checks a cull batch gets
                            --mode trash|tag|move [--target F]
  keywords                  list keywords with photo counts
  rename <old> <new>        rename a keyword everywhere, merging if the name exists
  mcp                       serve the command registry over MCP on stdio, then exit
  reveal <id>               show the file in the desktop's file manager
  open <id>                 hand the file to the desktop's default application
  mark <folder>             remember a folder's favourite, colour, sort and search
                            [--favourite on|off] [--colour NAME|none] [--sort KEY] [--search TEXT]
  marks                     every remembered folder
  memories [--today D]      resurfacing cards; D is YYYY-MM-DD, default today
  shortcuts                 every command that has a keyboard shortcut
  registry                  keywords, the names they swallowed, and orphaned aliases
  alias <name> [canonical]  link a keyword to another, or forget the link
  folders [--root R] [--in PREFIX]
                            list subfolders with recursive counts
  check                     verify every indexed file is still on disk
  changed                   list files whose size or time no longer matches the index
  refresh                   re-read exactly the changed files
  relocate                  find missing files elsewhere under their root
  retry [--all]             re-read metadata for photos that failed, or for all without EXIF
  models                    the pinned models, with licence, size and install state
  model-consent on|off      the opt-in switch for downloading models (network)
  model-install <id>        download a pinned model; resumable, digest-checked
  model-remove <id>         delete an installed model from the store
  update                    the running version and the last check's answer
  update-consent on|off     the opt-in switch for update checks (network)
  update-check              ask the project's GitHub releases for a newer Sortlens
  undo                      undo the last change
  remap <old> <new>         repoint a moved or renamed root, sample checked
  history [--limit N]       recent changes, newest first
  move <abs-dir> <id...>    move photos into a folder, keeping their review state;
                            the destination is absolute, and a relative one is refused
  moves                     list recent moves with their ids
  unmove <id>               put a moved batch back where it came from
  export <dest> [ids...]    write picks out; no ids means the whole library
                            --format original|jpeg|png --max N --quality N
                            --collision skip|rename|overwrite --no-metadata
  audit                     check the catalog files read back cleanly
  compact                   snapshot and empty the log
  jobs                      the long-running work from this run, with its progress
  diagnostics [--out F]     a report to attach to a bug report; absolute paths are redacted
  backup <file>             write a verified backup of the index
  restore <file> <folder>   verify a backup and unpack it into a new folder
  report                    support report

--json prints the engine's own answer instead of prose for every verb that answers one object,
  with the same flags its plain form takes. `--json schemas` is that table. A pass with progress
  and a cancel — add, rescan, batchsuggest, sharpen, export, model-install, model-remove,
  update-check — refuses `--json` and names the verbs that do answer.
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
        "label" => {
            let name = rest.get(1).map(String::as_str).unwrap_or("none");
            let label = if name == "none" {
                None
            } else {
                Some(
                    sortlens_core::ColorLabel::ALL
                        .into_iter()
                        .find(|label| label.id() == name)
                        .ok_or_else(|| {
                            let offered = sortlens_core::ColorLabel::ALL
                                .iter()
                                .map(|label| label.id())
                                .collect::<Vec<_>>()
                                .join(", ");
                            format!("{name} is not a colour label; use {offered}, or none")
                        })?,
                )
            };
            let changed = engine.label(&[id], label).map_err(|error| error.to_string())?;
            match (label, changed) {
                (Some(label), 1) => println!("labelled {id} {}", label.id()),
                (None, 1) => println!("{id}: label cleared"),
                _ => println!("{id}: no change"),
            }
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

/// Propose keywords for the whole library. Prints the proposals; writes none of them.
/// Delete a chosen set, through the same executor a cull batch uses.
///
/// `--mode tag` is the safe demonstration: it flags the photos as rejects and moves nothing, which is what the
/// golden harness uses so that a test never drops a file into the machine's real Trash.
fn delete_photos(engine: &mut Engine, rest: &[String]) -> Result<(), String> {
    let ids = parse_ids(rest.first().map(String::as_str).unwrap_or_default())?;
    let mode = match text_value(rest, "--mode").unwrap_or_else(|| "trash".to_string()).as_str() {
        "trash" => sortlens_cull::trash::Mode::Trash,
        "tag" => sortlens_cull::trash::Mode::TagOnly,
        "move" => {
            let target = text_value(rest, "--target").ok_or("--mode move needs --target")?;
            sortlens_cull::trash::Mode::Move(std::path::PathBuf::from(target))
        }
        other => return Err(format!("unknown mode: {other}")),
    };
    let report = engine.delete_photos(&ids, mode).map_err(|error| error.to_string())?;
    println!("deleted {} photo(s)", report.applied.len());
    for (id, message) in &report.failed {
        println!("  {id}: {message}");
    }
    Ok(())
}

/// Measure sharpness for every photo that has no current score, and say what could not be judged.
///
/// A progress verb, like `export`: it starts a pass, waits for it and reports counts, and it deliberately has no
/// `--json` schema, because a job with progress and a cancel is not one object and dressing it as one would hide
/// the cancel. `--json sharpen` refuses and says so.
fn sharpen(engine: &mut Engine) -> Result<(), String> {
    let queued = engine.begin_sharpness().map_err(|error| error.to_string())?;
    if queued == 0 {
        println!("every photo has a current score");
        return Ok(());
    }
    println!("measuring {queued} photo(s)…");
    let mut report = None;
    for _ in 0..12_000 {
        if let Some(done) = engine.poll_sharpness() {
            report = Some(done);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let Some(report) = report else {
        return Err("the sharpness pass did not finish".into());
    };
    println!("stored {} score(s)", report.measured);
    if report.unjudged > 0 {
        // Named rather than hidden: an unjudged frame is not a blurry one, and the count is how you notice that a
        // folder of night shots was skipped instead of measured.
        println!("{} photo(s) could not be judged and were left unscored", report.unjudged);
    }
    Ok(())
}

/// List the pinned models, install one (resuming where a previous attempt stopped), remove one, or
/// switch the consent for the one network call Sortlens can make.
fn models(engine: &mut Engine, command: &str, rest: &[String]) -> Result<(), String> {
    if command == "models" {
        let rows = engine.models_status().map_err(|error| error.to_string())?;
        if rows.is_empty() {
            println!("no models pinned");
        }
        for row in rows {
            let state = if row.installed { "on " } else { "off" };
            println!("{state}  {} · {}", row.label, row.summary);
        }
        return Ok(());
    }
    if command == "model-consent" {
        let on = match rest.first().map(String::as_str) {
            Some("on") => true,
            Some("off") => false,
            _ => return Err("say on or off".into()),
        };
        engine.set_models_consent(on);
        println!(
            "model downloads {}",
            if on {
                "allowed — nothing downloads until you install one"
            } else {
                "switched off"
            }
        );
        return Ok(());
    }
    let id = rest.first().ok_or("a model id is required (see models)")?;
    if command == "model-remove" {
        let reclaimed = engine.remove_model(id).map_err(|error| error.to_string())?;
        if reclaimed == 0 {
            println!("{id}: nothing installed");
        } else {
            println!("{id}: removed · {} reclaimed", sortlens_core::format_bytes(reclaimed));
        }
        return Ok(());
    }
    // model-install: a progress verb, like sharpen — it starts the download, waits and reports.
    engine.install_model(id).map_err(|error| error.to_string())?;
    println!("downloading {id}…");
    let mut last = 0;
    for _ in 0..86_400_000 {
        if let Some((done, total_bytes)) = engine.model_progress()
            && done / (10 * 1024 * 1024) > last / (10 * 1024 * 1024)
        {
            last = done;
            println!(
                "  {} of {}",
                sortlens_core::format_bytes(done),
                sortlens_core::format_bytes(total_bytes)
            );
        }
        if let Some(result) = engine.poll_model_install() {
            return match result {
                Ok(install) => {
                    println!("{} · {}", install.headline(), install.path.display());
                    Ok(())
                }
                Err(message) => Err(message),
            };
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    Err("the download did not finish".into())
}

fn batch_suggest(engine: &mut Engine) -> Result<(), String> {
    let planned = engine.begin_batch_suggest().map_err(|error| error.to_string())?;
    println!("looking at {planned} photo(s)…");
    let mut found = None;
    for _ in 0..6000 {
        if let Some(count) = engine.poll_batch() {
            found = Some(count);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let Some(count) = found else {
        return Err("the pass did not finish".into());
    };
    println!("{count} photo(s) have something to suggest; nothing was written");
    for (id, suggestions) in engine.batch_results() {
        for suggestion in suggestions {
            println!(
                "{:>6}  {:>5.2}  {:<10}  {}",
                id.0,
                suggestion.score,
                suggestion.source.label(),
                suggestion.tag
            );
        }
    }
    Ok(())
}

/// Update status, the consent switch, or one check (SL-I10). The check is a progress verb: it
/// starts on a worker and waits for the answer, like model-install.
fn updates(engine: &mut Engine, command: &str, rest: &[String]) -> Result<(), String> {
    if command == "update" {
        let status = engine
            .run("update.status", &serde_json::json!({}))
            .map_err(|error| error.to_string())?;
        println!(
            "Sortlens {} · repo {} · update checks {}",
            status["current"].as_str().unwrap_or("?"),
            status["repo"].as_str().unwrap_or("?"),
            if status["consent"].as_bool().unwrap_or(false) {
                "allowed"
            } else {
                "switched off"
            }
        );
        if let Some(last) = status["last"].as_object() {
            println!("last check: {}", last["headline"].as_str().unwrap_or("?"));
        }
        return Ok(());
    }
    if command == "update-consent" {
        let on = match rest.first().map(String::as_str) {
            Some("on") => true,
            Some("off") => false,
            _ => return Err("say on or off".into()),
        };
        engine.set_update_consent(on);
        println!(
            "update checks {}",
            if on {
                "allowed — nothing is fetched until you run one"
            } else {
                "switched off"
            }
        );
        return Ok(());
    }
    // update-check
    engine.check_for_updates().map_err(|error| error.to_string())?;
    println!("checking GitHub releases…");
    for _ in 0..600 {
        if let Some(answer) = engine.poll_update_check() {
            return match answer {
                Ok(report) => {
                    println!("{}", report.headline());
                    Ok(())
                }
                Err(message) => Err(message),
            };
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    Err("the check did not finish".into())
}

/// List the keyword packs in the library, or switch one on or off.
fn packs(engine: &mut Engine, command: &str, rest: &[String]) -> Result<(), String> {
    if command == "packs" {
        let found = engine.packs();
        if found.is_empty() {
            println!("no packs installed");
        }
        for pack in found {
            let state = if pack.enabled { "on " } else { "off" };
            println!("{state}  {}", pack.line());
        }
        return Ok(());
    }
    let name = rest.first().ok_or("a pack name is required")?;
    let on = match rest.get(1).map(String::as_str) {
        Some("on") => true,
        Some("off") => false,
        _ => return Err("say on or off".into()),
    };
    let changed = engine.set_pack(name, on).map_err(|error| error.to_string())?;
    let verb = if changed { "switched" } else { "already" };
    println!("{name}: {verb} {}", if on { "on" } else { "off" });
    Ok(())
}

/// List what a photo's own metadata suggests, or accept one of those suggestions.
fn suggestions(engine: &mut Engine, command: &str, rest: &[String]) -> Result<(), String> {
    let id = parse_id(rest.first())?;
    if command == "suggest" {
        let offered = engine.suggestions(id);
        if offered.is_empty() {
            println!("{id}: nothing in the file to suggest");
        }
        for suggestion in offered {
            println!("{:>5.2}  {:<16}  {}", suggestion.score, suggestion.source.label(), suggestion.tag);
        }
        return Ok(());
    }
    let keyword = rest.get(1).ok_or("a keyword is required")?;
    if command == "dismiss" {
        // `dismiss <id[,id...]> <keyword>`: every id on one line, so a batch is one line and one undo step.
        let ids = parse_ids(&rest[0])?;
        let recorded = engine
            .dismiss_suggestions(&ids, std::slice::from_ref(keyword))
            .map_err(|error| error.to_string())?;
        let verb = if recorded > 0 { "stops suggesting" } else { "already dismissed" };
        println!("{verb} {keyword} on {recorded} photo(s)");
        return Ok(());
    }
    let changed = engine.accept_suggestion(id, keyword).map_err(|error| error.to_string())?;
    let verb = if changed { "tagged" } else { "already tagged" };
    println!("{id}: {verb} {keyword}");
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

/// Whether a schema's verb changes the library or only reports on it.
///
/// Written down rather than inferred, because a script that pipes `--json` into a decision needs to know
/// whether running it twice is safe, and "it looks like a read" is not a contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Reports; writes nothing.
    Read,
    /// Changes the library, or writes an artefact outside it. Running it twice is not the same as running it once.
    Write,
}

/// One published JSON schema: the CLI verb, the command that answers it, and what a script may rely on.
struct Schema {
    verb: &'static str,
    command: &'static str,
    kind: Kind,
    /// The compatibility promise for this object, in one sentence.
    ///
    /// The general promise lives in `docs/CLI.md`; this says what is *specific* to the verb — which field is
    /// the answer, what a count means, and what may be added later.
    note: &'static str,
}

/// Every CLI verb that answers `--json`, and the engine command behind it.
///
/// A table rather than a guess, because "some verbs have a schema and some do not" is exactly the kind of
/// thing a script must be able to find out before it relies on it: a verb that is absent here refuses and
/// names the ones that are present, and `--JSON --schemas` prints this table for a machine to read.
const SCHEMAS: &[Schema] = &[
    Schema {
        verb: "roots",
        command: "library.roots",
        kind: Kind::Read,
        note: "`roots`: an array of `{root, photos}`; more keys may be added to an entry",
    },
    Schema {
        verb: "keywords",
        command: "tag.keywords",
        kind: Kind::Read,
        note: "keyword rows with counts; `count` is photos, never edges",
    },
    Schema {
        verb: "stats",
        command: "library.stats",
        kind: Kind::Read,
        note: "library counters; `total` is every indexed photo, including missing ones",
    },
    Schema {
        verb: "insights",
        command: "view.insights",
        kind: Kind::Read,
        note: "the Insights destination as one object: counters, cameras, lenses, keywords, years and months",
    },
    Schema {
        verb: "list",
        command: "library.page",
        kind: Kind::Read,
        note: "a page of photos with the ids that `rate` and `flag` accept; `total` is the filtered total",
    },
    Schema {
        verb: "page",
        command: "library.page",
        kind: Kind::Read,
        note: "the same page as `list`, with `--offset`; `sort` echoes the order the page was taken in",
    },
    Schema {
        verb: "check",
        command: "library.health",
        kind: Kind::Read,
        note: "missing files and unreadable ones, with the reason the catalog recorded",
    },
    Schema {
        verb: "moves",
        command: "edit.moves",
        kind: Kind::Read,
        note: "move receipts, newest last; `count` is files, not bytes",
    },
    Schema {
        verb: "move",
        command: "edit.movePicks",
        kind: Kind::Write,
        note: "`moved` is files that left their folder; one changed since it was indexed refuses the whole batch, and the destination is absolute",
    },
    Schema {
        verb: "unmove",
        command: "edit.undoMove",
        kind: Kind::Write,
        note: "`restored` is files put back where they came from; an unknown move id is a refusal, not 0",
    },
    Schema {
        verb: "changed",
        command: "library.changed",
        kind: Kind::Read,
        note: "files whose size or modification time disagrees with the catalog",
    },
    Schema {
        verb: "refresh",
        command: "library.refreshChanged",
        kind: Kind::Write,
        note: "`refreshed` is files re-read because their size or time moved; an unchanged library reports 0",
    },
    Schema {
        verb: "history",
        command: "library.history",
        kind: Kind::Read,
        note: "recent ops, newest first; the shape of a change is the op that made it",
    },
    Schema {
        verb: "audit",
        command: "library.audit",
        kind: Kind::Read,
        note: "the replay audit; `clean` is the verdict, the rest is evidence",
    },
    Schema {
        verb: "compact",
        command: "library.compact",
        kind: Kind::Write,
        note: "`reclaimed` is bytes of log the snapshot made unnecessary, not photos; a second run reports 0",
    },
    Schema {
        verb: "backup",
        command: "library.backup",
        kind: Kind::Write,
        note: "`files` is how many files the archive at the given path holds; the library itself is not changed",
    },
    Schema {
        verb: "restore",
        command: "library.restoreBackup",
        kind: Kind::Write,
        note: "`files` is how many were unpacked into the new folder; nothing is restored over an existing library",
    },
    Schema {
        verb: "memories",
        command: "memories.list",
        kind: Kind::Read,
        note: "resurfacing cards for one day; `--today YYYY-MM-DD` fixes the date, absent means today",
    },
    Schema {
        verb: "shortcuts",
        command: "app.shortcuts",
        kind: Kind::Read,
        note: "bound commands: key, context, label and command id",
    },
    Schema {
        verb: "registry",
        command: "tag.registry",
        kind: Kind::Read,
        note: "keywords, the names they swallowed, and orphaned aliases",
    },
    Schema {
        verb: "packs",
        command: "packs.list",
        kind: Kind::Read,
        note: "keyword packs with their rules and whether each is on",
    },
    Schema {
        verb: "jobs",
        command: "library.jobs",
        kind: Kind::Read,
        note: "long-running work with its progress; a finished job keeps its last report",
    },
    Schema {
        verb: "models",
        command: "model.status",
        kind: Kind::Read,
        note: "every pinned model with its licence, size and digest-checked install state; `consent` says whether downloads are allowed",
    },
    Schema {
        verb: "model-consent",
        command: "model.consent",
        kind: Kind::Write,
        note: "`consent` is the switch's new state and `changed` whether it moved; the CLI keeps it for this run only",
    },
    Schema {
        verb: "update",
        command: "update.status",
        kind: Kind::Read,
        note: "the running version, the consent switch, and the last check's answer with its release notes headline",
    },
    Schema {
        verb: "update-consent",
        command: "update.consent",
        kind: Kind::Write,
        note: "`consent` is the switch's new state and `changed` whether it moved; the CLI keeps it for this run only",
    },
    Schema {
        verb: "report",
        command: "app.supportReport",
        kind: Kind::Read,
        note: "versions, command counts, roots and warnings for a bug report; `report` prints the same object without the flag",
    },
    Schema {
        verb: "suggest",
        command: "suggest.list",
        kind: Kind::Read,
        note: "proposals for one photo: keyword, source, confidence. Nothing is written until `accept`",
    },
    Schema {
        verb: "folders",
        command: "library.subfolders",
        kind: Kind::Read,
        note: "subfolders under a root or prefix, with recursive counts",
    },
    Schema {
        verb: "mark",
        command: "folder.mark",
        kind: Kind::Write,
        note: "`changed` is whether the mark moved; `key` is the path identity it was stored under; omitted fields keep their value",
    },
    Schema {
        verb: "marks",
        command: "folder.marks",
        kind: Kind::Read,
        note: "`folders`: every marked folder as {key, favourite, colour, sort, search}, keyed by path identity",
    },
    Schema {
        verb: "rate",
        command: "photo.rate",
        kind: Kind::Write,
        note: "`changed` is how many photos took the new rating; a photo that already had it is not counted",
    },
    Schema {
        verb: "flag",
        command: "photo.flag",
        kind: Kind::Write,
        note: "`changed` is photos written, and a second identical run reports 0",
    },
    Schema {
        verb: "label",
        command: "photo.label",
        kind: Kind::Write,
        note: "`changed` is photos written; the request key is `label`, and null clears it",
    },
    Schema {
        verb: "tag",
        command: "photo.keyword",
        kind: Kind::Write,
        note: "`changed` is photos that gained the keyword; adding it twice reports 0",
    },
    Schema {
        verb: "untag",
        command: "photo.keyword",
        kind: Kind::Write,
        note: "the same object as `tag` with `add: false`; `changed` is photos that lost it",
    },
    Schema {
        verb: "date",
        command: "photo.setCaptured",
        kind: Kind::Write,
        note: "`changed` is photos whose capture date moved; the request is a list of `{id, captured}`",
    },
    Schema {
        verb: "rename",
        command: "tag.rename",
        kind: Kind::Write,
        note: "`changed` is photos moved to the new name; a merge into an existing keyword counts them too",
    },
    Schema {
        verb: "alias",
        command: "tag.alias",
        kind: Kind::Write,
        note: "`changed` is 1 when the link was stored or forgotten, 0 when it already said that",
    },
    Schema {
        verb: "accept",
        command: "suggest.accept",
        kind: Kind::Write,
        note: "`tagged` is 1 when the keyword was added, 0 when the photo already had it",
    },
    Schema {
        verb: "dismiss",
        command: "suggest.dismiss",
        kind: Kind::Write,
        note: "`dismissed` is how many photos newly stopped being offered that keyword; 0 when it already had",
    },
    Schema {
        verb: "pack",
        command: "packs.set",
        kind: Kind::Write,
        note: "`changed` is 1 when the switch moved, 0 when the pack was already in that state",
    },
    Schema {
        verb: "delete",
        command: "photo.delete",
        kind: Kind::Write,
        note: "one audited batch: `applied` is how many left their folder, `failed` how many were refused",
    },
    Schema {
        verb: "undo",
        command: "edit.undo",
        kind: Kind::Write,
        note: "`undone` is whether there was anything to undo; false is not an error",
    },
    Schema {
        verb: "retry",
        command: "library.retryMetadata",
        kind: Kind::Write,
        note: "`updated` and `remaining`; a photo that still has no EXIF is counted in `remaining`",
    },
    Schema {
        verb: "relocate",
        command: "library.relocate",
        kind: Kind::Read,
        note: "`relocated` is files whose new location was found; nothing is moved by looking",
    },
    Schema {
        verb: "remap",
        command: "library.remapRoot",
        kind: Kind::Write,
        note: "`remapped` is photos repointed at the new root after the sample check passed",
    },
    Schema {
        verb: "schemas",
        command: "",
        kind: Kind::Read,
        note: "this table: verb, command, kind and note, for a script discovering the contract",
    },
];

/// The ids in `1,2,3` form, so a script can rate a page of photos in one call.
fn parse_ids(text: &str) -> Result<Vec<sortlens_core::PhotoId>, String> {
    text.split(',')
        .map(|part| {
            part.trim()
                .parse::<u64>()
                .map(sortlens_core::PhotoId)
                .map_err(|_| format!("photo ids must be numbers: {part}"))
        })
        .collect()
}

/// Print the schema table, so a script can discover the contract instead of reading this file.
fn print_schemas() -> Result<(), String> {
    let rows: Vec<serde_json::Value> = SCHEMAS
        .iter()
        .map(|schema| {
            serde_json::json!({
                "command": schema.command,
                "kind": match schema.kind {
                    Kind::Read => "read",
                    Kind::Write => "write",
                },
                "note": schema.note,
                "verb": schema.verb,
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::Value::Array(rows)).map_err(|error| error.to_string())?
    );
    Ok(())
}

/// Print the engine's own answer for a verb that has a schema.
fn json_verb(engine: &mut Engine, verb: &str, rest: &[String]) -> Result<(), String> {
    if verb == "schemas" {
        return print_schemas();
    }
    let Some(schema) = SCHEMAS.iter().find(|schema| schema.verb == verb) else {
        let offered = SCHEMAS.iter().map(|schema| schema.verb).collect::<Vec<&str>>().join(", ");
        return Err(format!("{verb} has no JSON schema yet; --json works for: {offered}"));
    };
    let ids = || -> Result<Vec<u64>, String> {
        let text = rest.first().ok_or("photo ids are required (1 or 1,2,3)")?;
        Ok(parse_ids(text)?.iter().map(|id| id.0).collect())
    };
    let args = match schema.verb {
        "list" | "page" => {
            let limit = flag_value(rest, "--limit").unwrap_or(20);
            // The plain form honours `--tag` and `--source`, so this one must too: a JSON verb that
            // took a flag and dropped it would answer a different question than the one that was asked.
            let mut filter = serde_json::json!({});
            if let Some(tag) = text_value(rest, "--tag") {
                filter["tags"] = serde_json::json!([tag]);
            }
            if let Some(source) = source_flag(rest)? {
                filter["tag_source"] = serde_json::Value::String(source.id().to_string());
            }
            let mut args = serde_json::json!({
                "offset": flag_value(rest, "--offset").unwrap_or(0),
                "limit": limit,
                "filter": filter,
            });
            if let Some(sort) = text_value(rest, "--sort") {
                args["sort"] = serde_json::Value::String(sort);
            }
            args
        }
        "history" => serde_json::json!({ "limit": flag_value(rest, "--limit").unwrap_or(20) }),
        "memories" => match text_value(rest, "--today") {
            Some(day) => serde_json::json!({ "today": parse_day(&day)? }),
            None => serde_json::json!({}),
        },
        "folders" => {
            let mut args = serde_json::json!({});
            if let Some(root) = text_value(rest, "--root") {
                args["root"] = serde_json::Value::String(root);
            }
            if let Some(prefix) = text_value(rest, "--in") {
                args["prefix"] = serde_json::Value::String(prefix);
            }
            args
        }
        "suggest" => serde_json::json!({ "ids": [parse_id(rest.first())?.0] }),
        "mark" => {
            let path = rest.first().ok_or("a folder path is required")?;
            let mut args = serde_json::json!({ "path": path });
            if let Some(state) = text_value(rest, "--favourite") {
                args["favourite"] = serde_json::Value::Bool(matches!(state.as_str(), "on" | "true" | "yes"));
            }
            if let Some(colour) = text_value(rest, "--colour") {
                args["colour"] = if colour == "none" {
                    serde_json::Value::Null
                } else {
                    serde_json::Value::String(colour)
                };
            }
            if let Some(sort) = text_value(rest, "--sort") {
                args["sort"] = serde_json::Value::String(sort);
            }
            if let Some(search) = text_value(rest, "--search") {
                args["search"] = serde_json::Value::String(search);
            }
            args
        }
        "rate" => {
            let stars: u8 = rest.get(1).ok_or("stars are required")?.parse().map_err(|_| "stars must be 0-5")?;
            serde_json::json!({ "ids": ids()?, "stars": stars })
        }
        "flag" => {
            let flag = match rest.get(1).map(String::as_str).unwrap_or("none") {
                "pick" => "pick",
                "reject" => "reject",
                "none" => "none",
                other => return Err(format!("unknown flag: {other}")),
            };
            serde_json::json!({ "ids": ids()?, "flag": flag })
        }
        "label" => {
            let colour = match rest.get(1).map(String::as_str).unwrap_or("none") {
                "none" => serde_json::Value::Null,
                other => serde_json::Value::String(other.to_string()),
            };
            serde_json::json!({ "ids": ids()?, "label": colour })
        }
        "tag" | "untag" => {
            let tag = rest.get(1).ok_or("a keyword is required")?;
            serde_json::json!({ "ids": ids()?, "tag": tag, "add": schema.verb == "tag" })
        }
        "date" => {
            let text = rest.get(1).ok_or("a date or - is required")?;
            let captured = if text == "-" {
                serde_json::Value::Null
            } else {
                serde_json::json!(parse_day(text)?)
            };
            let ids = ids()?;
            let changes: Vec<serde_json::Value> = ids.iter().map(|id| serde_json::json!({ "id": id, "captured": captured })).collect();
            serde_json::json!({ "changes": changes })
        }
        "rename" => {
            let from = rest.first().ok_or("the old name is required")?;
            let to = rest.get(1).ok_or("the new name is required")?;
            serde_json::json!({ "from": from, "to": to })
        }
        "alias" => {
            let name = rest.first().ok_or("a keyword is required")?;
            let canonical = match rest.get(1) {
                Some(canonical) => serde_json::Value::String(canonical.clone()),
                None => serde_json::Value::Null,
            };
            serde_json::json!({ "alias": name, "canonical": canonical })
        }
        "accept" => {
            let id = parse_id(rest.first())?;
            let tag = rest.get(1).ok_or("a keyword is required")?;
            serde_json::json!({ "id": id.0, "tag": tag })
        }
        "dismiss" => {
            let tag = rest.get(1).ok_or("a keyword is required")?;
            serde_json::json!({ "ids": ids()?, "tag": tag })
        }
        "pack" => {
            let name = rest.first().ok_or("a pack name is required")?;
            let on = match rest.get(1).map(String::as_str).unwrap_or("on") {
                "on" => true,
                "off" => false,
                other => return Err(format!("expected on or off, not {other}")),
            };
            serde_json::json!({ "name": name, "on": on })
        }
        "retry" => serde_json::json!({ "all": rest.iter().any(|arg| arg == "--all") }),
        "delete" => {
            let ids = parse_ids(rest.first().map(String::as_str).unwrap_or_default())?;
            let ids: Vec<u64> = ids.iter().map(|id| id.0).collect();
            let mut args = serde_json::json!({ "ids": ids, "mode": text_value(rest, "--mode").unwrap_or_else(|| "trash".into()) });
            if let Some(target) = text_value(rest, "--target") {
                args["target"] = serde_json::Value::String(target);
            }
            args
        }
        "remap" => {
            let from = rest.first().ok_or("the old root is required")?;
            let to = rest.get(1).ok_or("the new root is required")?;
            serde_json::json!({ "from": from, "to": to })
        }
        "backup" => {
            let path = rest.first().ok_or("a destination file is required")?;
            serde_json::json!({ "path": path })
        }
        "restore" => {
            let path = rest.first().ok_or("a backup file is required")?;
            let dest = rest.get(1).ok_or("a destination folder is required")?;
            serde_json::json!({ "path": path, "dest": dest })
        }
        "move" => {
            let dest = rest.first().ok_or("a destination folder is required")?;
            let ids = parse_ids(&rest.get(1..).unwrap_or_default().join(","))?;
            serde_json::json!({ "dest": dest, "ids": ids.iter().map(|id| id.0).collect::<Vec<u64>>() })
        }
        "unmove" => {
            let id = parse_id(rest.first())?;
            serde_json::json!({ "id": id.0 })
        }
        "model-consent" | "update-consent" => {
            let on = match rest.first().map(String::as_str).unwrap_or("on") {
                "on" => true,
                "off" => false,
                other => return Err(format!("expected on or off, not {other}")),
            };
            serde_json::json!({ "on": on })
        }
        _ => serde_json::json!({}),
    };
    let value = engine.run(schema.command, &args).map_err(|error| error.to_string())?;
    println!("{}", serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?);
    Ok(())
}

/// Remember a folder's view (favourite, colour, sort, search), or list what is remembered.
///
/// The same flags `--json mark` reads: a folder's mark is one object, so the plain form prints the
/// object's facts on one line rather than a table a person has to line up by eye.
fn folder_marks(engine: &mut Engine, command: &str, rest: &[String]) -> Result<(), String> {
    if command == "marks" {
        let value = engine
            .run("folder.marks", &serde_json::json!({}))
            .map_err(|error| error.to_string())?;
        let rows = value["folders"].as_array().map(Vec::as_slice).unwrap_or_default();
        if rows.is_empty() {
            println!("no marked folders");
        }
        for row in rows {
            println!(
                "{}  {}{}",
                row["key"].as_str().unwrap_or("-"),
                if row["favourite"].as_bool().unwrap_or(false) {
                    "favourite"
                } else {
                    "-"
                },
                row["colour"].as_str().map(|colour| format!("  {colour}")).unwrap_or_default()
            );
        }
        return Ok(());
    }
    let path = rest.first().ok_or("a folder path is required")?;
    let mut args = serde_json::json!({ "path": path });
    if let Some(state) = text_value(rest, "--favourite") {
        args["favourite"] = serde_json::Value::Bool(matches!(state.as_str(), "on" | "true" | "yes"));
    }
    if let Some(colour) = text_value(rest, "--colour") {
        args["colour"] = if colour == "none" {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(colour)
        };
    }
    if let Some(sort) = text_value(rest, "--sort") {
        args["sort"] = serde_json::Value::String(sort);
    }
    if let Some(search) = text_value(rest, "--search") {
        args["search"] = serde_json::Value::String(search);
    }
    let value = engine.run("folder.mark", &args).map_err(|error| error.to_string())?;
    let mut line = format!("{path}: ");
    line.push_str(if value["favourite"].as_bool().unwrap_or(false) {
        "favourite"
    } else {
        "not favourite"
    });
    line.push_str(&format!(", colour {}", value["colour"].as_str().unwrap_or("none")));
    if let Some(sort) = value["sort"].as_str() {
        line.push_str(&format!(", sorted by {sort}"));
    }
    if let Some(search) = value["search"].as_str() {
        line.push_str(&format!(", search \"{search}\""));
    }
    if !value["changed"].as_bool().unwrap_or(false) {
        line.push_str(" (unchanged)");
    }
    println!("{line}");
    Ok(())
}

/// The long-running work this run started, with its progress.
///
/// A pass that outlives one command reports through `jobs`; the rows are the same object
/// `--json jobs` answers, so a person and a script read the same facts.
fn jobs(engine: &mut Engine) -> Result<(), String> {
    let value = engine
        .run("library.jobs", &serde_json::json!({}))
        .map_err(|error| error.to_string())?;
    let rows = value["jobs"].as_array().map(Vec::as_slice).unwrap_or_default();
    if rows.is_empty() {
        println!("no jobs");
    }
    for job in rows {
        println!(
            "#{:<4} {:<9} {:<14} {}",
            job["id"],
            job["status"].as_str().unwrap_or("-"),
            job["kind"].as_str().unwrap_or("-"),
            job["detail"].as_str().unwrap_or("")
        );
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // `--json` may sit anywhere: before the verb, after it, or between its arguments.
    let json = args.iter().any(|arg| arg == "--json");
    args.retain(|arg| arg != "--json");
    let mut library = library_dir();
    if let Some(index) = args.iter().position(|arg| arg == "--library") {
        if index + 1 >= args.len() {
            return Err("--library needs a folder".into());
        }
        library = PathBuf::from(args.remove(index + 1));
        args.remove(index);
    }
    // `--version` and `--help` answer before a library is opened: a version question must not create
    // one, and must work on a machine that has never run the app.
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("sortlens-cli {}", sortlens_core::VERSION);
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print!("{USAGE}");
        return Ok(());
    }
    let Some(command) = args.first().cloned() else {
        print!("{USAGE}");
        return Ok(());
    };
    let rest = &args[1..];

    let mut engine = Engine::open(&library).map_err(|error| error.to_string())?;
    if json {
        return json_verb(&mut engine, &command, rest);
    }
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
            report_scan(&done);
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
                tag_source: source_flag(rest)?,
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
                println!("{:<18} {}", highlight.label().to_lowercase(), highlight.value());
            }
            Ok(())
        }
        // The photo-field verbs are all "read the id, read the value, write it" — one helper, so the
        // command table stays a table.
        "rate" | "flag" | "label" | "date" | "tag" | "untag" => edit_photo(&mut engine, &command, rest),
        "suggest" | "accept" | "dismiss" => suggestions(&mut engine, &command, rest),
        "packs" | "pack" => packs(&mut engine, &command, rest),
        "models" | "model-consent" | "model-install" | "model-remove" => models(&mut engine, &command, rest),
        "update" | "update-consent" | "update-check" => updates(&mut engine, &command, rest),
        "batchsuggest" => batch_suggest(&mut engine),
        "delete" => delete_photos(&mut engine, rest),
        "sharpen" => sharpen(&mut engine),
        "diagnostics" => {
            let text = engine.diagnostics().map_err(|error| error.to_string())?;
            match text_value(rest, "--out") {
                Some(path) => {
                    std::fs::write(&path, text.as_bytes()).map_err(|error| format!("{path}: {error}"))?;
                    println!("wrote {} line(s) to {path}", text.lines().count());
                }
                None => print!("{text}"),
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
        "mark" | "marks" => folder_marks(&mut engine, &command, rest),
        "jobs" => jobs(&mut engine),
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
        // `page` is the plain-form spelling of the same answer `--json page` gives: one mapper, so the
        // two cannot drift on which flags they read.
        "page" => json_verb(&mut engine, "page", rest),
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

/// What a scan wants to say beyond its one-line summary: a correction the user can act on, and the files
/// that could not be read at all.
fn report_scan(done: &sortlens_engine::ScanFinished) {
    if done.mislabelled > 0 {
        println!("  {} routed by content, not name", done.mislabelled);
    }
    for error in &done.errors {
        eprintln!("  unreadable: {error}");
    }
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

/// The `--source` flag, as the same enum the log writes.
///
/// A word the enum does not know is refused with the list of real ones: silently treating a typo as "no filter"
/// would answer a different question than the one that was asked, which is the class of bug a filter must not have.
fn source_flag(args: &[String]) -> Result<Option<TagSource>, String> {
    let Some(text) = text_value(args, "--source") else {
        return Ok(None);
    };
    TagSource::parse(&text).map(Some).ok_or_else(|| {
        format!(
            "--source {text} is not a source; use {}",
            TagSource::ALL
                .into_iter()
                .chain([TagSource::Unrecorded])
                .map(TagSource::id)
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
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
