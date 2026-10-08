//! Per-option control policy: what each option's editing control offers.
//!
//! Every other per-option fact lives in the schema (key, label, doc, kind,
//! valid values). Which *control* an option gets used to be spelled out in a
//! match arm per option across five tables, so a typo'd arm compiled and
//! silently never matched, the font-size default was written twice with two
//! values, and nothing checked that a key the arms named was a real option.
//! Policy lives here instead: one table keyed by option key, and every key
//! the table names is one the schema knows — the check is what stops a typo
//! from rendering a control nothing reads.
//!
//! Bilingual copy follows the rule the rest of the app follows: anything a
//! user reads is a [`Text`], so a string cannot ship in one language only.

use crate::config::schema::Opt;
#[cfg(test)]
use crate::config::schema::{Kind, lookup};
use crate::i18n::Text;

/// A set of quick values an option's input offers as chips.
///
/// The label is what the chip reads, the value is what it writes; they differ
/// when the chip shows something friendlier than the config value.
pub(crate) struct ChipSet {
    pub(crate) values: &'static [(Text, &'static str)],
}

/// What a slider control offers: where it rests, how far a step moves it, and
/// the unit its value carries.
pub(crate) struct Slider {
    /// The value the control shows when the option is unset, and the default
    /// the preview rests at.
    pub(crate) default: f64,
    /// One notch of the slider.
    pub(crate) step: f64,
    /// The unit suffix, when the value carries one.
    pub(crate) unit: Option<Text>,
}

/// What a number field offers: the seed value, the step, and the unit.
pub(crate) struct NumberField {
    pub(crate) default: &'static str,
    pub(crate) step: f64,
    pub(crate) unit: Option<Text>,
}

/// The range a numeric control offers, when the UI narrows what Ghostty
/// accepts. The schema owns what is valid; a control may offer less.
pub(crate) struct UiRange {
    pub(crate) key: &'static str,
    pub(crate) min: f64,
    pub(crate) max: f64,
}

/// The default font family, written once.
///
/// The preview's fallback and the editor's fallback both read this; they used
/// to each write the name, and a change to one left the other behind.
pub(crate) const DEFAULT_FONT: &str = "JetBrains Mono";

/// The popular coding fonts the font list editor offers as quick adds.
pub(crate) const POPULAR_FONTS: &[&str] = &[
    "JetBrains Mono",
    "SF Mono",
    "Menlo",
    "Monaco",
    "Fira Code",
    "Cascadia Code",
];

/// Every key with a slider. The tests walk it, and [`slider`] asserts it
/// stays complete — a key added here without a slider, or a slider key not
/// listed, would be the silent-miss failure this module replaced.
const SLIDER_KEYS: &[&str] = &[
    "font-size",
    "background-opacity",
    "cursor-opacity",
    "minimum-contrast",
    "font-thicken-strength",
];

/// The narrowed ranges. Every entry must stay inside `Kind::bounds()` —
/// `ui_ranges_stay_inside_schema_bounds` enforces that.
pub(crate) const UI_RANGES: &[UiRange] = &[
    UiRange { key: "font-size", min: 8.0, max: 72.0 },
    UiRange { key: "window-width", min: 20.0, max: 500.0 },
    UiRange { key: "window-height", min: 10.0, max: 200.0 },
];

/// Padding steps, shared by the two window-padding options.
const PADDING_CHIPS: &[(Text, &str)] = &[
    (Text::new("0", "0"), "0"),
    (Text::new("4", "4"), "4"),
    (Text::new("8", "8"), "8"),
    (Text::new("12", "12"), "12"),
    (Text::new("16", "16"), "16"),
    (Text::new("24", "24"), "24"),
];

/// Cell adjustment steps, shared by the two adjust-cell options.
const CELL_CHIPS: &[(Text, &str)] = &[
    (Text::new("-1", "-1"), "-1"),
    (Text::new("0", "0"), "0"),
    (Text::new("+1", "+1"), "1"),
    (Text::new("+2", "+2"), "2"),
    (Text::new("-5%", "-5%"), "-5%"),
    (Text::new("+5%", "+5%"), "5%"),
    (Text::new("+10%", "+10%"), "10%"),
];

