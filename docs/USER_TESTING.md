# Click-through testing

This is the checklist for a pass that uses Sortlens the way a photographer does: launch the
window, click the control, and look at the result. A story is finished only when that interaction
passes. A failing story is fixed in the same pass and tried again. It is not skipped, and it is
not checked off from a unit test or a screenshot harness alone.

`docs/FEATURES.md` is the inventory. `docs/UX.md` is the layout the window is held to. This file
is the order a person walks them.

## How a pass runs

One wake walks several stories. A click that passes is short, so the wake keeps going. Feature
stories come first, in the order written. Interface stories start only after every feature story
is checked or recorded as blocked. The next wake starts again at the first story that is still
unchecked.

1. `git fetch` before editing so this file is the latest one on `main`. Other agents work in
   this repository. This pass does not take the plan loop's lease and does not ship a roadmap slice.
2. Launch the desktop app if it is not already showing a window: `cargo run -p sortlens`.
   Use a real library with photographs in it. If the machine has none, add one small folder of
   the owner's photos and say which folder. Do not invent a second app, and do not drive this
   through a browser. The window is a native egui app.
3. Take the next unchecked story. Do the clicks and keystrokes. Look at the window after each
   one. A pass needs the control to be reachable, the result to be the one the story names, and
   the screen to still match `docs/UX.md` (nothing clipped, overlapping, or left in English by
   accident).
4. When it passes, change `- [ ]` to `- [x]` and append one line to the log: the date, the
   story id, and what the window showed. Then take the next story in the same wake.
5. A story whose feature `docs/ROADMAP.md` lists as backlog, or as waiting on something outside
   the repository, is not built in this pass. Write `blocked: <reason>` in its note, leave the
   box unchecked, and continue with the next story.
6. If a story fails, errors, or needs a code change, stop the walk. Fix the smallest change
   that makes that story pass, and try it again in the window. When the gates that change
   requires are green (`cargo xtask ci`, and `cargo xtask screenshots` when `crates/ui`
   changed), commit the fix and the checklist on the current branch, push, and end the wake.
   Do not start another story after a fix.
7. If the wake only checked stories and changed no code, commit the checklist and push that
   too, so the next wake does not repeat them, then end. A blocked story is not a fix.

Destructive stories (delete, empty the rejects bin, apply a cull) use a copy of a folder, never
the only copy of a photograph.

## Feature stories

### Library

- [ ] **UT-A01 Add a folder.** As a photographer I can add a folder from the sidebar and see its photos in the grid without the files moving. Pass: the folder row appears, its count matches the photos shown, and a notice names the index. Note:
- [ ] **UT-A02 Several folders.** As a photographer I can add a second folder and browse all photos, then one folder at a time. Pass: both rows show their own counts, and scoping one folder hides the other. Note:
- [ ] **UT-A03 Subfolders.** As a photographer I can open a folder and walk into a subfolder, then back up. Pass: the canvas title is the folder's own name, and Up one level returns to the parent. Note:
- [ ] **UT-A04 Counts.** As a photographer I can trust the number beside a folder, a source, and a collection after I rate or reject a photo. Pass: the count changes on the next frame and matches the grid. Note:
- [ ] **UT-A05 Pin a folder.** As a photographer I can pin a folder from its menu and see it as a favourite. Pass: the pin survives a relaunch. Note:
- [ ] **UT-A06 Folder colour.** As a photographer I can set a folder's colour and see the dot on its row. Pass: the colour is the same swatch the photo labels use. Note:
- [ ] **UT-A07 Folder search.** As a photographer I can type part of a folder name and see only the matching rows. Pass: clearing the field restores the full list. Note:
- [ ] **UT-A08 Folder sort.** As a photographer I can sort folders by name, count, and date. Pass: the order changes and is still right after a relaunch. Note:
- [ ] **UT-A13 Missing photos.** As a photographer I can open Missing and see a photo whose file is gone. Pass: the row says the file is missing, and a present photo is not in the list. Note:
- [ ] **UT-A14 Sources.** As a photographer I can switch All Photos, Recently Added, Picks, Rated, and Rejects. Pass: each row's grid is that set, and All Photos clears the scope. Note:
- [ ] **UT-A15 Rejects bin.** As a photographer I can open the rejects bin, read a batch, restore it, and see the photos return. Pass: restore puts the files back and the batch leaves the bin. Use a copy. Note:
- [ ] **UT-A16 Recently Deleted.** As a photographer I can open Recently Deleted and see what the bin removed. Pass: the dialog lists the batch and an empty bin says so. Note:
- [ ] **UT-A17 Rescan.** As a photographer I can rescan a folder after adding a file on disk. Pass: the new photo appears and unchanged photos are not re-copied. Note:
- [ ] **UT-A18 Stop a scan.** As a photographer I can cancel an index from the activity row. Pass: the scan stops, the notice says it was cancelled, and the library stays consistent. Note:
- [ ] **UT-A19 Remove a folder.** As a photographer I can remove a folder from the library without deleting its files. Pass: the row is gone and the files are still on disk. Note:
- [ ] **UT-A20 Remap.** As a photographer I can point a missing folder at its new location. Pass: the photos resolve and the grid shows them. Note:

