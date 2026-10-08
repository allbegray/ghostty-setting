//! An option's stored value, read in the shapes the editors need.
//!
//! The editors ask the same few questions of a config string: is a boolean on,
//! where does this value sit in an enum's list, what colour does it name, which
//! flags are set. Those conversions used to sit inline in a 660-line render
//! function, where the only way to check one was to open the window.
//!
//! Everything here is a pure function of the stored string, so the rules are
//! testable without a window, a file, or a view.

use gpui_kit::Hsla;
use gpui_kit::SharedString;

/// An option's stored value.
#[derive(Clone, Copy, Debug)]
pub struct Stored<'a>(Option<&'a str>);

impl<'a> Stored<'a> {
    pub fn new(raw: Option<&'a str>) -> Self {
        Self(raw)
    }

    /// The stored text, or empty when the key is unset.
    pub fn text(&self) -> &'a str {
        self.0.unwrap_or_default()
    }

    /// Whether a boolean option is on.
    ///
    /// Only `true` is on: Ghostty treats a missing or malformed value as the
    /// option's default, which for every boolean is off.
    pub fn is_on(&self) -> bool {
        self.0 == Some("true")
    }

    /// Where this value sits in an enum's allowed values.
    pub fn index_in(&self, values: &[SharedString]) -> Option<usize> {
        let raw = self.0?;
        values.iter().position(|value| value.as_ref() == raw)
    }

    /// The flags this value sets, trimmed and without empties.
    ///
    /// Ghostty writes flags as a comma list, so `a, b` and `a,b` are the same
    /// selection and an unset key selects nothing.
    pub fn flags(&self) -> Vec<&'a str> {
        self.text()
            .split(',')
            .map(str::trim)
            .filter(|flag| !flag.is_empty())
            .collect()
    }

    /// The colour this value names, when it names one.
    pub fn color(&self) -> Option<Hsla> {
        let raw = self.0?;
        parse_hex_to_hsla(raw)
    }
}

/// The value a flag selection stores, or `None` when nothing is selected.
pub fn flags_value(flags: &[String]) -> Option<String> {
    if flags.is_empty() {
        None
    } else {
        Some(flags.join(","))
    }
}

/// `#rrggbb` for a colour, which is what the file stores.
pub fn hex(color: Hsla) -> String {
    let rgba = color.to_rgb();
    format!(
        "#{:02x}{:02x}{:02x}",
        (rgba.r * 255.0).round() as u8,
        (rgba.g * 255.0).round() as u8,
        (rgba.b * 255.0).round() as u8
    )
}

/// Parse `#rrggbb`, `rrggbb`, `#rgb` or `rgb` into a colour.
fn parse_hex_to_hsla(s: &str) -> Option<Hsla> {
    let hex = s.trim().trim_start_matches('#');
    let expand = |c: char| -> Option<u8> { c.to_digit(16).map(|d| (d * 17) as u8) };
    let (r, g, b) = match hex.len() {
        3 => {
            let mut chars = hex.chars();
            (
                expand(chars.next()?)?,
                expand(chars.next()?)?,
                expand(chars.next()?)?,
            )
        }
        6 => {
            let value = u32::from_str_radix(hex, 16).ok()?;
            (
                ((value >> 16) & 0xff) as u8,
                ((value >> 8) & 0xff) as u8,
                (value & 0xff) as u8,
            )
        }
        _ => return None,
    };
    Some(Hsla::from(gpui_kit::Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(items: &[&str]) -> Vec<SharedString> {
        items.iter().map(|s| SharedString::from(*s)).collect()
    }

    #[test]
    fn only_true_is_on() {
        assert!(Stored::new(Some("true")).is_on());
        assert!(!Stored::new(Some("false")).is_on());
        assert!(!Stored::new(Some("yes")).is_on());
        assert!(!Stored::new(None).is_on());
    }

    #[test]
    fn index_matches_the_stored_value_exactly() {
        let items = values(&["block", "bar", "underline"]);
        assert_eq!(Stored::new(Some("bar")).index_in(&items), Some(1));
        assert_eq!(Stored::new(Some("BAR")).index_in(&items), None);
        assert_eq!(Stored::new(None).index_in(&items), None);
    }

    #[test]
    fn flags_ignore_spacing_and_empties() {
        assert_eq!(Stored::new(Some("a, b ,c")).flags(), vec!["a", "b", "c"]);
        assert_eq!(Stored::new(Some("a,b")).flags(), vec!["a", "b"]);
        assert!(Stored::new(Some("")).flags().is_empty());
        assert!(Stored::new(None).flags().is_empty());
    }

    #[test]
    fn a_flag_selection_round_trips_through_the_file() {
        let stored = flags_value(&["a".to_string(), "b".to_string()]);
        assert_eq!(stored.as_deref(), Some("a,b"));
        assert_eq!(Stored::new(stored.as_deref()).flags(), vec!["a", "b"]);
        assert_eq!(flags_value(&[]), None);
    }

    #[test]
    fn colors_parse_in_every_accepted_form() {
        for raw in ["#1a1b26", "1a1b26", "#abc", "abc"] {
            assert!(Stored::new(Some(raw)).color().is_some(), "{raw} must parse");
        }
        assert!(Stored::new(Some("#12345")).color().is_none());
        assert!(Stored::new(Some("cell-foreground")).color().is_none());
        assert!(Stored::new(None).color().is_none());
    }

    #[test]
    fn a_color_survives_a_round_trip_through_hex() {
        let color = Stored::new(Some("#1a1b26")).color().unwrap();
        assert_eq!(hex(color), "#1a1b26");
    }
}
