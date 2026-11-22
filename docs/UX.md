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
│ ▤ Library  ▷ Cull  ◎ Detail  ⫼ Compare  ⚲ Places  ⧉ Similar  ◌ Soft Focus  ▦ Insights        │ 48
│              ★★★★☆   ⚑ Pick       3 selected   ─────●───   Fit ▾   100% ▾   ⌨                │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

Reading order matches task order: choose scope (left) → narrow (top/filter bar) → work (canvas)
→ inspect/annotate (right) → switch task (bottom).

---

## 3. Top bar

| Element | Behaviour | Command |
| --- | --- | --- |
| App mark + library name | Click = library menu (recent, open, new, backup, remap) | `app.libraryMenu` |
| Back / forward | Navigate view+scope history (not photo history) | `view.back`, `view.forward` |
| Search field | Typing filters photos by name; `⌘K` opens the palette; `Esc` clears | `library.filter {text}` |
| Filter button + badge | Toggles the filter bar; badge counts active filters | `view.filterBar` |
| Import | Add folder or files; keyboard `⌘O` | `file.addFolder`, `file.addFiles` |
| Export | Export selection | `dialog.export` |
| Activity bell | Unread count of notices, the recent list with ages, and a clear action; appears only when it has something to say and turns red when a notice is a problem | `panel.activity` |
| Help | Shortcut overlay, docs, diagnostics | `app.shortcuts` |
| Appearance | Auto/Light/Dark cycle; right-click = theme list | `view.appearance` |
| Settings | Sectioned settings dialog | `app.settings` |

The bar doubles as the macOS title bar (`craft-window`): the empty area drags the window, and
double-click follows the system preference. Controls win over the drag.

Search behaviour: **one field, two modes.** Typing searches filenames (live, debounced 120 ms).
Pressing `⌘K` in the field switches to command mode with grouped results: Views, Photo actions,
Library, Settings. `↑/↓` moves, `Enter` runs, `Esc` returns to photo search.

---

## 4. Left panel

Sections are collapsible, reorderable by drag, and can be hidden from the panel's title menu
(stored in settings, so nothing is ever lost). Each section has a header with: name, count,
search icon, sort icon, add icon.

### 4.1 Sources (fixed, cannot be deleted)

| Row | Predicate | Why it exists |
| --- | --- | --- |
| All Photos | everything not binned | the default |
| Recently Added | indexed in the last 30 days | "what did I just import" |
| Picks | flag = Pick | the cull output |
| Rated | rating ≥ 1 | the curated subset |
| Missing | file not found | trust: never lose track |
| Rejects (bin) | binned, restorable | undo at library scale |

Counts are always visible; a filter in effect adds a subtle "filtered" marker on the row so the
number is not mistaken for the total.

### 4.2 Folders

A real tree, rooted at each added folder. Each row shows: favourite star, colour dot, name,
count, offline badge. Subfolders expand in place; opening a folder selects it as the scope and
shows subfolders first, then its photos (matching the vendor behaviour).

Controls: search, sort (name / count / date), select-all, deselect-all, add folder. `Ctrl+Click`
two rows selects the range between them for batch operations. Right-click: favourite, colour,
rescan, reveal, remove (with "photos stay on disk" copy), remap, open in new window.

### 4.3 Collections

Same row anatomy minus the tree, plus a cover thumbnail. Right-click: add selection, remove
selection, favourite, colour, rename, delete (with a "photos and files are not deleted" line).
`+ New Collection` creates and immediately starts inline rename.

### 4.4 Left panel footer

Storage summary for the selected scope (photo count, on-disk size, cache size), app version,
and a link to About/credits. This is the only permanent branding area.

---

## 5. Canvas

### 5.1 Contextual header

Every view has the same header structure:

```
[ View title ]                             [ count ]   [ layout toggle ] [ density ]
```

In Library, the title is the scope (folder/collection name) and the count is live. In Cull, the
title is "Cull · <scope>" and the count is "photo 42 of 1,203". In Similar, the title is
"Similar Shots" and the count is "12 groups · 47 photos". In Places, the count is "13,369
geotagged". In Insights, the header holds the scope selector and a date-range chip.

### 5.2 Filter bar (collapsible, identical in every view)