/// The chip sets, keyed by option key.
pub(crate) const CHIP_SETS: &[(&str, ChipSet)] = &[
    ("command", ChipSet { values: &[
        (Text::new("/bin/zsh", "/bin/zsh"), "/bin/zsh"),
        (Text::new("/bin/bash", "/bin/bash"), "/bin/bash"),
        (Text::new("fish", "fish"), "/opt/homebrew/bin/fish"),
        (Text::new("tmux", "tmux"), "tmux"),
    ]}),
    ("background-blur", ChipSet { values: &[
        (Text::new("끔 (false)", "Off (false)"), "false"),
        (Text::new("은은하게 (10)", "Subtle (10)"), "10"),
        (Text::new("기본 (20)", "Default (20)"), "20"),
        (Text::new("강하게 (40)", "Strong (40)"), "40"),
        (Text::new("Glass Regular", "Glass Regular"), "macos-glass-regular"),
        (Text::new("Glass Clear", "Glass Clear"), "macos-glass-clear"),
    ]}),
    ("scrollback-limit", ChipSet { values: &[
        (Text::new("10MB", "10MB"), "10000000"),
        (Text::new("50MB", "50MB"), "50000000"),
        (Text::new("100MB", "100MB"), "100000000"),
        (Text::new("500MB", "500MB"), "500000000"),
        (Text::new("1GB", "1GB"), "1000000000"),
        (Text::new("무제한 (0)", "Unlimited (0)"), "0"),
    ]}),
    ("window-padding-x", ChipSet { values: PADDING_CHIPS }),
    ("window-padding-y", ChipSet { values: PADDING_CHIPS }),
    ("mouse-scroll-multiplier", ChipSet { values: &[
        (Text::new("1x (느림)", "1x (slow)"), "1"),
        (Text::new("2x", "2x"), "2"),
        (Text::new("3x (기본)", "3x (default)"), "3"),
        (Text::new("5x (빠름)", "5x (fast)"), "5"),
    ]}),
    ("adjust-cell-width", ChipSet { values: CELL_CHIPS }),
    ("adjust-cell-height", ChipSet { values: CELL_CHIPS }),
    ("selection-word-chars", ChipSet { values: &[
        (Text::new("기본값 복원", "Restore default"), "\\\\t'\\\"│`|:;,()[]{}<>$"),
    ]}),
];

/// The window-size chip pairs, keyed by option key. They accompany a number
/// field rather than replacing one, so they are read beside it.
pub(crate) const SIZE_CHIPS: &[(&str, &[(Text, &str)])] = &[
    ("window-width", &[
        (Text::new("80", "80"), "80"),
        (Text::new("100", "100"), "100"),
        (Text::new("120", "120"), "120"),
        (Text::new("140", "140"), "140"),
    ]),
    ("window-height", &[
        (Text::new("24", "24"), "24"),
        (Text::new("30", "30"), "30"),
        (Text::new("40", "40"), "40"),
        (Text::new("50", "50"), "50"),
    ]),
];

/// The window-size number fields: seed, step and unit.
pub(crate) const SIZE_FIELDS: &[(&str, NumberField)] = &[
    ("window-width", NumberField { default: "80", step: 10.0, unit: Some(Text::new("열", "cols")) }),
    ("window-height", NumberField { default: "24", step: 5.0, unit: Some(Text::new("행", "rows")) }),
];

