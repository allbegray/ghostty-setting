//! Per-`Kind` value editors.
//!
//! Every editor takes the current presence state (`None` = the key is absent
//! from the file) and reports whether it changed, so the caller decides when
//! to write back. Nothing here knows Ghostty's defaults — "unset" is always
//! represented by absence, never by a guessed default value.

use crate::config::{Kind, Opt};
use crate::ui::color;
use egui::Ui;

/// Scalar editor for everything except `List` and `Flags`, which have their
/// own multi-row editors.
pub fn edit(ui: &mut Ui, opt: &Opt, value: &mut Option<String>) -> bool {
    match opt.kind {
        Kind::Bool => edit_bool(ui, opt, value),
        Kind::Enum(items) => edit_enum(ui, opt, items, value),
        Kind::Int { min, max } => edit_int(ui, opt, value, min, max),
        Kind::Float { min, max } => edit_float(ui, opt, value, min, max),
        Kind::Color { special } => color::color_field(ui, opt.key, special, value),
        Kind::Text => edit_text(ui, opt, value),
        Kind::List | Kind::Flags(_) => false,
    }
}

fn edit_bool(ui: &mut Ui, opt: &Opt, value: &mut Option<String>) -> bool {
    let choices = ["true", "false"];
    let mut sel = match value.as_deref() {
        Some("true") => 1,
        Some("false") => 2,
        _ => 0,
    };
    let before = sel;

    egui::ComboBox::from_id_salt(("bool", opt.key))
        .width(110.0)
        .selected_text(if sel == 0 { "기본값" } else { choices[sel - 1] })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut sel, 0, "기본값");
            ui.selectable_value(&mut sel, 1, "true");
            ui.selectable_value(&mut sel, 2, "false");
        });

    commit_choice(value, sel, before, &choices)
}

fn edit_enum(
    ui: &mut Ui,
    opt: &Opt,
    items: &'static [&'static str],
    value: &mut Option<String>,
) -> bool {
    let current = value.as_deref().unwrap_or("");
    let mut sel = items
        .iter()
        .position(|i| *i == current)
        .map(|i| i + 1)
        .unwrap_or(0);
    let before = sel;

    egui::ComboBox::from_id_salt(("enum", opt.key))
        .width(170.0)
        .selected_text(if sel == 0 { "기본값" } else { items[sel - 1] })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut sel, 0, "기본값");
            for (i, label) in items.iter().enumerate() {
                ui.selectable_value(&mut sel, i + 1, *label);
            }
        });

    commit_choice(value, sel, before, items)
}

/// Map a changed combo selection back to presence: index 0 always means
/// "remove the key so Ghostty's default applies".
fn commit_choice(value: &mut Option<String>, sel: usize, before: usize, choices: &[&str]) -> bool {
    if sel == before {
        return false;
    }
    if sel == 0 {
        *value = None;
    } else {
        *value = Some(choices[sel - 1].to_string());
    }
    true
}

/// Numeric editors show a "configure…" affordance instead of a number while
/// unset, so the digits on screen are always a value actually in the file.
fn edit_int(ui: &mut Ui, opt: &Opt, value: &mut Option<String>, min: i64, max: i64) -> bool {
    let Some(mut n) = value.as_deref().and_then(|s| s.parse::<i64>().ok()) else {
        return ghost_enable(ui, opt, value, min.clamp(0, max).to_string());
    };
    if ui
        .add(egui::DragValue::new(&mut n).range(min..=max).speed(1.0))
        .changed()
    {
        *value = Some(n.to_string());
        return true;
    }
    false
}

fn edit_float(ui: &mut Ui, opt: &Opt, value: &mut Option<String>, min: f64, max: f64) -> bool {
    let Some(mut n) = value.as_deref().and_then(|s| s.parse::<f64>().ok()) else {
        let seed = if min > 0.0 { min } else { 0.0 };
        return ghost_enable(ui, opt, value, format_number(seed));
    };
    let resp = ui.add(
        egui::DragValue::new(&mut n)
            .range(min..=max)
            .speed((max - min) / 200.0)
            .max_decimals(3),
    );
    if resp.changed() {
        *value = Some(format_number(n));
        return true;
    }
    false
}

