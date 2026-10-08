//! Per-option control policy: what each option's editing control offers.
//!
//! Every other per-option fact lives in the schema (key, label, doc, kind,
//! valid values). Which *control* an option gets used to be spelled out in a
//! match arm per option across five tables, so a typo'd arm compiled and
//! silently never matched, the font-size default was written twice with two
//! values, and nothing checked that a key the arms named was a real option.
//! Policy lives here instead: one table keyed by option key, checked against
//! the schema by [`tests`].

use crate::config::schema::{Kind, Opt, lookup};

/// A set of quick values an option's input offers as chips.
///
/// The label is what the chip reads, the value is what it writes; they differ
/// when the chip shows something friendlier than the config value ("무제한 (0)"
/// writes `0`).
pub(crate) struct ChipSet {
    pub(crate) values: &'static [(&'static str, &'static str)],
}

/// What a slider control offers: where it rests, how far a step moves it, and
/// the unit its value carries.
pub(crate) struct Slider {
    /// The value the control shows when the option is unset.
    pub(crate) default: f64,
    /// One notch of the slider.
    pub(crate) step: f64,
    /// The unit suffix, when the value carries one.
    pub(crate) unit: Option<&'static str>,
}

/// What a number field offers: the seed value, the step, and the unit.
pub(crate) struct NumberField {
    pub(crate) default: &'static str,
    pub(crate) step: f64,
    pub(crate) unit: Option<&'static str>,
}

/// The range a numeric control offers, when the UI narrows what Ghostty
/// accepts. The schema owns what is valid; a control may offer less.
pub(crate) struct UiRange {
    pub(crate) key: &'static str,
    pub(crate) min: f64,
    pub(crate) max: f64,
}

/// The narrowed ranges. Every entry must stay inside `Kind::bounds()` —
/// `ui_ranges_stay_inside_schema_bounds` enforces that.
pub(crate) const UI_RANGES: &[UiRange] = &[
    UiRange { key: "font-size", min: 8.0, max: 72.0 },
    UiRange { key: "window-width", min: 20.0, max: 500.0 },
    UiRange { key: "window-height", min: 10.0, max: 200.0 },
];

/// The chip sets, keyed by option key.
pub(crate) const CHIP_SETS: &[(&str, ChipSet)] = &[
    ("command", ChipSet { values: &[
        ("/bin/zsh", "/bin/zsh"),
        ("/bin/bash", "/bin/bash"),
        ("fish", "/opt/homebrew/bin/fish"),
        ("tmux", "tmux"),
    ]}),
    ("background-blur", ChipSet { values: &[
        ("끔 (false)", "false"),
        ("은은하게 (10)", "10"),
        ("기본 (20)", "20"),
        ("강하게 (40)", "40"),
        ("Glass Regular", "macos-glass-regular"),
        ("Glass Clear", "macos-glass-clear"),
    ]}),
    ("scrollback-limit", ChipSet { values: &[
        ("10MB", "10000000"),
        ("50MB", "50000000"),
        ("100MB", "100000000"),
        ("500MB", "500000000"),
        ("1GB", "1000000000"),
        ("무제한 (0)", "0"),
    ]}),
    ("window-padding-x", ChipSet { values: &[
        ("0", "0"), ("4", "4"), ("8", "8"), ("12", "12"), ("16", "16"), ("24", "24"),
    ]}),
    ("window-padding-y", ChipSet { values: &[
        ("0", "0"), ("4", "4"), ("8", "8"), ("12", "12"), ("16", "16"), ("24", "24"),
    ]}),
    ("mouse-scroll-multiplier", ChipSet { values: &[
        ("1x (느림)", "1"),
        ("2x", "2"),
        ("3x (기본)", "3"),
        ("5x (빠름)", "5"),
    ]}),
    ("adjust-cell-width", ChipSet { values: &[
        ("-1", "-1"), ("0", "0"), ("+1", "1"), ("+2", "2"),
        ("-5%", "-5%"), ("+5%", "5%"), ("+10%", "10%"),
    ]}),
    ("adjust-cell-height", ChipSet { values: &[
        ("-1", "-1"), ("0", "0"), ("+1", "1"), ("+2", "2"),
        ("-5%", "-5%"), ("+5%", "5%"), ("+10%", "10%"),
    ]}),
    ("selection-word-chars", ChipSet { values: &[
        ("기본값 복원", "\\\\t'\\\"│`|:;,()[]{}<>$"),
    ]}),
];

