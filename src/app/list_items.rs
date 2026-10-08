//! The list behind the list-editor modal, and the rules for changing it.
//!
//! Editing a repeatable option's list used to be arithmetic repeated at eight
//! call sites — trim, skip the blank, push — with the guard expressions
//! differing and, in one case, the rule itself differing: adding a font through
//! the dropdown allowed a duplicate while the quick-add chip refused it. The
//! list and its rules live here instead, so one call site is one call, and the
//! rules are assertable without a window.

/// A repeatable option's items, and what can be done to them.
///
/// The interface is the list and three operations. Everything a caller has to
/// know is in the operations: a blank or whitespace-only value is not an item,
/// and whether a value the list already holds may be added is the caller's
/// choice — `add` allows it, `add_unique` refuses it.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct ListItems(Vec<String>);

impl ListItems {
    /// An empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// The list as the file holds it.
    pub fn from_values(values: Vec<String>) -> Self {
        Self(values)
    }

    /// Add a value: trimmed, and skipped when the result is blank. Reports
    /// whether the list took it.
    pub fn add(&mut self, value: &str) -> bool {
        let value = value.trim();
        if value.is_empty() {
            return false;
        }
        self.0.push(value.to_string());
        true
    }

    /// Add a value the list does not already hold: trimmed, skipped when blank,
    /// and refused when an equal item is present. Reports whether the list
    /// took it.
    pub fn add_unique(&mut self, value: &str) -> bool {
        let value = value.trim();
        if value.is_empty() || self.0.iter().any(|item| item == value) {
            return false;
        }
        self.0.push(value.to_string());
        true
    }

    /// Remove the item at `ix`, if there is one.
    pub fn remove_at(&mut self, ix: usize) {
        if ix < self.0.len() {
            self.0.remove(ix);
        }
    }

    /// The items, in order.
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    /// Whether the list holds no items.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How many items the list holds.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_is_added_trimmed() {
        let mut items = ListItems::new();
        assert!(items.add("  Menlo  "));
        assert_eq!(items.as_slice(), ["Menlo"]);
    }

    #[test]
    fn a_blank_value_is_not_an_item() {
        let mut items = ListItems::new();
        assert!(!items.add("   "));
        assert!(items.is_empty());
    }

    #[test]
    fn adding_twice_is_allowed_unless_the_list_refuses_it() {
        let mut items = ListItems::new();
        items.add("Menlo");
        assert!(items.add("Menlo"), "add allows a duplicate");
        assert_eq!(items.len(), 2);

        let mut unique = ListItems::new();
        unique.add_unique("Menlo");
        assert!(!unique.add_unique("Menlo"), "add_unique refuses a duplicate");
        assert_eq!(unique.as_slice(), ["Menlo"]);
    }

    #[test]
    fn add_unique_trims_and_skips_blanks_like_add() {
        let mut items = ListItems::new();
        assert!(!items.add_unique("  "));
        assert!(items.add_unique("  cv01  "));
        assert_eq!(items.as_slice(), ["cv01"]);
    }

    #[test]
    fn removing_uses_the_position_and_owns_its_guard() {
        let mut items = ListItems::new();
        items.add("a");
        items.add("b");
        items.add("c");

        items.remove_at(1);
        assert_eq!(items.as_slice(), ["a", "c"]);

        items.remove_at(9);
        assert_eq!(items.as_slice(), ["a", "c"], "an out-of-range removal is a no-op");
    }

    #[test]
    fn the_round_trip_through_the_file_keeps_order() {
        let mut items = ListItems::from_values(vec!["one".into(), "two".into()]);
        assert_eq!(items.len(), 2);
        items.add("three");
        items.remove_at(0);
        let values = items.as_slice().to_vec();
        assert_eq!(values, ["two", "three"]);
    }
}