/// Format without trailing-zero noise so `background-opacity = 1` stays `1`.
fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        let s = format!("{n:.4}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn ghost_enable(ui: &mut Ui, opt: &Opt, value: &mut Option<String>, seed: String) -> bool {
    ui.label(egui::RichText::new("설정되지 않음").weak().italics());
    if ui
        .small_button("설정…")
        .on_hover_text(format!("`{}` 키를 설정 파일에 추가합니다", opt.key))
        .clicked()
    {
        *value = Some(seed);
        return true;
    }
    false
}

fn edit_text(ui: &mut Ui, opt: &Opt, value: &mut Option<String>) -> bool {
    let mut text = value.clone().unwrap_or_default();
    let hint = if opt.hint.is_empty() {
        "값 입력"
    } else {
        opt.hint
    };
    let resp = ui.add(egui::TextEdit::singleline(&mut text).hint_text(hint).desired_width(250.0));
    if resp.changed() {
        *value = if text.is_empty() { None } else { Some(text) };
        return true;
    }
    false
}

/// Repeatable keys: an ordered list of values, one per config line.
///
/// Rows may be left empty while the user is still typing; the caller filters
/// them out when writing so a blank row never becomes a real `key =` line.
pub fn edit_list(ui: &mut Ui, opt: &Opt, values: &mut Vec<String>) -> bool {
    let mut changed = false;
    let mut remove: Option<usize> = None;
    let mut swap: Option<(usize, usize)> = None;

    for i in 0..values.len() {
        ui.horizontal(|ui| {
            if i > 0 && ui.small_button("↑").on_hover_text("위로").clicked() {
                swap = Some((i, i - 1));
            }
            if i + 1 < values.len() && ui.small_button("↓").on_hover_text("아래로").clicked() {
                swap = Some((i, i + 1));
            }

            let item = &mut values[i];
            let resp = ui
                .add(egui::TextEdit::singleline(item).hint_text(opt.hint).desired_width(310.0));
            if resp.changed() {
                changed = true;
            }
            if ui.small_button("✕").on_hover_text("삭제").clicked() {
                remove = Some(i);
            }
        });
    }

    if let Some(i) = remove {
        values.remove(i);
        changed = true;
    }
    if let Some((a, b)) = swap {
        values.swap(a, b);
        changed = true;
    }
    if ui.small_button("+ 추가").clicked() {
        values.push(String::new());
        changed = true;
    }

    changed
}

/// Comma-separated flags where an item may carry a `no-` prefix.
///
/// Tokens we do not model are carried through verbatim, so rewriting the
/// field can never silently drop a flag.
pub fn edit_flags(ui: &mut Ui, opt: &Opt, value: &mut Option<String>) -> bool {
    let Kind::Flags(known) = opt.kind else {
        return false;
    };

    let raw = value.clone().unwrap_or_default();
    let tokens: Vec<&str> = raw
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();

    let mut state: Vec<Option<bool>> = vec![None; known.len()];
    let mut extras: Vec<String> = Vec::new();

    for tok in &tokens {
        match *tok {
            "true" => state = vec![Some(true); known.len()],
            "false" => state = vec![Some(false); known.len()],
            _ => {
                let (neg, name) = match tok.strip_prefix("no-") {
                    Some(rest) => (true, rest),
                    None => (false, *tok),
                };
                match known.iter().position(|k| *k == name) {
                    Some(idx) => state[idx] = Some(!neg),
                    None => extras.push((*tok).to_string()),
                }
            }
        }
    }

    let mut changed = false;
    for (i, flag) in known.iter().enumerate() {
        let mut sel = match state[i] {
            Some(true) => 2,
            Some(false) => 3,
            None => 1,
        };
        let before = sel;
        ui.horizontal(|ui| {
            ui.label(*flag);
            egui::ComboBox::from_id_salt(("flag", opt.key, *flag))
                .width(90.0)
                .selected_text(["", "기본값", "켜기", "끄기"][sel])
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut sel, 1, "기본값");
                    ui.selectable_value(&mut sel, 2, "켜기");
                    ui.selectable_value(&mut sel, 3, "끄기");
                });
        });
        if sel != before {
            state[i] = match sel {
                2 => Some(true),
                3 => Some(false),
                _ => None,
            };
            changed = true;
        }
    }

    if changed {
        let mut out = extras;
        for (i, flag) in known.iter().enumerate() {
            match state[i] {
                Some(true) => out.push((*flag).to_string()),
                Some(false) => out.push(format!("no-{flag}")),
                None => {}
            }
        }
        *value = if out.is_empty() {
            None
        } else {
            Some(out.join(", "))
        };
    }

    changed
}