Row 1: rating operator (≥/=/≤) + five stars · flag toggles (Pick / Unflagged / Reject) · five
colour-label swatches · kind (All/Photos/RAW/Videos) · edited state.
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
- Animation: label fades/scales first (~90 ms), then translate (~140 ms). Reduced motion skips
  the translate and cuts the fade to 0.
- Action bar: icon buttons with text labels (Reject/Skip/Undo/Keep), stars, zoom, view toggles,
  Apply menu. All are real buttons, not painted text, so they are focusable and named.
- Session summary line under the bar, updating live.
- Right panel stays available (Tags/Suggestions), so culling and keywording are one pass.
- `T` applies the keyword you last added to the current photo (or the whole selection), so a look gets one
  name typed once and then a key per photo. The notice names the keyword — "Tagged 5 photos" would leave you
  guessing what was written.
- `Apply ▾` opens the plan dialog: mode (trash / rejects folder / permanent), count, total size,
  collapsible file list, primary action "Move 148 to Rejects Bin", secondary "Cancel".
- Resuming: entering Cull on the same scope offers "Resume where you left off · photo 313" or
  "Start over", never silently restarts.

### 5.6 Compare / Survey

Two-up compare with synced pan/zoom and a divider; survey tiles 2–9 selected photos; both are
read-only except for rating/flag, which apply to the active photo. Auto-advance optional.
(Deliberately P2: culling is the fast path, compare is the precision path.)

### 5.7 Similar Shots

Group cards: cover, count, "best" badge, similarity range, expand to a row of members. Group
actions: select all non-best, select all, ignore group. Global: threshold slider (re-runs
grouping, not hashing), "only this scope", "re-scan new photos". Threshold changes are instant
because hashes are cached.

### 5.8 Soft Focus

A sortable list/grid of photos by sharpness ascending, with the score bar, a threshold slider
and a "select below threshold" action. The threshold persists per library. It never deletes;
it is a review surface.

### 5.9 Places

Full-canvas map, offline vector basemap, clusters that expand into thumbnail fans, zoom to
cluster, "13,369 geotagged · 431 without location" chip, filter-bar scoping. Clicking a cluster
filters the Library view to those photos (`library.filter {geoBounds}`). Attribution sits in the
bottom-right corner of the map, as required.

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
| **Tags** | tag chips with remove, add field with autocomplete, "re-apply previous", suggestion count | one or many |
| **Suggestions** | grouped by source (objects / scene / EXIF), confidence, accept-all-of-kind, dismiss | one or many |
| **Activity** | job list (also in the bell), history of the last operations, audit entries for destructive ops | library |

A thin iconic strip at the far right switches tabs and shows which is active (ArtCraft pattern),
so the panel never disappears when you switch tools.

---

## 7. Bottom bar

Left: view switcher (Library, Cull, Detail, Compare, Places, Similar, Soft Focus, Insights) with
icons + labels in the tooltip; when the window is narrow it collapses to the current view icon
plus a menu, with the same items.

Centre: rating pill (5 stars), flag buttons (Unflag/Pick/Reject), tag count, "N selected".

Right: thumbnail size slider (grid views), zoom + click-zoom (detail/compare), shortcut help
(`?`), and a "reduce motion" indicator when animations are off.

Everything in this bar is also a menu item and a command, so nothing is only-here.

---

## 8. Dialogs

| Dialog | Trigger | Contents |
| --- | --- | --- |
| Settings | `⌘,` | Left nav: General, Review, Appearance, Library, AI, Remote, Shortcuts, Updates, About |
| Shortcuts | `?` | Searchable table, grouped by mode; rebinding and import/export arrive with `craft-keymap` (M7+) |
| Apply cull decisions | `Apply` | Mode, counts, size, file list, primary/secondary |
| Delete permanently | second confirmation for the `Permanent` mode | Type-to-confirm, counts |
| Export | selection | Format (original/JPEG/PNG), size, quality, metadata, destination, collision policy |
| First run | launch with no library | 3 steps: create/open library → add folder → optional remote test |
| About / credits | Help | version, licences, font/model attribution, update check |
| Library doctor | Settings → Library / Missing source | Per-folder status table, rescan, remap, remove-missing |

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
- Reduced motion: overlays fade only; no slide, no zoom animation; respects OS setting + an
  in-app toggle.
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
