# Behaviour contracts

Normative rules that outlive any one milestone. Code that contradicts this document is a bug in
the code, or a decision that has to be written down here first.

Scope: the parts of the product where "looks right" is not enough — what happens to the user's
files, what a rating means, and what the application promises after a crash.

---

## 1. Ratings, flags and labels

A photo has three independent annotations:

| Annotation | Domain | Empty value | XMP |
|---|---|---|---|
| Rating | 0–5 stars | 0 | `xmp:Rating` |
| Flag | pick, reject, none | none | `xmp:Label` is **not** used for flags |
| Colour label | red, yellow, green, blue, purple, none | none | `xmp:Label` |

Rules:

1. A rating of 0 means unrated, not "zero stars". Setting a photo to the rating it already has is
   a no-op and writes nothing.
2. Flag and colour label are orthogonal. A rejected photo can carry five stars; the cull surface
   still shows it as a reject.
3. Multi-selection applies a rating to every selected photo, and the pill shows the common value
   or a dash when the selection disagrees. Clicking a dash sets the value for all of them.
4. Every annotation change increments the catalog revision and appends one op. There is no
   "annotation" table to get out of sync.

## 2. The cull state machine

One photo, one state. The states are a partition: no photo is in two of them.

```
                ┌──────────┐
   import ─────▶│ unjudged │◀───────────── clear flag / undo
                └────┬─────┘
          pick │     │ reject            │ maybe later
               ▼     ▼                   ▼
          ┌────────┐ ┌────────┐    ┌──────────┐
          │  pick  │ │ reject │    │ deferred │
          └───┬────┘ └───┬────┘    └────┬─────┘
              │          │              │
              └──────────┴──────────────┘
                         │ Apply
                         ▼
                 ┌───────────────┐
                 │ applied_batch │  (op log records the outcome)
                 └───────────────┘
```

Transitions:

| From | To | Trigger |
|---|---|---|
| unjudged | pick | `P` or flag button |
| unjudged | reject | `X` or flag button |
| unjudged | deferred | `Space` (skip without judging) |
| pick / reject / deferred | unjudged | `U` or clicking the active flag again |
| any | same | re-applying the current value (no-op, no op written) |

Session rules:

1. **Nothing moves until Apply.** Rejects are staged in the session, not in the filesystem.
2. Apply is one transaction in the op log: either the whole batch is recorded or none of it is.
3. Undo after Apply restores the previous state of every photo in that batch, and the audit entry
   says who applied it and when.
4. Closing the app with a staged batch asks once, then either applies or discards. It never
   silently drops the batch.

## 3. What Apply does with rejects

`Settings > Library > Delete mode` is one of:

- **trash** (default). Files go to the OS trash (macOS Trash, Windows Recycle Bin, freedesktop
  trash). Reversible outside Sortlens. No confirmation dialog beyond the batch review.
- **move**. Files move to `reject_folder`, preserving their relative subfolder. Refuses to run
  when the folder is inside a watched root (that would make rejects reappear on the next scan) or
  on a different volume without enough free space.
- **tag only**. Nothing is moved. The reject state and sidecar carry the decision; the file stays.

Invariants, checked before the first file is touched:

1. Every path is inside a watched root.
2. Every file still exists, is readable, and its identity (size and modification time) matches the
   catalog entry. A file that changed on disk is reported, not deleted.
3. The target space is writable and has room for the batch.
4. No path in the batch is a symlink that resolves outside its root.

If any check fails, the batch does not start. Partial application is not an outcome.

## 4. XMP sidecars

Sidecars are on by default (`Settings > Library > Write sidecars`).

- Name: `IMG_1234.CR3` → `IMG_1234.xmp` in the same folder. Existing sidecars are read and merged,
  never replaced wholesale.
- Fields written: rating, label, keywords, and the cull decision under the `sortlens` namespace.
- Fields not written: anything Sortlens did not change. An edit made in another application is
  preserved.
- Encoding: UTF-8, `xpacket` wrapper, one `rdf:Description`.
- Writes are atomic: temp file in the same directory, then rename.
- A file with a read-only sidecar is still rated in the catalog; the sidecar failure is reported in
  the activity panel and retried on the next change, not on every frame.
- Sidecars are never deleted when a photo is rejected. The decision travels with the file.
- **Provenance is not written to a sidecar.** A keyword read out of a sidecar is recorded as `XMP`,
  one typed in Sortlens as `Manual`, one accepted from a suggestion as `AI`; the value lives in the
  catalog only. It is therefore not part of the annotation revision (`rev`) that a stale writer is
  refused against — a change of provenance alone can never lose another application's edit, because
  it is not an edit to the file.
- A library written before sources were recorded has `Unrecorded` for those keywords. Sortlens does
  not guess `Manual`: "nobody wrote it down" and "a person typed it" are different facts.
- **A dismissed suggestion is catalogue-only too.** Dismissing a proposal records the keyword in the
  photo's `dismissed_suggestions` (`Op::SuggestionDismissed`, one op for the whole selection, undone by
  one ⌘Z) and nothing in the sidecar: it is a decision about what Sortlens should offer, not a fact about
  the photograph, so another application must not inherit it and it does not touch `rev`. It is stored
  rather than derived because the proposals are derived — a refusal kept only on screen would last until
  the panel was reopened.

## 5. Scan and identity

- A photo's identity is its **content hash of the first and last 64 KiB plus its size**, not its
  path. Moving a file inside a watched root keeps its id, rating and collections.
