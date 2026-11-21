//! Screen-reader announcements.
//!
//! egui exposes its accessibility tree through AccessKit but has no "say this now" call, so the way
//! to announce something is to render it as a label marked as a polite live region. Assistive
//! technology then reads it when it changes, without interrupting whatever is being read.
//!
//! The sentences live in one place so the spoken text and the visible text cannot drift apart.

use egui::{Color32, RichText, Ui};

use sortlens_engine::Decision;

/// What a decision is called when spoken.
pub const fn decision_word(decision: Decision) -> &'static str {
    match decision {
        Decision::Pick => "picked",
        Decision::Reject => "rejected",
        Decision::Deferred => "deferred",
        Decision::Unjudged => "cleared",
    }
}

/// The cull status line: where you are and what the photo carries.
///
/// `position` is one-based. An empty queue says so rather than "0 of 0".
pub fn cull_status(position: usize, total: usize, decision: Decision) -> String {
    if total == 0 {
        return "No photos to cull".to_string();
    }
    let decided = match decision {
        Decision::Unjudged => String::new(),
        other => format!(", {}", decision_word(other)),
    };
    format!("Photo {position} of {total}{decided}")
}

/// The library status line: how many photos are in view and how many are selected.
pub fn library_status(in_view: usize, selected: usize) -> String {
    let photos = if in_view == 1 { "photo" } else { "photos" };
    if selected == 0 {
        format!("{in_view} {photos}")
    } else {
        format!("{in_view} {photos}, {selected} selected")
    }
}

/// One line of spoken feedback after a cull decision.
pub fn decision_sentence(decision: Decision, remaining: usize) -> String {
    let noun = if remaining == 1 { "photo" } else { "photos" };
    match decision {
        Decision::Unjudged => format!("Decision cleared, {remaining} {noun} left"),
        other => format!("{}, {remaining} {noun} left", capitalize(decision_word(other))),
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

    #[test]
    fn the_library_line_counts_photos_and_selection() {
        assert_eq!(library_status(0, 0), "0 photos");
        assert_eq!(library_status(1, 0), "1 photo");
        assert_eq!(library_status(240, 3), "240 photos, 3 selected");
    }

    #[test]
    fn every_decision_has_a_word() {
        for decision in [Decision::Pick, Decision::Reject, Decision::Deferred, Decision::Unjudged] {
            assert!(!decision_word(decision).is_empty());
        }
    }
}