### Grid and filters

- [ ] **UT-B01 Justified grid.** As a photographer I can view the library as justified rows. Pass: rows share a height, aspects stay natural, and the last row does not stretch one photo across the screen. Note:
- [ ] **UT-B02 Square grid.** As a photographer I can switch to square cells and back. Pass: both layouts show the same photos, and the header glyph matches the layout in force. Note:
- [ ] **UT-B04 Density.** As a photographer I can change thumbnail size from the header and from the bottom-bar slider. Pass: the grid reflows, and both controls agree. Note:
- [ ] **UT-B05 Search.** As a photographer I can type a filename in the top bar and see the grid narrow as I type. Pass: a name that exists is shown, a name that does not shows the empty-filter screen, and clearing the field restores the library. Note:
- [ ] **UT-B06 Sort.** As a photographer I can change the sort from the canvas header and see the menu's seven orders. Pass: the chosen order is what the grid uses, and the menu words match the current value. Note:
- [ ] **UT-B07 Rating filter.** As a photographer I can filter by stars. Pass: the grid keeps only photos at that rating, the chip shows the choice, and Clear removes it. Note:
- [ ] **UT-B08 Flag filter.** As a photographer I can filter by pick, reject, and unflagged. Pass: each state shows only those photos. Note:
- [ ] **UT-B09 Colour filter.** As a photographer I can filter by a colour label. Pass: the chip uses the label's swatch and name, not colour alone. Note:
- [ ] **UT-B10 Keyword filter.** As a photographer I can filter by a keyword and switch match-any and match-all. Pass: two keywords behave differently under the two modes. Note:
- [ ] **UT-B11 Kind filter.** As a photographer I can show photos, raw, or video. Pass: the grid matches the chip. Note:
- [ ] **UT-B13 Filter badge.** As a photographer I can hide the filter strip and see how many filters are on. Pass: the badge count matches the active chips, and Clear every filter resets the grid. Note:
- [ ] **UT-B15 Selection.** As a photographer I can click, Ctrl-click, Shift-click, select all, and clear the selection. Pass: the count in the bar matches the highlighted cells. Note:
- [ ] **UT-B17 Menus.** As a photographer I can right-click a photo, a folder, a collection, and a keyword. Pass: each menu does what its row says (reveal, open, rename, delete, colour) and a destructive row asks first. Note:
- [ ] **UT-B18 Detail.** As a photographer I can open a photo and step previous and next. Pass: the position reads `{n} of {total}`, Fit and 1:1 change the zoom, and Back returns to the grid with the selection kept. Note:
- [ ] **UT-B21 Details tab.** As a photographer I can read kind, folder, size, camera, lens, exposure, date, rating, flag, label, and status. Pass: an empty value says so, and a missing file is named. Note:
- [ ] **UT-B22 Reveal and open.** As a photographer I can show a photo in its folder and open it in the default app. Pass: the file manager or the app opens on that file. Note:
- [ ] **UT-B23 Shortcuts.** As a photographer I can press `?` and see the overlay, then rebind one shortcut in Settings. Pass: the overlay and the command palette show the new chord. Note:
- [ ] **UT-B24 Export.** As a photographer I can export the selection as originals, JPEG, and PNG. Pass: the dialog states the spec, the files land in the chosen folder, and the receipt counts them. Use a copy folder. Note:

