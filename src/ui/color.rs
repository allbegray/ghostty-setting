use egui::color_picker::Alpha;
use egui::{Color32, Ui};

/// Parse the hex forms Ghostty accepts: `#RRGGBB` or `RRGGBB`.
pub fn parse_hex(s: &str) -> Option<Color32> {
    let s = s.trim();
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(Color32::from_rgb(
        ((v >> 16) & 0xff) as u8,
        ((v >> 8) & 0xff) as u8,
        (v & 0xff) as u8,
    ))
}

pub fn to_hex(c: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

/// `true` when `s` looks like an attempt at hex that is not yet complete.
///
/// Used to block saving a half-typed color. X11 names (`red`) and special
/// values (`cell-foreground`) must not be caught by this, so it only fires
/// when the value is clearly heading toward six hex digits.
pub fn is_partial_hex(s: &str) -> bool {
    let t = s.trim();
    let body = t.strip_prefix('#').unwrap_or(t);
    !body.is_empty() && body.len() < 6 && body.chars().all(|c| c.is_ascii_hexdigit())
}

/// Inline color editor: a picker button plus a raw-value field.
///
/// The raw field is what keeps X11 color names and special values such as
/// `cell-foreground` editable — the picker can only speak hex, so it only
/// writes back once the user actually picks something.
pub fn color_field(
    ui: &mut Ui,
    key: &str,
    special: &'static [&'static str],
    value: &mut Option<String>,
) -> bool {
    let mut current = value.clone().unwrap_or_default();
    let mut changed = false;

    ui.horizontal(|ui| {
        let mut color = parse_hex(&current).unwrap_or(Color32::from_gray(120));
        if egui::color_picker::color_edit_button_srgba(ui, &mut color, Alpha::Opaque).changed() {
            current = to_hex(color);
            changed = true;
        }

        let mut text = current.clone();
        let resp = ui.add(
            egui::TextEdit::singleline(&mut text)
                .hint_text("#RRGGBB")
                .desired_width(104.0),
        );
        if resp.changed() {
            current = text;
            changed = true;
        }

        if !special.is_empty() {
            let label = if special.contains(&current.as_str()) {
                current.clone()
            } else {
                "특수값…".to_string()
            };
            egui::ComboBox::from_id_salt(("color-special", key))
                .width(110.0)
                .selected_text(label)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(false, "일반 색상").clicked() {
                        current.clear();
                        changed = true;
                    }
                    for s in special {
                        if ui.selectable_label(current == *s, *s).clicked() {
                            current = (*s).to_string();
                            changed = true;
                        }
                    }
                });
        }
    });

    if changed {
        *value = if current.trim().is_empty() {
            None
        } else {
            Some(current)
        };
    }
    changed
}
