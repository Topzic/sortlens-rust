# Sortlens (Rust) — UX and Layout Specification

Requirement driving this document: *all features and tools must be organised and accessible in
the app's layout.* That is an information-architecture problem before it is a visual one.

The design target is an ArtCraft sibling app: LightCraft's discipline (chrome, docking, tokens,
inspector, bottom bar) with Sortlens' culling workflow, not a reskinned web dashboard.

---

## 1. The five surfaces

Everything in the product lives on exactly one of five surfaces. If a feature does not fit, the
feature is wrong, not the surfaces.

| Surface | Contains | Persistent? |
| --- | --- | --- |
| **Top bar** | identity, search/palette, filter toggle, global actions | yes |
| **Left panel** | *what* you are looking at (sources, folders, collections) | yes, collapsible/dockable |
| **Canvas** | *the work* (grid, one photo, map, charts, groups) | always |
| **Right panel** | *details about the current thing* (metadata, tags, suggestions, activity) | yes, dockable, tabbed |
| **Bottom bar** | *which work* (view switcher) + current annotation | yes |

Modal dialogs exist only for: destructive plans, settings, shortcuts editor, about/credits,
first-run setup, and export options. Nothing that is browsed repeatedly is a modal.

---

## 2. Wireframe (default workspace, 1440×900)

