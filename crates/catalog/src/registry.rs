//! The keyword registry: one place that knows every keyword, what else it is called, and what is
//! wrong with it.
//!
//! Keywords are just strings on photos, which is what makes them easy to add and easy to end up with
//! three spellings of the same idea. The registry is the view that makes that visible: for each
//! keyword, how many photos carry it, which older names now resolve to it, and which aliases point at
//! a keyword nothing carries any more.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One keyword in the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The keyword as it is written on photos.
    pub name: String,
    /// How many photos carry it.
    pub count: usize,
    /// Older names that now resolve to this one.
    pub aliases: Vec<String>,
}

impl Entry {
    /// Whether the keyword has been renamed into this one from elsewhere.
    pub fn is_merged(&self) -> bool {
        !self.aliases.is_empty()
    }
}

/// The whole registry.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registry {
    /// Keywords in use, alphabetically.
    pub entries: Vec<Entry>,
    /// Aliases whose target nobody carries.
    pub orphans: Vec<(String, String)>,
}

impl Registry {
    /// Build it from a vocabulary, the alias map and the orphans.
    pub fn of(vocabulary: Vec<(String, usize)>, aliases: &BTreeMap<String, String>, orphans: Vec<(String, String)>) -> Self {
        let entries = vocabulary
            .into_iter()
            .map(|(name, count)| {
                let mut names: Vec<String> = aliases
                    .iter()
                    .filter(|(_, canonical)| canonical.as_str() == name)
                    .map(|(alias, _)| alias.clone())
                    .collect();
                names.sort();
                Entry {
                    name,
                    count,
                    aliases: names,
                }
            })
            .collect();
        Self { entries, orphans }
    }

    /// How many keywords are in use.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are no keywords at all.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many keywords were renamed into another one.
    pub fn merged(&self) -> usize {
        self.entries.iter().filter(|entry| entry.is_merged()).count()
    }

    /// One line for a header or a CLI.
    pub fn headline(&self) -> String {
        if self.is_empty() {
            return "No keywords yet".to_string();
        }
        let mut line = format!("{} keyword(s)", self.entries.len());
        if self.merged() > 0 {
            line.push_str(&format!(" · {} renamed into another", self.merged()));
        }
        if !self.orphans.is_empty() {
            line.push_str(&format!(" · {} alias(es) pointing nowhere", self.orphans.len()));
        }
        line
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn aliases(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(alias, canonical)| (alias.to_string(), canonical.to_string()))
            .collect()
    }

    #[test]
    fn a_keyword_lists_the_names_it_swallowed() {
        let map = aliases(&[("strand", "beach"), ("seaside", "beach"), ("shore", "coast")]);
        let registry = Registry::of(vec![("beach".to_string(), 12), ("coast".to_string(), 3)], &map, Vec::new());
        assert_eq!(registry.len(), 2);
        assert_eq!(registry.merged(), 2);
        let beach = &registry.entries[0];
        assert_eq!(beach.aliases, vec!["seaside".to_string(), "strand".to_string()]);
        assert_eq!(beach.count, 12);
        assert!(registry.headline().contains("2 renamed"), "{}", registry.headline());
    }

    #[test]
    fn orphans_and_an_empty_registry_read_plainly() {
        let empty = Registry::default();
        assert!(empty.is_empty());
        assert_eq!(empty.headline(), "No keywords yet");

        let registry = Registry::of(
            vec![("beach".to_string(), 1)],
            &aliases(&[("gone", "nobody")]),
            vec![("gone".to_string(), "nobody".to_string())],
        );
        assert_eq!(registry.orphans.len(), 1);
        assert!(registry.headline().contains("pointing nowhere"), "{}", registry.headline());
    }
}
