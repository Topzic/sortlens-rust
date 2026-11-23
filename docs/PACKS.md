# Keyword packs

A pack is a small JSON file that says "when a photo looks like this, these words apply". It is the
part of tagging that needs no model: no download, no licence negotiation, no network — just a file
someone can write, read, copy or hand to a friend.

Packs live in `<library>/packs/`, where `<library>` is the folder Sortlens keeps its index in:

| Platform | Folder |
| --- | --- |
| macOS | `~/Library/Application Support/Sortlens/Library/packs/` |
| Linux | `~/.local/share/sortlens/library/packs/` |
| Windows | `%APPDATA%\Sortlens\Library\packs\` |

`SORTLENS_LIBRARY_DIR` overrides the library folder, and `packs` on the command line prints the exact
path for the library you have open. Switching a pack on or off is a change like any other: it is in the
op log, it survives a restart, and `undo` takes it back.

## The file

```json
{
  "name": "Travel",
  "licence": "CC0-1.0",
  "keywords": [
    { "keyword": "japan", "when": { "folder_contains": "Japan" } },
    { "keyword": "night", "when": { "min_iso": 3200 } },
    { "keyword": "street", "when": { "focal_band": "wide", "camera_contains": "X100" } }
  ]
}
```

- `name` — what settings shows. Two packs with the same name are allowed; the file name is what
  `packs` reports, so a duplicate is visible rather than silent.
- `licence` — required, and shown next to the pack. A keyword list is someone's work; the field exists so
  that is never a surprise.
- `keywords` — at least one rule. `keyword` is trimmed and compared case-insensitively against the
  photo's existing keywords, exactly like a keyword you type.

## When a rule applies

Every field in `when` must hold. They are an **AND**, not an OR.

| Field | Matches when |
| --- | --- |
| `folder_contains` | the photo's path, relative to its indexed root, contains this text |
| `camera_contains` | the camera make or model contains this text |
| `lens_contains` | the lens model contains this text |
| `min_iso` | the photo's ISO is at least this number |
| `focal_band` | the focal length is `wide` (≤ 35 mm), `normal` (≤ 70 mm) or `telephoto` |

Text comparisons ignore case on both sides. A condition that the photo has no value for does not hold:
a photo with no EXIF focal length does not match `focal_band`, and a photo with no ISO does not match
`min_iso`.

## What is refused, and why

| Refused | Because |
| --- | --- |
| a rule with no conditions | it would tag every photo in the library, and one typo should not do that |
| a blank `name`, `licence` or `keyword` | the settings row would be unreadable |
| a pack with no keywords | it would do nothing, and look like it was working |
| `focal_band` other than the three names | a band that does not exist would silently match nothing |
| an unknown field inside `when` | a misspelled condition must fail loudly, not be ignored |

A pack that cannot be read is **listed** with its problem, naming the file, rather than being skipped.
It is not switched off behind your back: a pack you were relying on keeps working or tells you why it
stopped.

## What a pack does not do

A pack produces **proposals**. They appear in the detail view's "Suggested" row beside the camera and
lens hints, with their score and the word `pack`, and in the right panel's Suggestions tab for the whole
selection, under the `Scene` family. Nothing is written until you accept one, and accepting writes an
ordinary keyword — in the log, in the history, undone by the same `undo`. Accepting every proposal of a
family is one change and one undo. A proposal you refused with `×` is recorded in the library and is not
offered again for that photograph; one undo returns it.

Packs are not a substitute for a model. They cannot tell you what is *in* a photo; they can only apply
words you already decided on, to photos that match conditions you can read in the file.