### Cull and compare

- [ ] **UT-C01 Cull screen.** As a photographer I can start a cull and see one photo, the position, and the decision buttons. Pass: an empty library explains how to start, and a library with photos shows the first frame. Note:
- [ ] **UT-C02 Keyboard decisions.** As a photographer I can pick, reject, defer, and undo from the keyboard. Pass: the session counts change and the file is not deleted yet. Note:
- [ ] **UT-C03 Gestures.** As a photographer I can swipe a decision. Pass: the same decision as the key, with the overlay naming it. Note:
- [ ] **UT-C06 Up next.** As a photographer I can see the upcoming frames. Pass: the strip follows the session order. Note:
- [ ] **UT-C07 Resume.** As a photographer I can leave a cull and come back to the same photo. Pass: relaunch restores the cursor and the unapplied decisions. Note:
- [ ] **UT-C10 Apply.** As a photographer I can apply rejects and read the count, the size, and the warning before anything is removed. Pass: cancel leaves the files, and confirm moves only the rejects. Use a copy. Note:
- [ ] **UT-C11 Tag while culling.** As a photographer I can add a keyword without leaving the cull. Pass: the keyword is on that photo when I return to the grid. Note:
- [ ] **UT-C12 Last keyword.** As a photographer I can apply the previous keyword with one key. Pass: the notice names the keyword and the count. Note:
- [ ] **UT-C13 Compare.** As a photographer I can compare two photos side by side. Pass: the facts row names stars, label, exposure, length, size, and pixels, and Swap exchanges the sides. Note:
- [ ] **UT-C17 Summary.** As a photographer I can open the session summary. Pass: kept, rejected, and deferred match the decisions I made. Note:

### Annotations

- [ ] **UT-D01 Stars.** As a photographer I can set 0–5 stars on one photo and on a selection. Pass: the bar, the inspector, and a filter agree, and pressing the same star again clears it. Note:
- [ ] **UT-D02 Colour labels.** As a photographer I can set and clear each of the five colours. Pass: the chip, the menu, and the inspector use one name. Note:
- [ ] **UT-D03 Flags.** As a photographer I can pick, unflag, and reject. Pass: the flag is independent of the stars. Note:
- [ ] **UT-D04 Keywords.** As a photographer I can add a keyword from the inspector, see it suggested from ones I have used, and remove it from the chip. Pass: the hover says where the keyword came from, and the count updates. Note:
- [ ] **UT-D05 Batch keywords.** As a photographer I can keyword several selected photos at once. Pass: the notice counts the photos that changed. Note:
- [ ] **UT-D06 Rename and delete a keyword.** As a photographer I can rename a keyword and delete it everywhere. Pass: the photos show the new name, and delete asks before removing it. Note:
- [ ] **UT-D08 Sidecars.** As a photographer I can turn sidecars on and see a rating written beside the file. Pass: the sidecar appears on disk and Settings says what it holds. Note:
- [ ] **UT-D10 Undo.** As a photographer I can undo and redo a rating, a keyword, and a move. Pass: each step restores the previous state, and an empty stack says there is nothing to undo. Note:
- [ ] **UT-D12 Keyword source.** As a photographer I can tell a typed keyword from an accepted suggestion and from one read out of a sidecar. Pass: the chip marks the non-manual sources and the filter can select one source. Note:

### Similar, places, insights, memories, soft focus

