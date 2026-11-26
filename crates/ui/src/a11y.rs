//! Screen-reader announcements.
//!
//! egui exposes its accessibility tree through AccessKit but has no "say this now" call, so the way
//! to announce something is to render it as a label marked as a polite live region. Assistive
//! technology then reads it when it changes, without interrupting whatever is being read.
//!
//! The sentences live in one place so the spoken text and the visible text cannot drift apart.

use egui::{Color32, RichText, Ui};

use sortlens_engine::Decision;

use crate::i18n::{self, tl};

/// What a decision is called when spoken.
///
/// The word is a catalogue row like any painted label: a screen reader is a reader, and an Arabic
/// window that announced "picked" in English would be two interfaces in one frame.
pub fn decision_word(decision: Decision) -> String {
    match decision {
        Decision::Pick => tl!("picked"),
        Decision::Reject => tl!("rejected"),
        Decision::Deferred => tl!("deferred"),
        Decision::Unjudged => tl!("cleared"),
    }
    .to_string()
}

/// The cull status line: where you are and what the photo carries.
///
/// `position` is one-based. An empty queue says so rather than "0 of 0". The numbers are the
/// reader's digits and the sentence is the reader's row, so a Japanese status reads
/// `12枚中3枚目、採用` where it read `Photo 3 of 12, picked`.
pub fn cull_status(position: usize, total: usize, decision: Decision) -> String {
    if total == 0 {
        return tl!("No photos to cull").to_string();
    }
    let position = i18n::count(position as u64);
    let total = i18n::count(total as u64);
    match decision {
        Decision::Unjudged => crate::i18n::fmt("Photo {position} of {total}", &[("position", &position), ("total", &total)]),
        other => crate::i18n::fmt(
            "Photo {position} of {total}, {word}",
            &[("position", &position), ("total", &total), ("word", &decision_word(other))],
        ),
    }
}

/// One line of spoken feedback after a cull decision.
///
/// One photo and several are two English rows, the way the rest of the shell words a count, so a
/// language is not forced through English's singular and plural; the decision's own word stays the
/// value the row places, capitalized for English's sentence position (the other five catalogues
/// have no case, so a capital there is the translator's word unchanged).
pub fn decision_sentence(decision: Decision, remaining: usize) -> String {
    let n = i18n::count(remaining as u64);
    if decision == Decision::Unjudged {
        return if remaining == 1 {
            crate::i18n::fmt("Decision cleared, {n} photo left", &[("n", &n)])
        } else {
            crate::i18n::fmt("Decision cleared, {n} photos left", &[("n", &n)])
        };
    }
    let word = capitalize(&decision_word(decision));
    if remaining == 1 {
        crate::i18n::fmt("{word}, {n} photo left", &[("word", &word), ("n", &n)])
    } else {
        crate::i18n::fmt("{word}, {n} photos left", &[("word", &word), ("n", &n)])
    }
}

/// Draw a polite live region, so a change is read out but never interrupts.
///
/// The text is visible too: a status line that only exists for assistive technology would be a
/// second, invisible UI that nobody else can check.
pub fn live_line(ui: &mut Ui, text: &str, colour: Color32) -> egui::Response {
    let response = ui.add(egui::Label::new(RichText::new(text).size(10.5).color(colour)).selectable(false));
    let spoken = text.to_string();
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_live(egui::accesskit::Live::Polite);
        node.set_label(spoken);
    });
    response
}

/// Give a custom-drawn, clickable region a name, a role and a selected state.
///
/// The grid, the cull strip and the contact sheets paint their own cells, which means egui knows nothing
/// about them: no label, no role, nothing for a screen reader to land on. A cell that can be clicked and
/// cannot be described is a control that does not exist for anyone using assistive technology, so every
/// custom region registers itself here.
pub fn label_widget(ui: &egui::Ui, response: &egui::Response, kind: egui::Role, label: &str, selected: bool) {
    // `WidgetInfo` is what makes egui give the widget a node at all: a plain `allocate_rect` reports
    // nothing, so a node built for it would be discarded. This is the difference between an accessible
    // grid and a grid that looks accessible in the source.
    let label = label.to_string();
    // `widget_info` is only called when the widget asks for it, so it takes a closure rather than a value.
    response.widget_info(move || egui::WidgetInfo::labeled(kind, true, label.clone()));
    if selected {
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_selected(true);
        });
    }
}

