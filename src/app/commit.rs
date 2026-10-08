//! What a control's change means for the file.
//!
//! The one path that changes an option's value ends in a decision with two
//! different outcomes that look alike: `set(key, "")` writes
//! `font-family = ` (an explicit reset, in Ghostty's reading), while removing
//! the key leaves no line at all. Which one a widget meant by "empty" used to
//! be a ternary spelled at each call site — a widget that said blank because
//! the user cleared it means removal; a widget that said blank because the
//! option's value *is* blank means the reset. Getting that wrong silently
//! changes the user's config, and nothing tested the choice.
//!
//! The decision lives here, pure and testable: a control reports what happened
//! in its own vocabulary, and this says which file edit follows and which slot
//! drew it. The view applies the result and settles; the controls stop
//! spelling file semantics inside their closures.

use crate::app::editors::Kept;
use crate::config::linefile::LineFile;

/// What a control reported.
#[derive(Clone, Debug, PartialEq)]
pub enum Reported {
    /// A text field's content, already read out.
    Typed(String),
    /// A control handed over a ready value.
    Chosen {
        /// The editor the value came from, which survives the change.
        slot: Kept,
        value: String,
    },
    /// The full set of a repeatable key's items.
    List(Vec<String>),
    /// The option is being cleared back to its default.
    Cleared,
}

/// The file edit a report calls for.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    /// Write this value.
    Set(String),
    /// Remove the key's lines.
    Clear,
    /// Replace a repeatable key's lines with these items.
    ReplaceAll(Vec<String>),
}

impl Edit {
    /// Apply the edit to the file.
    pub fn apply(self, key: &str, file: &mut LineFile) {
        match self {
            Edit::Set(value) => file.set(key, &value),
            Edit::Clear => file.remove(key),
            Edit::ReplaceAll(values) => file.set_all(key, &values),
        }
    }
}

/// The decision: what edit the report calls for, and which slot drew it.
#[derive(Clone, Debug, PartialEq)]
pub struct Decision {
    pub edit: Edit,
    pub kept: Kept,
}

/// What a control reports when it hands over a ready value: a confirmed
/// dropdown, a settled colour, a moved slider, a clicked chip.
///
/// The value is written as it stands, and the slot that survives is the one
/// the report came from.
pub fn chosen(slot: Kept, value: &str) -> Decision {
    Decision {
        edit: Edit::Set(value.to_string()),
        kept: slot,
    }
}

/// Turn a control's report into the decision, without touching the file.
///
/// The view owns applying it; this is the pure part, so the rules are
/// assertable without a window.
pub fn decide(report: Reported) -> Decision {
    match report {
        Reported::Typed(value) => typed(&value),
        Reported::Chosen { slot, value } => chosen(slot, &value),
        Reported::List(items) => list(&items),
        Reported::Cleared => cleared(),
    }
}

/// What a report from a text field means.
///
/// A field's blank means removal — the user cleared the key — and anything
/// else is written trimmed. That is the whole distinction, and it is made once,
/// here, rather than per call site.
pub fn typed(value: &str) -> Decision {
    let value = value.trim();
    Decision {
        edit: if value.is_empty() {
            Edit::Clear
        } else {
            Edit::Set(value.to_string())
        },
        kept: Kept::Input,
    }
}

/// What a repeatable key's full list means.
///
/// The list replaces the key's lines wholesale, keeping the user's grouping
/// elsewhere in the file.
pub fn list(values: &[String]) -> Decision {
    Decision {
        edit: Edit::ReplaceAll(values.to_vec()),
        kept: Kept::Nothing,
    }
}

/// What a reset or a stateless control's pick means: the key goes back to its
/// default, and every editor for it is stale.
pub fn cleared() -> Decision {
    Decision {
        edit: Edit::Clear,
        kept: Kept::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> LineFile {
        LineFile::parse("font-family = Menlo\nfont-size = 14\n")
    }

    /// The distinction the module exists for: a cleared field removes the
    /// line, where setting an empty value would write `font-family = `.
    #[test]
    fn an_emptied_field_removes_the_line() {
        let mut file = file();
        typed("").edit.clone().apply("font-family", &mut file);
        assert_eq!(file.render(), "font-size = 14\n");
        assert!(file.get("font-family").is_none());
    }

    /// Whitespace is not a value: a field holding spaces is a cleared field.
    #[test]
    fn a_blank_field_removes_the_line() {
        let mut file = file();
        typed("   ").edit.clone().apply("font-family", &mut file);
        assert!(file.get("font-family").is_none());
    }

    #[test]
    fn a_typed_value_is_written_trimmed() {
        let mut file = file();
        typed("  Menlo  ").edit.clone().apply("font-family", &mut file);
        assert_eq!(file.render(), "font-family = Menlo\nfont-size = 14\n");
    }

    /// A chosen value is written as it stands — no trimming, no blank rule:
    /// a dropdown reports a value or nothing.
    #[test]
    fn a_chosen_value_is_written_verbatim() {
        let mut file = file();
        chosen(Kept::Select, "Nord").edit.clone().apply("theme", &mut file);
        assert_eq!(file.render(), "font-family = Menlo\nfont-size = 14\ntheme = Nord\n");
    }

    /// A round trip that leaves the file as it found it: on, then off.
    #[test]
    fn a_toggle_on_then_off_restores_the_file() {
        let original = file();
        let mut file = file();

        // on: the flag's item is written
        let mut items: Vec<String> = original
            .get_all("font-feature")
            .iter()
            .map(|s| s.to_string())
            .collect();
        items.push("liga".into());
        list(&items).edit.clone().apply("font-feature", &mut file);
        assert!(file.render().contains("liga"));

        // off: the item goes, and the rest of the file is untouched
        items.retain(|f| f != "liga");
        list(&items).edit.clone().apply("font-feature", &mut file);
        assert_eq!(file.render(), original.render());
    }

    /// A replace of an empty list removes the key's lines rather than writing
    /// a placeholder.
    #[test]
    fn an_empty_list_removes_the_key() {
        let mut file = file();
        list(&[]).edit.clone().apply("font-family", &mut file);
        assert!(file.get("font-family").is_none());
    }

    /// The slot naming, asserted beside the edit it belongs to: a widget's
    /// report says which editor drew it, so no call site guesses.
    #[test]
    fn each_report_names_the_slot_it_drew() {
        assert_eq!(typed("14").kept, Kept::Input);
        assert_eq!(chosen(Kept::Select, "nord").kept, Kept::Select);
        assert_eq!(chosen(Kept::Color, "#112233").kept, Kept::Color);
        assert_eq!(chosen(Kept::Slider, "13").kept, Kept::Slider);
        assert_eq!(cleared().kept, Kept::Nothing);
    }

    /// A stateless control leaves every editor stale, and its edit removes.
    #[test]
    fn a_clearing_control_keeps_nothing() {
        let decision = cleared();
        assert_eq!(decision.edit, Edit::Clear);
        assert_eq!(decision.kept, Kept::Nothing);
    }
}