- A file whose size or modification time changed is re-read; if the content hash matches, the entry
  is updated in place rather than duplicated.
- Two files with the same content hash are duplicates; both are kept until the user acts. Sortlens
  never deletes a duplicate automatically.
- A missing file is marked missing, not removed. Its annotations survive, and restoring the file to
  its old path restores the entry. "Remove missing entries" is an explicit command.
- Unreadable files (permission, I/O error) are counted in the scan report and retried on the next
  scan; they never fail the whole import.

## 6. Previews

- Tiers: 128 px (grid), 384 px (large grid and filmstrip), 1024 px (detail), screen size (loupe).
- A preview is never upscaled. The grid shows the largest tier that fits.
- RAW files use the embedded JPEG preview when present. Demosaicing only happens when the user asks
  for it (the develop path), not while browsing.
- A preview that cannot be decoded shows a placeholder with the file type, not a blank cell, and the
  failure is logged once per file rather than once per frame.
- Cache entries are content-addressed and disposable. Deleting the cache never loses user data.

## 7. Crash and durability

- The op log is append-only and checksummed per record. A torn final record is discarded on open;
  earlier records are kept.
- A snapshot is written when the log exceeds 4 MiB or on clean exit, whichever comes first.
- Opening a catalog whose log is ahead of its snapshot replays the tail. Opening a catalog with a
  corrupt middle record stops at that record, reports it, and opens read-only rather than guessing.
- Settings are flushed on exit and 500 ms after the last change; a crash loses at most the current
  debounce window.

## 8. What sortlens does not do

- It does not upload, sync or teleport a photo unless the user turns on remote access.
- It does not rename files. Ever. Renaming is a file manager's job and it breaks other
  applications' sidecars.
- It does not delete anything on import, including duplicates and files it cannot decode.
- It does not modify a file's pixels outside an explicit export.
- It does not phone home. Update checks are opt-in and are the only network request in a default
  install.

## Memories

"On this day" and its siblings are **derived**, never stored: the same library and the same day
always produce the same cards, so a card can be checked against the grid.

- **Capture dates are read as UTC.** EXIF stores a local wall-clock time with no reliable offset, so
  a photo shot near midnight can land on the neighbouring day. Inventing an offset nobody recorded
  would be worse than saying this out loud.
- **On this day** matches month and day in *earlier* years only, best first (stars, then pick over
  nothing, then pixels, then recency). 29 February falls back to the 28th in common years, so the
  card does not vanish for three years out of four.
- **This week** is the seven days ending today, compared by month and day inside each earlier year,
  which also makes a window that crosses new year wrap instead of going empty.
- **Never looked at** is photos with no stars and no flag, at least a year old, oldest first. Judging
  a photo (rating or flagging) removes it from that card — the point of the card is what was skipped.
- **Cards overlap on purpose.** One photo can be on three cards; the header counts per card, not
  unique photos.
- **A card says what it left out** ("3 of 10 photos · since 2019") rather than silently truncating.
- **A wrong date can be corrected** with `photo.setCaptured`, which is an op like any other: undoable,
  in the change log, and written to the sidecar so the correction survives the next read.

## Showing a file to the desktop

Two actions, `photo.reveal` and `photo.open`, and one rule about who does what.

- **The engine builds the command; the host runs it.** `sortlens_core::reveal` turns a path and a
  desktop into a program and an argument list, with no process table involved, so the exact argv is
  asserted in a test for each platform. The app and the CLI execute it; the engine's command result
  says `"ran": false` and hands back the line, because a library opened by the wasm client has no
  process table to run anything in.
- **Arguments are always a list.** A file called `$(rm -rf ~).jpg` is one argument, never a shell
  string.
- **Reveal selects the file where the desktop can**: macOS `open -R`, Windows `explorer /select,`.
  Linux has no portable way to select a file in its file manager, so reveal opens the *containing
  folder* and the app says so instead of pretending the photo is highlighted.
- **A file that is no longer on disk is refused** with an instruction to rescan or relocate the
  folder, rather than spawning a process that fails silently.
- **Nothing is mutated.** Neither action touches the catalog, the op log or a sidecar: showing a file
  is not an edit, and it must not appear in the change history.
- Windows `explorer` exits non-zero even when it worked, so a non-zero exit is only treated as a
  failure when the program could not be started at all.

## Exporting

An export is a **read of the library and a write somewhere else**. It never moves, edits or deletes an
original, and it writes **no operation** to the log: nothing about the library changed, so nothing is
recorded, and an export cannot be undone because there is nothing to undo.

- **Original** copies the bytes as they are: same extension in the source's case, no re-encode, no
  resize. Asking for a longest edge with this format is refused by the UI rather than silently ignored —
  copying cannot resize.
- **JPEG** decodes, optionally downscales so the *long* edge is the requested size, and re-encodes at
  the requested quality (1–100, validated not clamped). Upscaling never happens. With metadata on, the
  source's APP1 (EXIF) and APP13 (IPTC) blocks are carried into the new file; other application
  segments are not, because forwarding every one of them would carry another program's editing state.
- **Collisions** are decided before anything is written, in the order the photos are given, so two
  files with the same name inside one export collide with each other and not only with the folder:
  `skip` leaves the existing file alone, `keep both` writes `name-2.jpg` upward (up to 1000, then it
  reports rather than loops), `replace` overwrites.
- A photo that is no longer on disk is named and the export refuses, rather than writing part of the
  set quietly.
- Progress is per file and the export can be stopped; files already written stay written, and the
  report says how many of each.