/// Give a text field the name it is read by.
///
/// A `TextEdit` publishes its role and its text, but no name unless a visible label is bound to it,
/// and the shell's one field paints its question as a hint instead — a hint screen readers may never
/// announce. The palette's field and the photo search therefore took no name at all. This sets the
/// same sentence the hint paints, so the field a reader meets and the one a sighted user sees cannot
/// describe different jobs.
pub fn name_field(ui: &egui::Ui, response: &egui::Response, label: &str) {
    let label = label.to_string();
    // The node already exists — `TextEdit` registered it — so this names it without replacing the
    // role, the text or the placeholder egui set. Setting it here rather than through
    // `widget_info` also survives an edit frame, where `widget_info` emits an event instead.
    ui.ctx().accesskit_node_builder(response.id, |node| node.set_label(label));
}

/// What one photo is called when read aloud: its name, then the facts that decide a cull.
///
/// Written as a sentence rather than a list of fields because it is spoken, and ordered by what matters
/// first: which photo, how good, whether it is flagged, and what it is tagged with.
#[must_use]
pub fn describe_photo(photo: &sortlens_engine::Photo) -> String {
    let mut parts = vec![photo.name.clone()];
    if photo.rating > 0 {
        let n = i18n::count(u64::from(photo.rating));
        parts.push(if photo.rating == 1 {
            crate::i18n::fmt("{n} star", &[("n", &n)])
        } else {
            crate::i18n::fmt("{n} stars", &[("n", &n)])
        });
    }
    match photo.flag {
        sortlens_core::Flag::Pick => parts.push(tl!("pick").to_string()),
        sortlens_core::Flag::Reject => parts.push(tl!("reject").to_string()),
        sortlens_core::Flag::None => {}
    }
    if let Some(label) = photo.label {
        parts.push(crate::i18n::fmt("{colour} label", &[("colour", &crate::shell::colour_word(label))]));
    }
    if photo.missing {
        parts.push(tl!("file missing").to_string());
    }
    if !photo.duration_label().is_empty() {
        parts.push(photo.duration_label());
    }
    if !photo.tags.is_empty() {
        parts.push(photo.tags.join(", "));
    }
    parts.join(", ")
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_status_names_the_position_and_the_decision() {
        assert_eq!(cull_status(3, 12, Decision::Pick), "Photo 3 of 12, picked");
        assert_eq!(cull_status(1, 1, Decision::Reject), "Photo 1 of 1, rejected");
        assert_eq!(cull_status(2, 5, Decision::Unjudged), "Photo 2 of 5");
        assert_eq!(cull_status(0, 0, Decision::Unjudged), "No photos to cull");
    }

    #[test]
    fn a_decision_says_what_is_left() {
        assert_eq!(decision_sentence(Decision::Pick, 1), "Picked, 1 photo left");
        assert_eq!(decision_sentence(Decision::Reject, 4), "Rejected, 4 photos left");
        assert_eq!(decision_sentence(Decision::Deferred, 0), "Deferred, 0 photos left");
        assert_eq!(decision_sentence(Decision::Unjudged, 2), "Decision cleared, 2 photos left");
    }

    /// A named region paints, and the name survives being asked for twice in one frame.
    ///
    /// What this does *not* prove is that egui put the label in the accessibility tree: this version has no
    /// public way to read the tree back, so `cargo xtask a11y` checks the coverage instead — every
    /// custom-painted clickable region in the shell must register one — and the names themselves are
    /// tested as the pure functions above. A test that claimed to read the tree would be a lie.
    #[test]
    fn a_painted_region_takes_a_name_without_complaining() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut actions = 0;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let (_, response) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::click());
                label_widget(ui, &response, egui::Role::Button, "DSC_0001.jpg, 3 stars", true);
                if response.clicked() {
                    actions += 1;
                }
            });
        });
        output.textures_delta.clear();
        assert_eq!(actions, 0, "naming a region does not click it");
        assert!(!output.shapes.is_empty());
    }

    /// A text field takes the name it is given and keeps the text it holds: naming it must not
    /// turn a text box into an unnamed or emptied control.
    #[test]
    fn a_text_field_takes_its_name_and_keeps_its_text() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut text = String::from("beach");
        let mut seen: Option<Option<(Option<String>, Option<String>)>> = None;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let field = ui.text_edit_singleline(&mut text);
                name_field(ui, &field, "Type a command…");
                seen = Some(
                    ui.ctx()
                        .accesskit_node_builder(field.id, |node| (node.label().map(str::to_owned), node.value().map(str::to_owned))),
                );
            });
        });
        output.textures_delta.clear();
        let (label, value) = seen.flatten().expect("the field's node exists");
        assert_eq!(label.as_deref(), Some("Type a command…"), "the field is named");
        assert_eq!(value.as_deref(), Some("beach"), "and its text is still what it holds");
    }

    #[test]
    fn a_photo_is_described_by_name_then_the_facts_that_decide_a_cull() {
        let mut photo = test_photo("DSC_0001.jpg");
        assert_eq!(describe_photo(&photo), "DSC_0001.jpg");
        photo.rating = 1;
        assert_eq!(describe_photo(&photo), "DSC_0001.jpg, 1 star");
        photo.rating = 4;
        photo.flag = sortlens_core::Flag::Pick;
        photo.tags = vec!["harbour".to_string(), "night".to_string()];
        assert_eq!(describe_photo(&photo), "DSC_0001.jpg, 4 stars, pick, harbour, night");
        photo.flag = sortlens_core::Flag::Reject;
        photo.missing = true;
        assert_eq!(
            describe_photo(&photo),
            "DSC_0001.jpg, 4 stars, reject, file missing, harbour, night"
        );
        photo.rating = 0;
        photo.flag = sortlens_core::Flag::None;
        photo.tags.clear();
        photo.label = Some(sortlens_core::ColorLabel::Green);
        assert!(describe_photo(&photo).contains("label"));

        // A clip says how long it is, after the facts that decide a cull and before its tags.
        photo.label = None;
        photo.missing = false;
        photo.kind = sortlens_core::MediaKind::Video;
        photo.duration_ms = Some(72_000);
        assert_eq!(describe_photo(&photo), "DSC_0001.jpg, 1:12");
        photo.tags = vec!["harbour".to_string()];
        assert_eq!(describe_photo(&photo), "DSC_0001.jpg, 1:12, harbour");
        photo.tags.clear();
        photo.duration_ms = None;
        assert_eq!(describe_photo(&photo), "DSC_0001.jpg", "a clip with no known length says nothing");
    }

    fn test_photo(name: &str) -> sortlens_engine::Photo {
        sortlens_engine::Photo {
            id: sortlens_core::PhotoId(1),
            root: "/photos".into(),
            rel: name.into(),
            name: name.into(),
            size: 1,
            modified: None,
            kind: sortlens_core::MediaKind::Image,
            duration_ms: None,
            fps: None,
            video_codec: None,
            key: 1,
            rating: 0,
            flag: sortlens_core::Flag::None,
            label: None,
            missing: false,
            width: 0,
            height: 0,
            captured: None,
            camera: String::new(),
            lens: String::new(),
            iso: None,
            aperture: None,
            shutter: None,
            focal: None,
            tags: Vec::new(),
            tag_sources: std::collections::BTreeMap::new(),
            dismissed_suggestions: std::collections::BTreeSet::new(),
            edited: false,
            retry: false,
            sharpness: None,
            location: None,
            rev: 0,
            added: None,
        }
    }

    /// The spoken sentences are catalogue rows, in every language the picker offers.
    ///
    /// A screen reader is a reader: an Arabic window that announced "picked" in English would be
    /// two interfaces in one frame. The painted half of this file is read back to prove each
    /// sentence routes through `tl!` or `fmt` instead of carrying the literal, and the catalogue
    /// is the witness that every row a translator was handed is there. English is held byte for
    /// byte (routing a sentence must not reword the screen), Arabic for the reader's own digits and
    /// no English left behind, and Japanese for its own word order.
    #[test]
    fn the_spoken_sentences_speak_the_interface_language() {
        // The module's route markers, built from pieces so this test's own source does not read as
        // a route the catalogue audit would have to find a row for.
        let markers = [concat!("t", "l!("), concat!("f", "m", "t(")];
        // Every English key the spoken sinks use, with the Arabic row it must resolve to.
        const ROWS: [(&str, &str); 14] = [
            ("picked", "منتقاة"),
            ("rejected", "مرفوضة"),
            ("deferred", "مؤجلة"),
            ("cleared", "ممسوحة"),
            ("No photos to cull", "لا صور للفرز"),
            ("Photo {position} of {total}", "صورة {position} من {total}"),
            ("Photo {position} of {total}, {word}", "صورة {position} من {total}، {word}"),
            ("{word}, {n} photo left", "{word}، بقيت {n} صورة واحدة"),
            ("{word}, {n} photos left", "{word}، بقيت {n} صور"),
            ("Decision cleared, {n} photo left", "أُزيل القرار، بقيت {n} صورة واحدة"),
            ("Decision cleared, {n} photos left", "أُزيل القرار، بقيت {n} صور"),
            ("{n} star", "{n} نجمة"),
            ("{n} stars", "{n} نجوم"),
            ("file missing", "الملف مفقود"),
        ];
        // The source with each line's own indentation trimmed away, so a wrapped call still reads
        // as one call; the test module is cut off so the assertion is about the sinks, not itself.
        let source: String = include_str!("a11y.rs").lines().map(str::trim).collect();
        let paint = source.split("#[cfg(test)]").next().unwrap_or(&source);
        for (english, _) in ROWS {
            let routed = markers.iter().any(|marker| paint.contains(&format!("{marker}\"{english}\"")));
            assert!(routed, "the spoken row {english:?} is not routed through the catalogues");
        }
        // English is unchanged, and Arabic is the translator's row rather than the English fallback.
        crate::i18n::set_current(crate::i18n::from_code("en").expect("English is offered"));
        for (english, _) in ROWS {
            assert_eq!(crate::i18n::tr(english), english, "English keeps the key it speaks");
        }
        crate::i18n::set_current(crate::i18n::from_code("ar").expect("Arabic is offered"));
        for (english, arabic) in ROWS {
            assert_eq!(crate::i18n::tr(english), arabic, "the spoken row for {english:?} is Arabic");
        }
        // The sentences themselves: the reader's digits, no English word left behind, and the
        // decision's own row where English puts a capitalized word.
        let status = cull_status(3, 12, Decision::Pick);
        assert!(status.contains("صورة") && status.contains("٣") && status.contains("١٢"), "{status}");
        assert!(!status.contains("Photo") && !status.contains("picked"), "{status}");
        let one = decision_sentence(Decision::Pick, 1);
        assert!(one.contains("منتقاة") && one.contains("١") && !one.contains("photo"), "{one}");
        let several = decision_sentence(Decision::Reject, 4);
        assert!(
            several.contains("مرفوضة") && several.contains("٤") && !several.contains("Rejected"),
            "{several}"
        );
        let cleared = decision_sentence(Decision::Unjudged, 2);
        assert!(cleared.contains("أُزيل القرار") && cleared.contains("٢"), "{cleared}");
        assert!(!cleared.contains("Decision"), "{cleared}");
        let mut photo = test_photo("DSC_0001.jpg");
        photo.rating = 4;
        photo.flag = sortlens_core::Flag::Pick;
        photo.label = Some(sortlens_core::ColorLabel::Green);
        photo.missing = true;
        let spoken = describe_photo(&photo);
        assert!(spoken.contains("٤") && spoken.contains("نجوم"), "{spoken}");
        assert!(spoken.contains("مختارة") && spoken.contains("أخضر"), "{spoken}");
        assert!(spoken.contains("الملف مفقود"), "{spoken}");
        assert!(
            !spoken.contains("stars") && !spoken.contains("label") && !spoken.contains("Green"),
            "{spoken}"
        );
        assert!(!spoken.contains("file missing"), "{spoken}");
        // Japanese writes its own count and word order, not an English sentence reordered by eye.
        crate::i18n::set_current(crate::i18n::from_code("ja").expect("Japanese is offered"));
        let japanese = cull_status(3, 12, Decision::Deferred);
        assert!(japanese.contains("枚中3枚目") && japanese.contains("保留"), "{japanese}");
        crate::i18n::set_current(crate::i18n::from_code("en").expect("English is offered"));
    }

    #[test]
    fn every_decision_has_a_word() {
        for decision in [Decision::Pick, Decision::Reject, Decision::Deferred, Decision::Unjudged] {
            assert!(!decision_word(decision).is_empty());
        }
    }
}