```
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ◉ Sortlens   Holidays-2026 ▾    ← →   [ Search photos or ⌘K ]      ⚲ 3   ↑ ⤓  ♪  ?  ◐  ⚙    │ 42
├───────────────────────┬──────────────────────────────────────────────────────┬───────────────┤
│ SOURCES               │  Library                        12,408 photos  ▦ ▤   │ ⓘ Details     │ 48
│  ▣ All Photos   32,288│  ─────────────────────────────────────────────────── │               │
│  ◷ Recently Added 412 │  ★ ≥0  ● ● ● ● ●   ⚑ Pick Unflag Reject   ⌕ tag…    │ Camera        │
│  ⚑ Picks         4,879│  Type ▾  Edited ▾  Camera ▾  Lens ▾  Tag ▾   Clear  │  Sony A7 IV   │
│  ★ Rated         1,003│ ─────────────────────────────────────────────────── │ Lens          │
│  ⚠ Missing          17│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐        │  70-200 f/2.8 │
│  ♻ Rejects       1,134│  │        │ │        │ │        │ │        │        │               │
│                       │  └────────┘ └────────┘ └────────┘ └────────┘        │ Exposure      │
│ FOLDERS       ⌕ ↑↓ +  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐        │  1/500 f/4   │
│  ★ 05-08_Presque_Isle │  │        │ │        │ │        │ │        │        │  ISO 400      │
│  ● 04-13_Jackson_Cre… │  └────────┘ └────────┘ └────────┘ └────────┘        │  135 mm       │
│  ● 04-10_REI_Botany…  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐        │               │
│  ○ 04-25_Lakefield_T… │  │        │ │        │ │        │ │        │        │ Captured      │
│  ⚠ 04-26_Twickenham…  │  └────────┘ └────────┘ └────────┘ └────────┘        │  2026-04-13   │
│                       │                                                      │               │
│ COLLECTIONS    ⌕ ↑↓ + │  ── loaded 120 of 12,408 ──────────────────────────  │ Tags      +   │
│  ● Birds         327  │                                                      │  #wildlife ×  │
│  ★ Nova Scotia   259  │                                                      │  #dawn     ×  │
│  ● Summer         13  │                                                      │               │
├───────────────────────┴──────────────────────────────────────────────────────┴───────────────┤
│ ▤ Library ✦ For You ▷ Cull ◎ Detail ⫼ Compare ⚲ Places ⧉ Similar ◌ Soft Focus ▦ Insights   │ 48
│              ★★★★☆   ⚑ Pick       3 selected   ─────●───   Fit ▾   100% ▾   ⌨                │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

Reading order matches task order: choose scope (left) → narrow (top/filter bar) → work (canvas)
→ inspect/annotate (right) → switch task (bottom).

---

## 3. Top bar

The bar doubles as the macOS title bar (`craft-window`): the empty area drags the window, and
double-click follows the system preference. Controls win over the drag.

Every control is a glyph with a name, tooltip and accessible label; no control in this bar is a word
that a picture can carry. The value chips that used to sit here (theme, density, layout, motion) are
gone: density and layout moved to the canvas header beside the sort, and theme and motion are in
Appearance.

| Element | Behaviour | Command |
| --- | --- | --- |
| App mark + name | The mark is `sortlens-icon`'s own geometry, painted at 16 px, then the product name and the library label | — (the mark is not a menu yet) |
| Search field | Typing filters photos by name; `⌘K` switches the same field to command mode; the hint reads `Search photos or ⌘K` | feeds `library.page` |
| Command palette | Grouped by the command registry's own groups (Library, Review, Organise, View, App, Help), each row printing the shortcut the registry declares; `↑/↓` moves, `Enter` runs, `Esc` returns to photo search with its text untouched. A row that cannot run right now (nothing selected, nothing staged) says why in a notice | every row is a `Ready` registry command |
| Filter button + badge | Toggles the filter bar; the badge is the live active-filter count and is painted onto the button, so the bar never reflows | shell state; the count is `UiState::active_filter_count` |
| Import | Add a folder to index; keyboard `⌘O` | `library.addFolder` |
| Export | Exports the selection; disabled, with a reason on hover, when nothing is selected | `edit.export` |
| Activity bell | Unread count of notices, the recent list with ages, and a clear action; appears only when it has something to say and turns red when a notice is a problem | shell state |
| Help | Shortcut overlay (filterable) | `app.shortcuts` |
| Appearance | Left click cycles the theme; right click lists every preset and the reduced-motion switch | `SetTheme` / `SetMotion` host actions |
| Settings | Sectioned settings dialog | `app.settings` |
| Back / forward | Not shipped: there is no view+scope history yet | — |
| Library menu | Not shipped: opening, recent libraries, backup and remap live in the sidebar and Settings | — |

Search behaviour: **one field, two modes.** Typing searches filenames (live, debounced 120 ms).
Pressing `⌘K` in the field, or anywhere else in the window, switches it to command mode with the
palette hanging under it. `↑/↓` moves, `Enter` runs, `Esc` returns to photo search. The palette's
rows are the only commands it can show: a shared test resolves every one against the registry and
fails if a row is missing from it, is not `Ready`, or is listed twice.

---

## 4. Left panel

Sections are collapsible, reorderable by drag, and can be hidden from the panel's title menu
(stored in settings, so nothing is ever lost). Each section has a header with: name, count,
search icon, sort icon, add icon.

### 4.1 Sources (fixed, cannot be deleted)

Six rows, because six questions: a photograph can be *in* a scope, and a scope is one question with one
answer. Picking a row replaces whatever narrowed the view before, so "Picks" always means the same thing
rather than "picks, among the ones you starred, inside that folder". The folder you are standing in is the
one exception — that is where you are, not what you asked for.

| Row | Predicate | Why it exists |
| --- | --- | --- |
| All Photos | everything whose file is still there | the default |
| Recently Added | recorded into the library in the last 30 days | "what did I just import" |
| Picks | flag = Pick | the cull output |
| Rated | rating ≥ 1 | the curated subset |
| Missing | the file is gone | trust: never lose track |
| Rejects | flag = Reject, still in the grid | the cull's leftovers |

Each count is the count that row would show, not the library total, and every one is computed by the same
predicate the row opens — a row and its grid cannot disagree. `Missing` is the only row that asks about the
filesystem instead of an annotation; the filter bar's `Missing` chip is the same answer approached from the
other side (it widens the current view to include missing files, the row narrows to only those, and either
one lights the chip).

The bin is *not* a scope: it is `Recently Deleted` in the panel's foot (§4.4), because it opens a restore
dialog rather than filling the grid. Not shipped: the per-row "filtered" marker that would say a row's number
is not the whole story.

### 4.2 Folders

A real tree, rooted at each added folder. Each row shows: favourite star, colour dot, name,
count, offline badge. Subfolders expand in place; opening a folder selects it as the scope and
shows subfolders first, then its photos (matching the vendor behaviour).

Controls: search, sort (name / count / date), select-all, deselect-all, add folder. `Ctrl+Click`
two rows selects the range between them for batch operations. Right-click: favourite, colour,
rescan, reveal, remove (with "photos stay on disk" copy), remap, open in new window.

### 4.3 Collections

Same row anatomy minus the tree. Right-click: add selection, remove selection, favourite, colour,
rename, delete (with a "photos and files are not deleted" line). `+ New Collection` creates and
immediately starts inline rename.

Several collections can be acted on at once: click ticks one, ⌘-click adds or removes, ⇧-click takes
the run between the last click and this row *in sidebar order*, and a ticked row carries a dot next to
its colour dot. When two or more are ticked a line appears saying how many, with ✕ to clear; the
context menu then acts on everything ticked, and Rename/Delete stay single-collection unless you chose
"Delete N collections" explicitly. A whole batch undoes with one ⌘Z.

Folder rows carry the same marks collections do: a star when pinned and a colour dot beside the name, both
set from the row's context menu. The mark belongs to the folder's *path identity*, so it survives a rescan and
a drive that comes back with the same name, and it is one undo step.

A folder also remembers how you were browsing it: opening it restores its sort (a folder with no remembered
sort follows the library's) and puts its last search back in the search box, where you can see it and clear it
rather than being filtered quietly.

### 4.4 Left panel footer

A hairline separates the sections from a strip pinned to the panel's foot: `Recently Deleted` (with the
bin's count), `Check Library`, `Recent Changes` and `Keyboard Shortcuts`. These are the rows that open a
dialog instead of choosing what the grid holds, so they are not sources and they do not move when the
sections above them grow.

Each row *is* a command palette row: its label and its shortcut are the command registry's (`library.deleted`,
`library.health`, `library.history`, `app.shortcuts`) and clicking it runs the same handler `⌘K` does. That is
deliberate — a hand-written copy of four dialog flags beside the palette's copy is drift waiting to happen.

Not shipped: the footer's storage summary (photo count, on-disk size, cache size for the selected scope) and
the version/credits block; and the sections are not yet collapsible, reorderable or scrollable as §4 opens by
describing.

---

## 5. Canvas

### 5.1 Contextual header

Every view has the same header structure:

```
[ View title ]                             [ count ]   [ layout toggle ] [ density ]
```

In Library, the title is the scope (folder/collection name) and the count is live. In Cull, the
title is "Cull · <scope>" and the count is "photo 42 of 1,203". In Similar, the title is
"Similar Shots" and the count is "12 groups · 47 photos". In Places the header carries the
library's count and the map's own footer carries the geotagged count. In Insights, the header
holds the scope selector and a date-range chip.

### 5.2 Filter bar (collapsible, identical in every view)

Row 1: rating operator (≥/=/≤) + five stars · flag toggles (Pick / Unflagged / Reject) · five
colour-label swatches · kind (All/Photos/RAW/Videos) · source (Manual / AI / XMP / Not recorded) · edited state.

Shipped today: the minimum-rating chip prints its answer as lit star glyphs (`3★ and up` is gone — a typed
`★` is a per-face speck, not the app's star), the colour chip paints the chosen label's own swatch and an
empty ring for "any colour", the flag chip wears the state's own glyph (`pick`, `reject`, `flag`), and the
keyword menu ends in a chevron. Every one of them is still a cycling toggle with a spoken name, so the
pictures changed and the behaviour did not. The row **wraps** rather than overflowing: at 1200 points with
both docks open, a five-star minimum plus the other chips used to push `Clear` past the canvas edge, and a
filter bar that cannot be undone is worse than a taller one. The whole strip is not painted while the
catalog is empty — nine questions about photographs that do not exist yet sat on top of the Welcome card —
and "empty" is the catalog's own total, never the filtered count, because a filter that hides every photo
must keep the strip that clears it.

The **source** chip answers "did a machine touch this photo's keywords". It is one click to cycle and
one to clear, it counts in the header badge like every other dimension, and it asks about *some*
keyword rather than all of them — a photo with one accepted suggestion among ten typed words belongs in
the answer. "Not recorded" is a real choice, not a fallback: it finds the keywords that were in the
library before sources were kept, which is the honest answer instead of calling them manual.
Row 2 (when narrow): camera, lens, tag pickers · Save as Smart Collection · Clear · Presets.

Every control is a toggle with a tooltip that states the semantics ("any of the selected labels
matches"). Active controls are filled and outlined, not merely tinted, so colour is never the
only signal.

### 5.3 Library view

- Two layouts, both virtualised: **justified** (default, the photo's own aspect, rows of equal height
  filling the width) and **square** (a uniform lattice, cropped to fit). `G` toggles; the chip beside the
  density says which is on; the choice persists.
  - A justified row stops growing when the next photo would push it below the target height, taking
    whichever of the two heights is closer to the target. A short last row keeps the target height and sits
    left-aligned rather than stretching two photographs across the window. A photo wider than the window
    gets a row of its own.
  - Photos with no known dimensions pack as 3:2, because that is the shape of most photographs — a library
    whose metadata never loaded still looks like photographs instead of tiles.
- Density chip: Compact / Comfortable / Roomy, one click to cycle, persisted. It names the minimum cell
  (104 / 132 / 184 pt); in justified mode it is the **target row height** instead, so the setting means the
  same thing in both layouts.
- Thumbnail anatomy: image, selection ring, rating stars (only if > 0), flag, label stripe,
  video badge + duration pill (`0:42`, from the container), offline badge, AI-suggestion dot (if unaccepted
  suggestions exist),
  filename on hover and in captions mode.
- Sketch while a thumbnail is decoding; never a blank cell. A cached thumbnail whose file has gone is painted
  faded (alpha 110) with the `?` badge, and the hover says the picture is from before the drive left. A tier already in memory is painted scaled while
  a sharper one decodes (the detail view, compare and cull do this with the grid's 128 px tier).
- Selection: click, ⌘/Ctrl-click, Shift-click, drag-marquee, ⌘A, ⌘D. Focus ring distinct from
  selection ring (focus = keyboard cursor, selection = chosen set).
- Context menu: Open, Reveal, Open with…, Rate, Label, Flag, Add tag…, Add to collection,
  Remove from collection, Move to folder…, Export…, Delete….
- Infinite scroll with row recycling; no pagination controls.

### 5.4 Detail (lightbox)

```
┌───────────────────────────────────────────────┬──────────────────────┐
│                                               │ 42 / 1,203     ⤢ ✕  │
│                                               │                      │
│              [ the photo, fit ]               │  Histogram           │
│                                               │  Camera / lens       │
│                                               │  Exposure / ISO      │
│                                               │  Dimensions / size   │
│                                               │  Captured / imported │
│                                               │  GPS + place         │
│                                               │  Tags        +       │
│                                               │  Suggestions         │
├───────────────────────────────────────────────┴──────────────────────┤
│  ‹  ★★★★★  ●●●●●  P U X   Tags…   ⌗ Zoom 100% ▾   ⤓ ⤒ E R   ⋯  ›     │
└──────────────────────────────────────────────────────────────────────┘
```

- Fit / Fill / 50 / 100 / 200 / 400 %, click-to-zoom at a configurable ratio (100/200/400).
- Pan by drag or space-drag; double-click toggles fit↔100%.
- Filmstrip (optional) along the bottom, horizontally virtualised, synced selection.
- EXIF panel groups fields; missing fields are omitted rather than shown as "—".
- Prev/next pre-fetches the next and previous photo at loupe size before you ask.
- `Esc` closes, `←/→` steps, `I` cycles info overlay, `L` lights out.

### 5.5 Cull

```
┌───────────────────────────────────────────────────────────┬──────────────┐
│                                                           │ UP NEXT      │
│                                                           │ ┌──────────┐ │
│                                                           │ │  DSC_2937│ │
│                  [ current photo ]                        │ ├──────────┤ │
│                  KEEP overlay (on decision)               │ │  DSC_2938│ │
│                                                           │ ├──────────┤ │
│                                                           │ │  DSC_2939│ │
│                                                           │ └──────────┘ │
├───────────────────────────────────────────────────────────┴──────────────┤
│   Reject   Skip   Undo   Keep    ★ ★ ★ ★ ★    100% ▾   ▦ ⧉ ⓘ   Apply ▾     │
│   ─────────────── 312 kept · 148 rejected · 22 skipped ────────────────    │
└───────────────────────────────────────────────────────────────────────────┘
```

- Current photo centred at fit; zoom levels 1–4 map to 100/200/300/400 % (drag to pan).
- Decision overlay is drawn *inside the photo bounds* (vendor bug fixed by construction: the
  overlay rect is the image rect, not the canvas rect), then the photo slides out.
- Animation: the label fades in first (~90 ms), then the photograph translates out (~140 ms), and the
  overlay is painted inside the image bounds rather than the canvas. The two phases are separate on purpose:
  the label is the confirmation and must land before anything moves, or the eye follows the photograph and
  never reads what was decided.
- Action bar: icon buttons with text labels (Reject/Skip/Undo/Keep), stars, zoom, view toggles,
  Apply menu. All are real buttons, not painted text, so they are focusable and named.
- Session summary line under the bar, updating live.
- Right panel stays available (Tags/Suggestions), so culling and keywording are one pass.
- Inline keyword strip under the action bar: chips for the keywords the photo on screen carries (click
  takes one off) and a field that adds one with Enter. While the field is focused it owns the keys, so
  P and T type letters rather than decide.
- `T` applies the keyword you last added — to the selection in the library, to the photo on screen in
  a cull session — so a look gets one name typed once and then a key per photo. The notice names the
  keyword — "Tagged 5 photos" would leave you guessing what was written.
- `Apply ▾` opens the plan dialog: mode (trash / rejects folder / permanent), count, total size,
  collapsible file list, primary action "Move 148 to Rejects Bin", secondary "Cancel".
- Resuming: entering Cull on the same scope offers "Resume where you left off · photo 313" or
  "Start over", never silently restarts.

### 5.6 Compare / Survey

Compare is a destination of its own in the bottom bar: two-up, the pair at one magnification, the
differences that matter on the line beneath. It opens on the second ticked photo, or the next frame
in the sorted order when only one is ticked, and `C` opens or closes it. Read-only except for
rating/flag, which apply to the primary.

Survey (tiles of 2–9 selected photos) has not shipped: culling is the fast path, compare is the
precision path, and a survey would be a third way of looking at the same decision.

### 5.7 Similar Shots

Group cards: cover, count, "best" badge, similarity range, expand to a row of members. Group
actions: select all non-best, select all, ignore group. Global: threshold slider (re-runs
grouping, not hashing), "only this scope", "re-scan new photos". Threshold changes are instant
because hashes are cached.

### 5.8 Soft Focus

Its own destination in the bottom bar, not a section at the foot of Insights: a "Measure sharpness"
button (shown when photos have no current score, with how many),
a histogram of stored scores, a slider for how much of the library to review, and a "Select N photo(s)" action that
ticks them in the library so you can look before acting. It never deletes; it is a review surface.

**The cut is relative — the softest N% of this library, not "blurry".** A sharpness score is unitless: it ranks
frames inside one library, in one light, at one focal length, and no absolute number means the same in another. So
the section says what a relative cut can actually claim, and when every score is identical it says that a relative
cut cannot separate them rather than offering a button that would select the whole library. Where several photos
share the score at the cut, the selection is honestly larger than the percentage and the count says so.

The slider is session state, not a stored setting: it is a way of looking at the histogram, and remembering yester-
day's cut would make the review appear to make a claim it did not.

### 5.9 Places

Full-canvas map centred on the mean of the library's own photographs, with dots over it. A dot's
radius grows with the square root of its photo count — area tracks the count, so a dot standing
for four hundred photographs is twice the width of one rather than a hundred times — and a dot
for more than one photo has its count printed in it.

Clicking a dot **selects those photos** and names where they are ("Selected 42 photo(s) at
50.82N 0.14W"); clicking the sea selects nothing and clears nothing. `+` and `−` sit over the
map's top-right corner and stop at the ends of the tile set's range: zooming past the top clamps,
because the tiles stopping is not a reason for the dots to stop.

The basemap is a `{z}/{x}/{y}.png` tile folder the person supplies, in `tiles/` beside the
catalog, with an `attribution.txt` beside it. **The credit is printed over the map every frame it
is drawn**, bottom-left, and a folder without one is refused outright — an unlabelled map implies
the application made it. Nothing is downloaded and no map data ships. A tile that is missing or
will not decode is painted as background: a hole in a tile set is a hole, not a notice.

The footer states what is on the map — "4,182 photo(s) with a location · 96 dot(s) · click a dot
to select it" — and the state of the basemap: absent, refused (with the reason), or fine.

**Names come from the person's own gazetteer.** A `places.txt` beside the catalog, tab-separated
`name<TAB>lat<TAB>lon[<TAB>country]`, is what turns "50.82N 0.14W" into "Brighton": a dot within
30 km of a listed place prints its name beneath it, and clicking it says "Selected 42 photo(s) near
Brighton (50.82N 0.14W)". A dot with no place near enough says its coordinates — a coordinate is a
true answer and an invented place name is not. The file is re-read when it changes and nothing is
downloaded; the footer says whether names are in use or why they are not.

**Deliberately absent here:** a vector basemap (a tile folder is what a person can legally supply),
thumbnail fans inside a dot, a geo-bounds filter on the Library (`library.filter {geoBounds}` does
not exist; selecting is what a dot does), and label decluttering — a dense gazetteer will overlap
its labels, and curating the file is the answer until there is a pass that hides colliding ones.

### 5.10 Insights

Cards: total images, library size, reviewed %, space reclaimed. Review progress bar with
kept/rejected/skipped legend. Highlights: first year, busiest month, busiest day, top weekday.
Seasonality chart (fixed height, 12 columns). Scope selector in the header (All / current folder
/ collection). Every number is clickable → jumps to the matching library view.

### 5.11 Memories

A feed of cards, each a provider: On This Day (grouped by year), Highlights (recent favourites),
Places (named clusters), Recently Found (newly indexed). Card actions: open as a grid, play as a
cull session, save as a collection. "Refresh" re-runs providers; results are deterministic per
catalog revision so refresh never reshuffles randomly.

### 5.12 Empty states

Each view declares: title, one-sentence explanation, 1–2 primary actions, optional hint.
Examples: no library → "Add a folder to begin. Your photos stay where they are." with [Add
folder]; empty filter → "No photos match these filters." with [Clear filters]; Cull done →
"Session complete · 312 kept, 148 rejected" with [Apply decisions] [Review rejected].

---

## 6. Right panel

Four tabs (dockable, reorderable, hideable) plus a header that names the current item:

| Tab | Contents | Applies to |
| --- | --- | --- |
| **Details** | file, dimensions, size, dates, camera, lens, exposure, GPS/place, video info | one photo |
| **Tags** | tag chips with remove, add field with autocomplete, "re-apply previous", suggestion count; a chip that was not typed by hand says so (`cat · AI`), because a claim whose origin you have to hover to see is one you will not check | one or many |
| **Suggestions** | grouped by family (objects / scene / EXIF), each row with its source and confidence, `accept all` per family, `×` to dismiss. A proposal that covers only part of the selection prints how many photographs it applies to; one that covers all of them does not, because `3×` on every row of a three-photo selection is noise | one or many |
| **Activity** | job list (also in the bell), history of the last operations, audit entries for destructive ops | library |

A thin iconic strip at the far right switches tabs and shows which is active (ArtCraft pattern),
so the panel never disappears when you switch tools: the strip is painted whether the body is open or not,
clicking a tool opens its tab beside the strip, and clicking the active tool is the one gesture that closes
the body — a rail you cannot close from the rail keeps 340 points of a 900-point window forever. The panel's
header names the active tool in words, because the strip is glyphs and a glyph alone is a guess for anyone
who has not learned it. Accepting and dismissing are one ⌘Z for the whole selection: a kind accepted is one
change, and a dismissal is recorded in the library, so a proposal you refused does not come back the next
time the panel asks.

---

## 7. Bottom bar

Left: view switcher — Library, For You, Cull, Detail, Compare, Places, Similar, Soft Focus,
Insights — each with its own icon and its name. The row is measured, not guessed: when the labels
plus the controls beside them no longer fit, the row collapses to the current destination plus a
caret that opens the same nine items. A narrow window loses width, never a destination.

Centre: the annotation pill. Five stars (clicking the lit star clears the rating), Pick/Unflag/
Reject, the primary's keyword count when it has keywords, and "N selected". The stars and flags
call the same operations as `0–5` and `P`/`X`/`U`, and act on the whole selection.

Right: thumbnail size slider in the grid view (the same three steps the canvas header cycles, from
one state), Fit/1:1 in Detail and Compare (also toggled by a click on the photograph and by `Z`),
shortcut help (`?`), and a "reduce motion" indicator while animations are off.

Every destination is a `Ready` registry command (`view.detail`, `view.compare`, `view.softFocus`,
`view.places`, `view.similar`, `view.insights`, `cull.start`), so the bar is a front door to the
command registry rather than a second list that could drift from it.

---

## 8. Dialogs

| Dialog | Trigger | Contents |
| --- | --- | --- |
| Settings | `⌘,` | Left nav: General, Review, Appearance, Library, AI, Remote, Shortcuts, Updates, About |
| Shortcuts | `?` | Searchable table, grouped by mode; rebinding and import/export arrive with `craft-keymap` (M7+) |
| Apply cull decisions | `Apply` | Mode, counts, size, file list, primary/secondary |
| Delete permanently | second confirmation for the `Permanent` mode | Type-to-confirm, counts |
| Export | selection | Format (original/JPEG/PNG), size, quality (JPEG only), metadata (JPEG only), destination, collision policy |
| First run | launch with no library | 3 steps: create/open library → add folder → optional remote test |
| About / credits | Help | version, licences, font/model attribution, update check |
| Library doctor | Settings → Library / Missing source | Per-folder status table, rescan, remap, remove-missing |
| AI models | Settings → AI | Consent switch for model downloads (network, opt-in), then one row per pinned model: licence, size, install state, Install / Remove; a download shows its progress and a Stop that keeps the partial file |

Destructive dialogs follow one template: what will happen, how many, how big, a list, the
irreversible warning only in permanent mode, and the safe action as the default button.

---

## 9. Keyboard map (defaults)

| Scope | Keys |
| --- | --- |
| Global | `⌘K` palette · `⌘,` settings · `?` shortcuts · `⌘F` search · `⌘O` add folder · `⌘Z`/`⇧⌘Z` undo/redo · `⌘S` save sidecars now · `F11` full screen |
| Views | `1..8` switch view when the grid is not focused? No — digits are ratings. Use `⌘1..8` for views, `G` grid↔square, `\` filter bar, `D` detail, `C` cull |
| Library | arrows move focus · `Space` quick look · `Enter` open detail · `0–5` stars · `6–9` labels · `P`/`U`/`X` pick/unflag/reject · `⌘A`/`⌘D` select/deselect · `R` reveal · `E` edit · `Delete` delete… |
| Cull | `→`/`K` keep · `←`/`X` reject · `↓`/`S` skip · `⌫`/`U` undo · `F` favourite · `1–4` zoom · `Space` pan · `Tab` inspector · `A` apply |
| Detail | `←/→` step · `+/-` zoom · `0` fit · `1` 100 % · `I` info · `L` lights out · `Esc` close |
| Map | `+/-` zoom · arrows pan · `0` world · `Enter` open cluster as grid |
| Palette | `↑/↓` move · `Enter` run · `Tab` section · `Esc` close |

Conflicts are detected by `craft-keymap`; the defaults avoid OS-reserved chords; user overrides
are stored in settings and exportable.

---

## 10. Themes and tokens

Four presets, all contrast-checked:

| Preset | Use |
| --- | --- |
| **Charcoal** (default, dark) | general editing in normal light |
| **Midnight** (near black) | culling in a dark room |
| **Silver** (light grey) | bright offices |
| **Paper** (near white) | printing/colour review |

Tokens (colour): chrome, canvas, grid_bg, cell, cell_selected, skeleton, ring_active,
ring_inner, ring_selected, divider, inset, field, field_border, button, button_border, hover,
pressed, tool_active, text, text_label, text_dim, text_disabled, icon, track, thumb, thumb_hover,
accent, star, pick, reject, caution, mask-free (no masks in v1).
Tokens (metrics): top_bar_h 42, bottom_bar_h 48, panel_w 270, strip_w 48, row_h 44,
thumb 90–480, section_h 57.

Rules: no literal colours in widgets (theme lookup only); label colours are functions so the
same red is used in the grid, filter bar, inspector and toast; every preset has a test asserting
text ≥ 4.5:1 and rings/controls ≥ 3:1 against their actual surfaces.

Appearance mode: Auto (follow OS) / Light / Dark, with the preset remembered per mode, so
switching the OS does not destroy the user's choice. Theme packages (M10) map a
`craft-ext-core` palette onto these tokens; unknown token names are ignored and the app
re-checks its own contrast pairs after applying.

---

## 11. Accessibility

- Focus: a single roving-tabindex model in the grid; `Tab` moves between regions (bars, panels,
  canvas), arrows move within the grid; the focus ring is always visible and distinct from
  selection.
- Names: icon-only buttons get an accessible label beginning with the visible label; the grid
  cell's label is "<filename>, <date>, <rating> stars, <flag>, <label>"; the cull buttons
  announce their decision.
- State: selected/checked/pressed are reported, not just painted.
- Live regions: "Photo 42 of 1,203, DSC_2937", "Kept. 312 kept, 148 rejected", "3 photos
  selected", "Import finished: 412 photos".
- Errors are announced and also shown in text; a toast alone is insufficient.
- Colour independence: flag icons, label names in tooltips/accessible text, star counts as text.
- Reduced motion: the cull overlay draws its label at full strength and skips the slide entirely, so the
  stage has already cut to the next photo. The toggle is ours (`appearance.motion`, persisted, and a chip in
  the top bar because it is the setting you turn on while the animation is bothering you). The operating
  system's own "reduce motion" preference is **not** read: egui exposes no cross-platform signal for it, and
  guessing from a platform API on one platform is worse than a setting that works everywhere. That is named in
  M10's deliberately-absent table with its unblocker.
- Zoom: UI scale 0.8–2.0, and the layout reflows rather than clipping (two-row filter bar,
  collapsed bottom bar, single-column inspector).
- Contrast: enforced by theme tests; the high-contrast preset adds a pure black/white/yellow
  scheme and stronger strokes.
- i18n: every label is an English key; RTL mirrors the shell (sidebar to the right, icons
  mirrored where directional); the photo is never mirrored.

---

## 12. Component inventory (build these once)

| Component | Notes |
| --- | --- |
| `IconButton` | from `craft_ui::buttons`, our style adapter; accessible name required |
| `StarRating` | 5 stars, 0 allowed, keyboard 0–5, batch mode shows mixed state |
| `FlagPill` | Unflag/Pick/Reject with icons and text tooltips |
| `LabelSwatch` | 5 colours, ring + tick for selection, name in accessible text |
| `TagChip` | removable, coloured by first use, overflow "+N" |
| `FilterToggle` | filled + outlined when on; never colour-only |
| `Picker` | dropdown with search, "Any …" first item, active outline |
| `SectionHeader` | name, count, actions, collapse, drag handle |
| `SidebarRow` | icon, name, count, badges, colour dot, drag payload |
| `Thumb` | skeleton, image, overlays, selection/focus rings, badges |
| `CanvasHeader` | title, count, layout toggle, density |
| `CullActionBar` | 4 decision buttons + stars + zoom + apply |
| `PlanDialog` | counts/size/list, mode choice, safe default |
| `Toast` | text + optional action (Undo), announced |
| `ActivityRow` | name, progress, cancel, error expander |
| `EmptyState` | title, body, 1–2 actions, hint |
| `Inspector` | tabbed dock panel with a header naming the item |
| `Chart` | fixed-height painter for seasonality/histogram (no chart dependency) |
| `MapCanvas` | pan/zoom, clusters, thumbnails, attribution |

Each component gets: a kittest snapshot in light/dark/RTL, an accessibility assertion, and a
"used in" list. No component may read global state directly; it takes data + a command callback.

The shell-level suite is real today: `cargo xtask screenshots` paints eight surfaces in Charcoal and
Paper against committed baselines, and fails on a pixel difference beyond the tolerance in
`kittest.toml` (`docs/ARCHITECTURE.md` §12). Component-level and RTL snapshots land with the component
work in 3B and the i18n scaffolding in Phase 4; until then the shell suite is the visual gate, and a
baseline is the review of what a change looks like rather than a claim that the design is finished.

## Icons

One set, `crates/ui/src/icons.rs`: 40 monochrome 24-unit SVGs under `assets/icons/`. `craft_ui::icons`
prepares each glyph once (white `currentColor`, 1.75 stroke) and `egui_extras`' SVG loader rasterises it at
exactly the size its slot asks for, so an icon paints in the theme's own colour and stays sharp at 2×.

Where they paint today: the Welcome promises' three rows; the sidebar's source rows, folder rows, keyword rows and
collection rows; the filter
strip's chips (rating, flags, colour, kind, source, edited, missing, clear); every bottom-bar destination, as
a glyph beside its label; the right panel's four-tab far-edge rail; the canvas header's density and layout controls; and
the top bar's Settings (`sliders`), Help (`keyboard`), Appearance (`contrast`), Activity (`bell`), Export
(`export`), Import (`folder-plus`), Filter (`funnel`) and panel toggle (`panel`). An icon-only button carries
its words as the tooltip *and* the accessible name: replacing a word with a picture must not remove the word
from a screen reader.

Four tests hold the set together. The enum, the catalog and the folder must name the same icons in both
directions; a glyph with a hard-coded colour or a missing `currentColor` fails; every glyph must rasterise
through the real loader to something with both ink and empty space at 14 and 32 px; and the shell test counts
the textured rectangles painted in the top bar, bottom bar and sidebar, so a surface that loses its icons
fails rather than quietly shrinking; a second counts the three Welcome glyphs in their own column; and two more
hold the filter strip (three lit stars for a three-star minimum and the label's own swatch, plus the strip's
absence on an empty library). Still target state: the `←`/`→` text in the keymap overlay (3B.6c–g). The `⌘K`
palette's rows are text by design — a command is a sentence, and a glyph for every verb would be a second
vocabulary to learn.

The widget states themselves come from the same place: `Tokens::palette` is the `craft_ui::theme::WidgetPalette`
mapping, and `apply` installs it, so a suite widget reading `Visuals` and a Sortlens widget reading `Tokens`
agree about what a hovered or pressed control looks like by construction rather than by two lists kept in step.

## Charts

Two shapes, both drawn by `crates/ui/src/charts.rs`: a column chart and a proportional row. No chart
dependency, no animation, no viewport-relative height — a chart that grows with the window makes the
same library look different at different window sizes and makes two screenshots incomparable.

- A non-zero value never rounds below one pixel: an invisible bar reads as "nothing here".
- The axis maximum is a round number (1, 2 or 5 × a power of ten), so a reader can estimate a bar
  without hovering it.
- Every column and row carries a hover tooltip with the exact count and share, because a chart you
  cannot read a number off is decoration.

## The export dialog

Reachable from the cull view ("Export picks…"), the grid's context menu and the detail header, so it
is available wherever someone has decided what to keep.

It shows, in one place: how many files and what they will be (always in words, never a bare format
code), the format as two chips, the longest edge and quality for a re-encode, whether metadata travels,
and the collision policy as three words — *Skip*, *Keep both*, *Replace* — because "rename" and
"overwrite" are terms about files, not about photos. A copy says so plainly: "same bytes, same name, no
resize and no re-encode".

While it runs, the dialog shows a progress bar with "n of total" and a **Stop** button, and the activity
panel shows the same job. When it finishes, a receipt replaces the dialog with what was written, into
which folder, and the sentence that matters: the originals are where they were.

## Accessibility: what is claimed and what is checked

Announcements are visible polite live regions (`a11y::live_line`), not invisible AccessKit-only text, so the
status a screen reader hears is the status everyone can see.

Every region the app paints itself — grid cells, the detail preview, the lookahead strip, cull thumbnails,
similar clusters, memory cards and sidebar rows — registers a name: `a11y::describe_photo` says the file
name first, then stars, flag, missing state, colour label and keywords, because that is the order that
decides a cull. `cargo xtask a11y` fails when a function in the shell makes something clickable without
naming it.

What is **not** checked here: the accessibility tree an assistive client actually receives. egui exposes no
public way to read nodes back in this version, so that needs a real screen reader on a machine with a
desktop session. Until then this document says "named", never "audited".
