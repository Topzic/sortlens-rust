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
- **Progress.** Long work (`export`, `sharpen`, a scan) reports through `jobs`, not by streaming JSON.
  `add`, `rescan`, `batchsuggest`, `sharpen`, `export`, `model-install`, `model-remove` and `update-check`
  each start a pass with progress and a cancel, and pretending a pass is one object would hide the cancel: they
  refuse `--json` and name the verbs that do answer. `diagnostics` refuses too, because its answer is the
  redacted text bundle a person attaches to a report, not an object a script parses. The refusal is a decision
  recorded in the tests, not an omission: the golden harness pins one, and the contract test names every pass.

## The verbs

Read: `roots`, `keywords`, `stats`, `insights`, `list`, `page`, `folders`, `check`, `moves`, `changed`,
`history`, `audit`, `memories`, `shortcuts`, `registry`, `packs`, `jobs`, `suggest`, `models`, `update`,
`report`, `marks`, `relocate`, `schemas`.

Write: `rate`, `flag`, `label`, `tag`, `untag`, `date`, `rename`, `alias`, `accept`, `pack`, `undo`, `retry`,
`remap`, `mark`, `delete`, `move`, `unmove`, `refresh`, `compact`, `backup`, `restore`, `model-consent`,
`update-consent`.

`page` is the same answer as `list --json`, with `--offset`; `report` prints the support report's object with or
without `--json`, so a script can ask for it either way. `backup` and `restore` write an archive rather than the
catalog, which is why they are writes: running one twice is safe for the library but is still an effect. `move`
takes an absolute destination folder and refuses a relative one rather than guessing which folder was meant.

### The same flags

A verb's JSON form reads the same flags its plain form reads, and refuses a value it cannot honour. That is a
contract, not an implementation detail: `list --json --tag coast` must be the filtered page, not the whole
library with a keyword somebody forgot to apply. `--source` takes `manual`, `ai`, `xmp` or `unrecorded`, and
an unknown word is refused with the list of real ones.

`delete <ids> [--mode trash|tag|move --target F]` goes through the same audited batch a cull applies: every chosen
file is checked against the size and modification time the catalog recorded, and one changed file refuses the whole
batch rather than deleting the others. `--mode tag` moves nothing — it flags the photos as rejects, which is what a
script that wants to review before acting should use.

Passes (no `--json`): `add`, `rescan`, `batchsuggest`, `sharpen`, `export`, `model-install`, `model-remove`,
`update-check`; `diagnostics` is text for the same kind of reason. `sharpen` measures every photo whose score is missing or came from
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
`changed`, `suggest.dismiss` answers `dismissed` — how many photographs newly stopped being offered that keyword,
so a repeat reports `0` — and `retry` answers `updated` and `remaining`.

## Where this is tested

- `every_write_verb_has_a_json_schema_and_a_stated_contract` — every mutating verb answers `--json` instead of
  refusing, every schema's note is a sentence, and the table marks writes as writes. It was proved to bite by
  renaming a verb in the table: the test fails naming the verb and quoting the refusal.
- The golden steps `40`–`69` in `apps/sortlens-cli/tests/golden.rs` record real answers, including a repeat
  write that reports `{"changed": 0}`, a keyword-filtered page that fails if `--tag` stops being applied, and a
  pass that still has no schema on purpose.
- The contract test's lists are the whole surface: 23 one-object writes must have a schema, eight passes and
  `diagnostics` must refuse one, and every note must be a sentence.
- `the_usage_text_names_every_verb_the_table_offers` reads the help text a person sees and requires every
  schema verb to appear in it — the check that would have caught the stale usage text this change rewrote.