- [ ] **UT-E01 Soft focus.** As a photographer I can measure sharpness and see a histogram of scores. Pass: the button reports progress, an empty library explains itself, and the softest slice can be selected. Note:
- [ ] **UT-E03 Similar.** As a photographer I can find similar photos and switch structure, colour, and either. Pass: groups show a count, leftovers can be selected, and delete asks with the count. Use a copy. Note:
- [ ] **UT-E06 Places.** As a photographer I can open the map, see photos that have a location, and click a cluster. Pass: the notice names the count and the place, and a library with no locations shows the empty state. Note:
- [ ] **UT-E08 Insights.** As a photographer I can open Insights and read the figures, the columns, and the highlights. Pass: Recount updates the page, and an empty library says nothing is dated or rated yet. Note:
- [ ] **UT-E12 For You.** As a photographer I can open For You and read a memory card. Pass: a card names a count and a date, and an empty library shows the empty state. Note:

### Suggestions

- [ ] **UT-F06 Suggestions.** As a photographer I can open Suggestions for one photo and accept or dismiss a proposal. Pass: accept writes a keyword marked as suggested, dismiss removes the row, and nothing is written before I accept. Note:
- [ ] **UT-F07 Packs.** As a photographer I can see installed keyword packs and turn one on or off. Pass: an empty pack list says where a pack file goes. Note:
- [ ] **UT-F09 Suggest the library.** As a photographer I can start a suggestion pass, watch it in Activity, and stop it. Pass: stop leaves already-accepted keywords in place and writes no new ones. Note:

### Collections and moves

- [ ] **UT-G01 Collections.** As a photographer I can create, rename, colour, favourite, and delete a collection. Pass: delete asks first and does not move any files. Note:
- [ ] **UT-G02 Membership.** As a photographer I can add and remove the selection from a collection. Pass: the collection's count matches the photos inside it. Note:
- [ ] **UT-G07 Move.** As a photographer I can move picks into a folder and undo the move. Pass: the receipt names the count and the folder, and undo puts the files back. Use a copy. Note:

### Safety

- [ ] **UT-H01 Delete.** As a photographer I can delete a photo to the trash and see a confirmation that names the count. Pass: cancel deletes nothing. Confirm removes the photo from the grid and leaves an undo. Use a copy. Note:
- [ ] **UT-H05 Backup.** As a photographer I can back up the library and read what the backup leaves out. Pass: the note says it is not a copy of the photos, and the file is written. Note:
- [ ] **UT-H07 Health.** As a photographer I can run the library health check. Pass: the dialog reports the result in words, not a raw error code. Note:
- [ ] **UT-H08 Quit guard.** As a photographer I can quit with an unapplied cull or a running job. Pass: the dialog names the unfinished work and lets me stay. Note:

### Settings and the command palette

- [ ] **UT-I01 Settings.** As a photographer I can open Settings and reach Appearance, library, AI models, updates, shortcuts, and sync. Pass: each section shows its own controls and a closed dialog leaves the window as it was. Note:
- [ ] **UT-I04 Themes.** As a photographer I can cycle Charcoal, Midnight, Silver, Paper, and High Contrast. Pass: the choice survives a relaunch and the window stays readable in each. Note:
- [ ] **UT-I07 Language.** As a photographer I can switch the interface to Arabic and to Japanese. Pass: the sidebar, the canvas header, and a dialog change language immediately, Arabic lays out right to left, and switching back to English restores the English window. Note:
- [ ] **UT-I09 Shortcut editor.** As a photographer I can open the shortcut editor, search a command, and close it without saving a conflict. Pass: a conflict is named, and Esc returns to the library. Note:
- [ ] **UT-I10 Updates.** As a photographer I can read the update check and run it only after agreeing. Pass: the panel states that it makes one request, and Check now changes to Checking. Note:
- [ ] **UT-I11 About.** As a photographer I can open About and read the version and the credits. Pass: the product name is Sortlens and the credits are readable. Note:
- [ ] **UT-I14 Diagnostics.** As a photographer I can copy a support report. Pass: the report contains no absolute photo path. Note:
- [ ] **UT-K Palette.** As a photographer I can press Ctrl+K, type a command, and run it. Pass: the list groups commands, the empty search says nothing matches, and Esc closes it. Note:

### Phone width and sync

These are exercised in a window sized under 600 points, on this machine. A story that needs a handset stays blocked.

