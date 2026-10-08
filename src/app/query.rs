//! What the view asks of the file: which options are visible, how many are
//! set, whether anything changed.
//!
//! These are pure functions of the file and the current filter, so they are
//! testable without a view. The rules they carry — a repeatable key counts as
//! set when any of its lines has a value, and a search matches either language
//! — used to be reachable only through an `Entity<SettingsView>`.

use crate::config::linefile::LineFile;
use crate::config::schema::{CATEGORIES, OPTS, Opt, lookup};
use crate::i18n::Lang;

/// Whether the file differs from what was last read from disk.
pub fn dirty(file: &LineFile, original: &str) -> bool {
    file.render() != original
}

/// Whether an option has a value in the file.
///
/// A repeatable key is set when any of its lines carries a value: `font-family`
/// with one empty line and one real font is configured, not empty.
pub fn is_set(file: &LineFile, opt: &Opt) -> bool {
    if opt.repeatable() {
        file.get_all(opt.key).iter().any(|value| !value.is_empty())
    } else {
        file.get(opt.key).is_some()
    }
}

/// The options the list shows: a category's keys, or every match for a search.
///
/// A search looks at the key and at every language's label and documentation,
/// so a Korean user who knows the English term (or the reverse) still finds
/// the option.
pub fn visible(search: &str, category: usize) -> Vec<&'static Opt> {
    let query = search.trim().to_lowercase();
    if query.is_empty() {
        return CATEGORIES[category]
            .keys
            .iter()
            .filter_map(|key| lookup(key))
            .collect();
    }
    OPTS.iter()
        .filter(|opt| {
            opt.key.contains(&query)
                || Lang::ALL.iter().any(|lang| {
                    opt.label.get(*lang).to_lowercase().contains(&query)
                        || opt.doc.get(*lang).to_lowercase().contains(&query)
                })
        })
        .collect()
}

/// How many of the visible options are set.
pub fn set_count(file: &LineFile, search: &str, category: usize) -> usize {
    visible(search, category)
        .iter()
        .filter(|opt| is_set(file, opt))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(lines: &[&str]) -> LineFile {
        LineFile::parse(&lines.join("\n"))
    }

    #[test]
    fn a_repeatable_key_is_set_when_any_line_has_a_value() {
        let opt = lookup("font-family").expect("font-family is in the schema");
        assert!(is_set(&file(&["font-family = ", "font-family = Menlo"]), opt));
        assert!(!is_set(&file(&["font-family = "]), opt));
        assert!(!is_set(&file(&[]), opt));
    }

    #[test]
    fn a_single_key_is_set_when_it_appears() {
        let opt = lookup("font-size").expect("font-size is in the schema");
        assert!(is_set(&file(&["font-size = 14"]), opt));
        assert!(!is_set(&file(&["theme = nord"]), opt));
    }

    #[test]
    fn an_unfiltered_list_is_the_category() {
        let shown = visible("", 0);
        assert_eq!(shown.len(), CATEGORIES[0].keys.len());
        assert_eq!(shown[0].key, CATEGORIES[0].keys[0]);
    }

    /// The same option answers to both languages, in either mode.
    #[test]
    fn search_matches_every_language() {
        for query in ["font-family", "글꼴", "Font", "families"] {
            let shown = visible(query, 0);
            assert!(
                shown.iter().any(|opt| opt.key == "font-family"),
                "searching {query:?} must find font-family"
            );
        }
        assert!(visible("definitely-not-an-option", 0).is_empty());
    }

    #[test]
    fn the_count_follows_the_file_and_the_filter() {
        let configured = file(&["font-size = 14", "font-family = Menlo"]);
        assert_eq!(set_count(&configured, "", 0), 2);
        assert_eq!(set_count(&configured, "font-size", 0), 1);
        assert_eq!(set_count(&file(&[]), "", 0), 0);
    }

    #[test]
    fn dirty_tracks_the_last_read() {
        let file = file(&["font-size = 14"]);
        let original = file.render();
        assert!(!dirty(&file, &original));

        let mut changed = file.clone();
        changed.set("font-size", "15");
        assert!(dirty(&changed, &original));
    }
}