/// The default font family, written once.
///
/// The preview's fallback and the editor's fallback both read this; they used
/// to each write the name, and a change to one left the other behind.
pub(crate) const DEFAULT_FONT: &str = "JetBrains Mono";

/// The popular coding fonts the font list editor offers as quick adds.
///
/// They are a content list, not control policy, but they live beside the
/// default they start from.
pub(crate) const POPULAR_FONTS: &[&str] = &[
    "JetBrains Mono",
    "SF Mono",
    "Menlo",
    "Monaco",
    "Fira Code",
    "Cascadia Code",
];

/// The window-size chip pairs, keyed by option key. They accompany a number
/// field rather than replacing one, so they are read beside it.
pub(crate) const SIZE_CHIPS: &[(&str, &[(&str, &str)])] = &[
    ("window-width", &[("80", "80"), ("100", "100"), ("120", "120"), ("140", "140")]),
    ("window-height", &[("24", "24"), ("30", "30"), ("40", "40"), ("50", "50")]),
];

/// The window-size number fields: seed, step and unit.
pub(crate) const SIZE_FIELDS: &[(&str, NumberField)] = &[
    ("window-width", NumberField { default: "80", step: 10.0, unit: Some("열") }),
    ("window-height", NumberField { default: "24", step: 5.0, unit: Some("행") }),
];

/// What the slider for `key` offers, when it has one.
pub(crate) fn slider(key: &str) -> Option<Slider> {
    Some(match key {
        "font-size" => Slider { default: 13.0, step: 1.0, unit: Some("pt") },
        "background-opacity" => Slider { default: 1.0, step: 0.05, unit: None },
        "cursor-opacity" => Slider { default: 1.0, step: 0.05, unit: None },
        "minimum-contrast" => Slider { default: 1.0, step: 0.5, unit: None },
        "font-thicken-strength" => Slider { default: 0.0, step: 16.0, unit: None },
        _ => return None,
    })
}

/// The quick values an option's input offers as chips, when it has any.
pub(crate) fn chips(key: &str) -> Option<&'static [(&'static str, &'static str)]> {
    CHIP_SETS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, set)| set.values)
}

/// The window-size chip pairs for `key`.
pub(crate) fn size_chips(key: &str) -> Option<&'static [(&'static str, &'static str)]> {
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
pub(crate) fn policy_keys() -> impl Iterator<Item = &'static str> {
    UI_RANGES
        .iter()
        .map(|range| range.key)
        .chain(CHIP_SETS.iter().map(|(key, _)| *key))
        .chain(SIZE_CHIPS.iter().map(|(key, _)| *key))
        .chain(SIZE_FIELDS.iter().map(|(key, _)| *key))
        .chain(["font-thicken-strength"])
}

/// Whether `value` is a value the option accepts.
///
/// A numeric kind accepts anything the parser reads; an enum accepts only its
/// listed values. The chip sets for numeric options are the interesting case:
/// a chip that writes a value Ghostty would reject must fail a test.
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
                    "'{key}' chip '{label}' writes '{value}', which the option refuses"
                );
            }
            for (_, value) in size_chips(key).into_iter().flatten() {
                assert!(
                    accepts(opt, value),
                    "'{key}' size chip '{value}' is not a value the option accepts"
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

    /// A slider rests at its default and steps through a value the option
    /// accepts: a default outside the valid range would show a value the file
    /// cannot hold.
    #[test]
    fn a_slider_default_rests_inside_the_valid_range() {
        for key in policy_keys() {
            let Some(opt) = lookup(key) else { continue };
            let Some(slider) = slider(key) else { continue };
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
}