- [ ] **UT-J-width Phone layout.** As a photographer I can narrow the window under 600 points. Pass: one dock opens as a full-width sheet, the grid is usable, the filter strip is two rows, and every bottom-bar control is on screen. Note:
- [ ] **UT-J03 Sync on the desktop.** As a photographer I can start the server from Sync & devices and read the address and the code. Pass: stop clears the live code, and the section is not shown in the phone-width window. Note:
- [ ] **UT-J04 Send.** As a photographer on a phone-width window I can see the unpaired send form: address, port, test connection, and the code field. Pass: an unreachable address says why, and the desktop window does not show this section. Note:
- [ ] **UT-J10 Device.** A run on a handset, the iOS Photos library, an `.ipa`, and a Play bundle. Blocked until a machine has the Android NDK and Xcode, and until the owner adds an Apple identity. Note:

## Interface stories

Start these only after the feature stories above are checked or blocked. The reference is
`docs/UX.md`: five surfaces, the 1440×900 wireframe, and the phone-width rules. A story fails
when a person would hesitate, mis-click, or read one control as two different things.

- [ ] **UT-U01 Top bar.** At 1440×900 the mark, the search field, and the glyphs sit on one row. Nothing overlaps the field, a glyph has a hover name, and the drag region still moves the window. Note:
- [ ] **UT-U02 Narrow top bar.** At 900 points wide with both docks open, the canvas title does not run under the sort control, and the search field does not clip a word in half. Note:
- [ ] **UT-U03 Sidebar.** Section headers, rows, counts, and the utilities foot do not overlap. A long folder name truncates. Counts stay inside the panel, including in Arabic. Note:
- [ ] **UT-U04 Short sidebar.** Below about 620 points tall, the keyword and collection rows scroll and the foot stays put. A row under the fold can be reached. Note:
- [ ] **UT-U05 Filter strip.** Chips wrap. The shown-count never sits on top of Clear. An active chip looks selected, and colour is never the only signal. Note:
- [ ] **UT-U06 Empty states.** A library with no photos shows the welcome card. A filter that hides everything shows the empty-filter screen and keeps the strip. The two screens are not mixed. Note:
- [ ] **UT-U07 Grid chrome.** Selection and focus rings are visible on Charcoal and on Paper. Badges sit inside the cell. The last row is not covered by the bottom bar. Note:
- [ ] **UT-U08 Inspector.** Detail rows align to one column. A long path truncates. The keyword chips wrap and the remove mark is on the chip, not floating. Note:
- [ ] **UT-U09 Bottom bar.** Destinations fit or collapse to the menu at the width `docs/UX.md` names. The annotation pill does not run off the window. The current destination is named once. Note:
- [ ] **UT-U10 Phone width.** At 390×780 the sheet is the full width, the canvas is not squeezed beside it, and no control is drawn past the edge. Note:
- [ ] **UT-U11 Dialogs.** Export, delete, rename, settings, and about open centred, trap Escape as cancel, and do not leave a dim layer behind after close. Note:
- [ ] **UT-U12 Themes.** Charcoal, Midnight, Silver, Paper, and High Contrast share one layout. Text on each theme can be read. High Contrast is not just Charcoal with a brighter accent. Note:
- [ ] **UT-U13 Motion.** Reduced motion removes the cull slide. The Appearance menu tells the truth about which setting is on. Note:
- [ ] **UT-U14 Notices.** A toast is readable, does not cover the button that raised it, and the bell lists it in the same words. Note:
- [ ] **UT-U15 Icons.** Actions that `docs/UX.md` assigns a glyph use that glyph. A text bullet is not standing in for an icon. Note:
- [ ] **UT-U16 Commands.** The command palette, the sidebar, and a context menu use one name for the same command. A planned command says when it arrives instead of failing silently. Note:
- [ ] **UT-U17 Language.** In Japanese and in Arabic, a wrapped label stays inside its control, a count uses that language's digits, and English is left only for a name, a key cap, or a path. Note:
- [ ] **UT-U18 Focus.** Keyboard focus is visible, and Tab does not land on a control hidden under a closed dock or a closed dialog. Note:

## Log

Append one line per finished or blocked story. Do not rewrite history.

| Date | Story | Result |
| --- | --- | --- |
