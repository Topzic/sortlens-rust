# The `sortlens-cli` JSON contract

`--json` turns a command's answer into JSON for a script or an agent. This file is the promise: what a
caller may rely on, what may change without notice, and which verbs have a schema at all.

Run `sortlens-cli --json schemas` for the same table as data. It is generated from the binary's own schema
table, so it cannot drift from what the commands actually do — a test asserts every mutating verb appears in
it with a note that is a sentence rather than a placeholder.

## The promise

1. **Additive only.** A field that is present today will not be renamed, removed, or change its type within a
   major version. New fields may appear anywhere. A script must ignore keys it does not know.
2. **Absent and null are different.** A field that is `null` is known to be absent (a photo with no capture
   date); a field that is not there at all is not part of this schema's promise.
3. **Arrays are arrays.** An empty result is `[]` with a `total` of 0 where a total exists — never `null`, never
   a missing key.
4. **Counts are photos unless the note says otherwise.** `changed`, `total`, `photos` all count photographs.
   Where a count is bytes, files, or edges, the note for that verb says so.
5. **A write reports what it changed.** `changed` counts photos the write actually altered, so running the
   same `rate` twice answers `3` then `0`. A caller can therefore tell a write from a no-op without reading
   the library back.
6. **Exit codes carry the verdict.** `0` means the command answered. A refusal — an unknown id, a file that
   changed since it was indexed, a verb with no schema — exits non-zero and explains itself on stderr. A
   refusal is never JSON: a script must not mistake an error object for a result.
7. **Errors are not fatal to the library.** A refused command leaves the library as it was; the log has no
   half-written op.

## What is deliberately not promised

- **Field order and whitespace.** The output is pretty-printed today for a human reading a diff; a script that
  depended on indentation would break the day it changed.
- **Timings, paths and versions** are included where they are useful (`roots`, `stats`) but are values, not
  schema: the `root` string is whatever the library was opened with.
- **Progress.** Long work (`export`, `sharpen`, `relocate`, a scan) reports through `jobs`, not by streaming
  JSON. There is no `--json` for `export` or `sharpen`: each is a job with progress and a cancel, and pretending a
  job is one object would hide the cancel. Both refuse `--json` and name the verbs that have a schema; the golden
  harness pins that refusal so "no schema" stays a decision rather than an oversight.

## The verbs

Read: `roots`, `keywords`, `stats`, `list`, `folders`, `check`, `moves`, `changed`, `history`, `audit`,
`memories`, `shortcuts`, `registry`, `packs`, `jobs`, `suggest`, `relocate`, `schemas`.

Write: `rate`, `flag`, `label`, `tag`, `untag`, `date`, `rename`, `alias`, `accept`, `pack`, `undo`, `retry`,
`remap`.

Passes (no `--json`): `export`, `sharpen`. `sharpen` measures every photo whose score is missing or came from
another `SCORE_VERSION`, stores them as one undoable change, and prints how many frames could not be judged —
an undecodable or near-black frame is left unscored rather than called sharp.

Once a verb has a schema it keeps one. A verb that is added later and has none refuses and names the ones
that do, which is the difference between "this build cannot do that" and "this build silently printed text".

### Ids and lists

`rate`, `flag`, `label`, `tag`, `untag` and `date` accept `1` or `1,2,3` for their ids, so a script can rate a
page of photos in one call and read one count back. The plain (non-JSON) form of those commands takes the same
list.

### Per-verb notes

Each verb's note lives in the schema table (`--json schemas`) because that is what a script reads. The notes
say which key is the answer, whether the request key differs from the CLI word, and what a count means — for
example `label` takes `label` (not `colour`) and `null` clears it, `suggest.accept` answers `tagged` rather than
`changed`, and `retry` answers `updated` and `remaining`.

## Where this is tested

- `every_write_verb_has_a_json_schema_and_a_stated_contract` — every mutating verb answers `--json` instead of
  refusing, every schema's note is a sentence, and the table marks writes as writes. It was proved to bite by
  renaming a verb in the table: the test fails naming the verb and quoting the refusal.
- The golden steps `40`–`49` in `apps/sortlens-cli/tests/golden.rs` record real answers, including a repeat
  write that reports `{"changed": 0}` and a verb that has no schema.
