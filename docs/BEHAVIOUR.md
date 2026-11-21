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