/// What the slider for `key` offers, when it has one.
///
/// The key list is [`SLIDER_KEYS`], so a key with no entry falls through and a
/// test walks every key on it.
pub(crate) fn slider(key: &str) -> Option<Slider> {
    if !SLIDER_KEYS.contains(&key) {
        return None;
    }
    Some(match key {
        "font-size" => Slider { default: 13.0, step: 1.0, unit: Some(Text::new("pt", "pt")) },
        "background-opacity" => Slider { default: 1.0, step: 0.05, unit: None },
        "cursor-opacity" => Slider { default: 1.0, step: 0.05, unit: None },
        "minimum-contrast" => Slider { default: 1.0, step: 0.5, unit: None },
        "font-thicken-strength" => Slider { default: 0.0, step: 16.0, unit: None },
        _ => return None,
    })
}

/// The quick values an option's input offers as chips, when it has any.
pub(crate) fn chips(key: &str) -> Option<&'static [(Text, &'static str)]> {
    CHIP_SETS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, set)| set.values)
}

/// The window-size chip pairs for `key`.
pub(crate) fn size_chips(key: &str) -> Option<&'static [(Text, &str)]> {
    SIZE_CHIPS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, pairs)| *pairs)
}

/// The window-size number field for `key`.
pub(crate) fn size_field(key: &str) -> Option<&'static NumberField> {
    SIZE_FIELDS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, field)| field)
}

/// The range a numeric control offers: the narrowed UI range when one is
/// declared, otherwise the schema's valid range.
pub(crate) fn edit_bounds(opt: &Opt) -> (f64, f64) {
    if let Some(range) = UI_RANGES.iter().find(|range| range.key == opt.key) {
        return (range.min, range.max);
    }
    opt.kind.bounds().unwrap_or((0.0, 100.0))
}

/// Every key the policy names, across all its tables.
///
/// Derived from the tables themselves, never hand-written: a key added to one
/// table is visited by the schema checks automatically, which is the whole
/// point — the failure this module replaced was a key nobody checked.
#[cfg(test)]
pub(crate) fn policy_keys() -> impl Iterator<Item = &'static str> {
    UI_RANGES
        .iter()
        .map(|range| range.key)
        .chain(CHIP_SETS.iter().map(|(key, _)| *key))
        .chain(SIZE_CHIPS.iter().map(|(key, _)| *key))
        .chain(SIZE_FIELDS.iter().map(|(key, _)| *key))
        .chain(SLIDER_KEYS.iter().copied())
}

/// The keys the option editor special-cases while opening a list modal, and
/// the content lists they depend on. `keybind` and `font-feature` are list
/// keys the modal shapes on; `font-family` seeds its font dropdown.
#[cfg(test)]
pub(crate) const LIST_MODAL_KEYS: &[&str] = &["keybind", "font-family", "font-feature", "config-file"];

