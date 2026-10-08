//! What a row draws, as decisions over plain data.
//!
//! A row's editor asks three questions of the file and of its own candidate
//! values — is this chip the value the file holds, is this flag in the set the
//! file holds, what does this keybinding read as — and each answer used to be
//! an expression inside a closure that needed a live window to run. The rules
//! are pure; the window was never the point. They live here, as functions of
//! plain data, so a chip that stops lighting for a value the file holds fails
//! a test instead of only looking wrong.

/// Whether `target` is the value the file holds, for a chip's highlight.
///
/// The file's value is what it is, trimmed or not: a chip lights when it
/// carries exactly that value, so `font-family = Menlo` lights the Menlo chip
/// and no other.
pub(crate) fn chip_is_active(current: &str, target: &str) -> bool {
    current == target
}

/// Whether `flag` is in the set the file holds, for a flag chip's highlight.
pub(crate) fn flag_is_active(current: &[String], flag: &str) -> bool {
    current.iter().any(|f| f == flag)
}

/// How a keybinding line reads in a summary: its trigger, and the action it
/// runs. A line without `=` has an empty action.
pub(crate) fn binding_parts(line: &str) -> (&str, &str) {
    let mut parts = line.splitn(2, '=');
    let trigger = parts.next().unwrap_or(line);
    let action = parts.next().unwrap_or("");
    (trigger, action)
}

/// The keybindings a summary shows, and how many it leaves out.
///
/// A row shows the first two bindings and counts the rest; the count is a
/// decision over the list, not something the render blocks work out.
pub(crate) fn binding_summary(lines: &[String]) -> (Vec<(&str, &str)>, usize) {
    let shown: Vec<(&str, &str)> = lines.iter().take(2).map(|line| binding_parts(line)).collect();
    let more = lines.len().saturating_sub(shown.len());
    (shown, more)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chip_lights_only_for_the_value_the_file_holds() {
        assert!(chip_is_active("80", "80"));
        assert!(!chip_is_active("80", "100"));
        // A file value the chips don't offer lights nothing, rather than
        // lighting the closest one.
        assert!(!chip_is_active("85", "80"));
    }

    #[test]
    fn a_flag_chip_lights_only_for_a_flag_the_file_holds() {
        let file = vec!["liga".to_string(), "dlig".to_string()];
        assert!(flag_is_active(&file, "liga"));
        assert!(flag_is_active(&file, "dlig"));
        assert!(!flag_is_active(&file, "calt"));
        assert!(!flag_is_active(&[], "liga"));
    }

    /// A keybinding reads as trigger = action, and a line missing the `=`
    /// reads as a trigger with no action rather than panicking or dropping.
    #[test]
    fn a_binding_splits_on_its_first_equals() {
        assert_eq!(binding_parts("super+c=copy_to_clipboard"), ("super+c", "copy_to_clipboard"));
        // An empty action is `key =` — Ghostty reads it as the trigger with
        // nothing to run.
        assert_eq!(binding_parts("super+c="), ("super+c", ""));
        assert_eq!(binding_parts("super+c"), ("super+c", ""));
    }

    /// The summary shows the first two lines and stops; the "three more"
    /// count is the rest.
    #[test]
    fn a_summary_shows_the_first_two_lines() {
        let lines: Vec<String> = ["a=1", "b=2", "c=3", "d=4"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let (shown, more) = binding_summary(&lines);
        assert_eq!(shown, vec![("a", "1"), ("b", "2")]);
        assert_eq!(more, 2, "the rest are the 'more' count");
    }

    /// A list of two or fewer shows all of itself and counts nothing more.
    #[test]
    fn a_short_list_has_no_more_count() {
        let lines: Vec<String> = ["a=1"].iter().map(|s| s.to_string()).collect();
        let (shown, more) = binding_summary(&lines);
        assert_eq!(shown, vec![("a", "1")]);
        assert_eq!(more, 0);
    }
}