/// Whether `value` is a value the option accepts.
///
/// A numeric kind accepts anything the parser reads inside its range, an enum
/// accepts only its listed values; every other kind accepts anything, because
/// its values are free text or a repeat it cannot pre-judge.
#[cfg(test)]
pub(crate) fn accepts(opt: &Opt, value: &str) -> bool {
    match opt.kind {
        Kind::Int { min, max } => match value.parse::<i64>() {
            Ok(number) => (min..=max).contains(&number),
            Err(_) => false,
        },
        Kind::Float { min, max } => match value.parse::<f64>() {
            Ok(number) => (min..=max).contains(&number),
            Err(_) => false,
        },
        Kind::Enum(values) => values.contains(&value),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The failure mode this module exists for: a policy entry naming a key
    /// that isn't an option would render a control nothing reads.
    #[test]
    fn every_policy_key_is_an_option() {
        for key in policy_keys() {
            assert!(
                lookup(key).is_some(),
                "'{key}' is named by the control policy but is not an option"
            );
        }
    }

    /// A chip writes its value straight into the file, so a chip the option
    /// would reject is a chip that writes a config Ghostty refuses.
    #[test]
    fn every_chip_value_is_one_the_option_accepts() {
        for key in policy_keys() {
            let Some(opt) = lookup(key) else { continue };
            for (label, value) in chips(key).into_iter().flatten() {
                assert!(
                    accepts(opt, value),
                    "'{key}' chip '{}' writes '{value}', which the option refuses",
                    label.s()
                );
            }
            for (label, value) in size_chips(key).into_iter().flatten() {
                assert!(
                    accepts(opt, value),
                    "'{key}' size chip '{}' is not a value the option accepts",
                    label.s()
                );
            }
        }
    }

    /// The UI may offer less than Ghostty accepts, but never more: a narrowed
    /// range that escapes the valid range would let a control write a value
    /// the config rejects.
    #[test]
    fn ui_ranges_stay_inside_schema_bounds() {
        for range in UI_RANGES {
            let opt = lookup(range.key).unwrap_or_else(|| panic!("{} is not an option", range.key));
            let (valid_min, valid_max) = opt
                .kind
                .bounds()
                .unwrap_or_else(|| panic!("{} is not numeric", range.key));
            assert!(
                valid_min <= range.min && range.max <= valid_max,
                "{}: UI range {}..{} escapes the valid range {}..{}",
                range.key,
                range.min,
                range.max,
                valid_min,
                valid_max
            );
        }
    }

    /// The key list and the lookup are one fact in two places: a key listed
    /// without a slider, or a slider key missing from the list, is the
    /// silent-miss failure this module replaced.
    #[test]
    fn every_slider_key_has_a_slider() {
        for key in SLIDER_KEYS {
            assert!(slider(key).is_some(), "'{key}' is listed but has no slider");
        }
    }

    /// A slider rests at its default and steps through a value the option
    /// accepts: a default outside the valid range would show a value the file
    /// cannot hold. Walked over every slider key, not a hand-written subset.
    #[test]
    fn a_slider_default_rests_inside_the_valid_range() {
        for key in SLIDER_KEYS {
            let opt = lookup(key).unwrap_or_else(|| panic!("{key} is not an option"));
            let slider = slider(key).unwrap_or_else(|| panic!("{key} has no slider"));
            if let Some((min, max)) = opt.kind.bounds() {
                assert!(
                    (min..=max).contains(&slider.default),
                    "'{key}' slider rests at {}, outside {}..{}",
                    slider.default,
                    min,
                    max
                );
            }
        }
    }

    /// A chip set only makes sense on an option whose control is an input:
    /// a chip writing to a boolean or an enum would offer values the control
    /// cannot read back.
    #[test]
    fn a_chip_set_sits_on_an_option_a_text_control_edits() {
        for key in policy_keys() {
            let Some(opt) = lookup(key) else { continue };
            if chips(key).is_none() {
                continue;
            }
            assert!(
                matches!(opt.kind, Kind::Text | Kind::Int { .. } | Kind::Float { .. }),
                "'{key}' carries chips but its kind {:?} has no text control",
                opt.kind
            );
        }
    }

    /// The bilingual rule: a label a user reads must answer in the language
    /// they chose. A label that stops answering in one language is the bug
    /// the move almost shipped, so every policy label is checked in both.
    #[test]
    fn every_policy_label_answers_in_both_languages() {
        let restore = crate::i18n::current();
        for &lang in crate::i18n::Lang::ALL {
            crate::i18n::set(lang);
            for key in policy_keys() {
                for (label, _) in chips(key).into_iter().flatten() {
                    assert!(!label.get(lang).is_empty(), "'{key}' has no {lang:?} label");
                }
            }
            for (key, field) in SIZE_FIELDS {
                if let Some(unit) = field.unit {
                    assert!(
                        !unit.get(lang).is_empty(),
                        "'{key}' has no {lang:?} unit"
                    );
                }
            }
        }
        crate::i18n::set(restore);
    }

    /// The modal's list keys are the keys its shapes name: a shape without an
    /// option would be dead code, and an option without its shape would fall
    /// to the generic editor.
    #[test]
    fn every_list_modal_key_is_an_option() {
        for key in LIST_MODAL_KEYS {
            let opt = lookup(key).unwrap_or_else(|| panic!("{key} is not an option"));
            assert!(
                matches!(opt.kind, Kind::List),
                "'{key}' opens the list modal but is not a repeatable key"
            );
        }
    }
}
