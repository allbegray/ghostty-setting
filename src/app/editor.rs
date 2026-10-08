//! The value editor for one option row.
//!
//! One entry point, [`value_editor`], picks the control a `Kind` needs. The
//! six arms and their helpers are implementation: the row that calls this
//! knows only the option, the view and the window.

use super::*;
use super::list_editor::ListEditorModal;
use super::list_items::ListItems;
use crate::app::value::{self, Stored};

fn render_bounded_slider_number(
    this: &mut SettingsView,
    key: &'static str,
    val_str: &str,
    default_val: f64,
    min: f64,
    max: f64,
    step: f64,
    is_float: bool,
    unit: Option<&'static str>,
    window: &mut Window,
    cx: &mut Context<SettingsView>,
) -> gpui_kit::AnyElement {
    let num_state = this.get_or_create_number_input(
        key,
        &format!("{default_val}"),
        min,
        max,
        step,
        window,
        cx,
    );

    let mut num_input = NumberInput::new(&num_state).small();
    if let Some(suf) = unit {
        num_input = num_input.suffix(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(suf),
        );
    }

    let cur: f32 = val_str.parse().unwrap_or(default_val as f32);
    let (slider_state, created) =
        this.editors
            .slider(key, cur, min as f32, max as f32, step as f32, cx);
    if created {
        cx.subscribe(&slider_state, move |this, _, event, cx| {
            let (SliderEvent::Change(val) | SliderEvent::Release(val)) = event;
            let new_val = if is_float {
                format!("{:.2}", val.start())
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            } else {
                format!("{}", val.start().round() as i64)
            };
            this.commit(key, Some(&new_val), Kept::Slider, cx);
        })
        .detach();
    }

    h_flex()
        .gap_3()
        .items_center()
        .child(
            div()
                .w(px(160.))
                .child(Slider::new(&slider_state))
        )
        .child(
            div()
                .w(px(110.))
                .child(num_input)
        )
        .into_any_element()
}

fn render_chips_only(
    key: &'static str,
    current_val: &str,
    chips: &[(&'static str, &'static str)],
    view: Entity<SettingsView>,
    cx: &mut Context<SettingsView>,
) -> gpui_kit::AnyElement {
    h_flex()
        .gap_1()
        .flex_wrap()
        .children(chips.iter().map(|(label, target)| {
            let is_active = current_val == *target;
            let view = view.clone();
            let target_str = target.to_string();
            div()
                .id(format!("{key}-chip-{label}"))
                .cursor_pointer()
                .px_1p5()
                .py(px(1.))
                .rounded_sm()
                .border_1()
                .text_xs()
                .when(is_active, |s| {
                    s.bg(cx.theme().primary.opacity(0.12))
                        .border_color(cx.theme().primary)
                        .text_color(cx.theme().primary)
                })
                .when(!is_active, |s| {
                    s.bg(cx.theme().background)
                        .border_color(cx.theme().border)
                        .text_color(cx.theme().muted_foreground)
                        .hover(|s| s.text_color(cx.theme().foreground))
                })
                .child(*label)
                .on_click(move |_, _, cx| {
                    view.update(cx, |this, cx| {
                        this.commit(key, Some(&target_str), Kept::Nothing, cx);
                    });
                })
        }))
        .into_any_element()
}
fn render_input_with_chips(
    state: &Entity<InputState>,
    key: &'static str,
    current_val: &str,
    chips: &[(&'static str, &'static str)],
    view: Entity<SettingsView>,
    cx: &mut Context<SettingsView>,
) -> gpui_kit::AnyElement {
    h_flex()
        .gap_2()
        .items_center()
        .child(
            div()
                .max_w(px(180.))
                .child(Input::new(state).small())
        )
        .child(
            h_flex()
                .gap_1()
                .flex_wrap()
                .children(chips.iter().map(|(label, target)| {
                    let is_active = current_val == *target;
                    let view = view.clone();
                    let target_str = target.to_string();
                    div()
                        .id(format!("{key}-chip-{label}"))
                        .cursor_pointer()
                        .px_1p5()
                        .py(px(1.))
                        .rounded_sm()
                        .border_1()
                        .text_xs()
                        .when(is_active, |s| {
                            s.bg(cx.theme().primary.opacity(0.12))
                                .border_color(cx.theme().primary)
                                .text_color(cx.theme().primary)
                        })
                        .when(!is_active, |s| {
                            s.bg(cx.theme().background)
                                .border_color(cx.theme().border)
                                .text_color(cx.theme().muted_foreground)
                                .hover(|s| s.text_color(cx.theme().foreground))
                        })
                        .child(*label)
                        .on_click(move |_, _, cx| {
                            view.update(cx, |this, cx| {
                                this.commit(key, Some(&target_str), Kept::Nothing, cx);
                            });
                        })
                }))
        )
        .into_any_element()
}

pub(super) fn value_editor(
    this: &mut SettingsView,
    opt: &'static Opt,
    window: &mut Window,
    cx: &mut Context<SettingsView>,
) -> gpui_kit::AnyElement {
    match opt.kind {
        Kind::Bool => {
            let on = Stored::new(this.file.get(opt.key).as_deref()).is_on();
            let view = cx.entity();
            let key = opt.key;
            Switch::new(opt.key)
                .checked(on)
                .on_change(move |&value, _, cx| {
                    view.update(cx, |this, cx| {
                        let value = if value { "true" } else { "false" };
                        this.commit(key, Some(value), Kept::Nothing, cx);
                    });
                })
                .into_any_element()
        }
        Kind::Enum(items) => {
            let items_vec: Vec<SharedString> =
                items.iter().map(|s| s.to_string().into()).collect();
            let current = this.file.get(opt.key);
            let selected = Stored::new(current.as_deref())
                .index_in(&items_vec)
                .map(IndexPath::new);
            let (state, created) =
                this.editors
                    .select(opt.key, items_vec, selected, window, cx);
            if created {
                let key = opt.key;
                cx.subscribe(&state, move |this, _, event, cx| {
                    let SelectEvent::Confirm(value) = event;
                    this.commit(key, value.as_ref().map(|v| v.as_ref()), Kept::Select, cx);
                })
                .detach();
            }
            div()
                .max_w(px(260.))
                .child(Select::new(&state).small())
                .into_any_element()
        }
        Kind::Int { .. } | Kind::Float { .. } | Kind::Text => {
            let key = opt.key;
            let current_val = this.file.get(key).unwrap_or_default();
            let (bound_min, bound_max) = edit_bounds(opt);

            match key {
                "font-size" => render_bounded_slider_number(
                    this,
                    key,
                    &current_val,
                    13.0,
                    bound_min,
                    bound_max,
                    1.0,
                    true,
                    Some("pt"),
                    window,
                    cx,
                ),
                "background-opacity" => render_bounded_slider_number(
                    this,
                    key,
                    &current_val,
                    1.0,
                    bound_min,
                    bound_max,
                    0.05,
                    true,
                    None,
                    window,
                    cx,
                ),
                "cursor-opacity" => render_bounded_slider_number(
                    this,
                    key,
                    &current_val,
                    1.0,
                    bound_min,
                    bound_max,
                    0.05,
                    true,
                    None,
                    window,
                    cx,
                ),
                "minimum-contrast" => render_bounded_slider_number(
                    this,
                    key,
                    &current_val,
                    1.0,
                    bound_min,
                    bound_max,
                    0.5,
                    true,
                    None,
                    window,
                    cx,
                ),
                "window-width" => {
                    let num_state = this.get_or_create_number_input(key, "80", bound_min, bound_max, 10.0, window, cx);
                    let num_input = NumberInput::new(&num_state).small().suffix(
                        div().text_xs().text_color(cx.theme().muted_foreground).child(Text::new("열", "cols").s()),
                    );
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(div().w(px(120.)).child(num_input))
                        .child(render_chips_only(key, &current_val, &[("80", "80"), ("100", "100"), ("120", "120"), ("140", "140")], cx.entity(), cx))
                        .into_any_element()
                }
                "window-height" => {
                    let num_state = this.get_or_create_number_input(key, "24", bound_min, bound_max, 5.0, window, cx);
                    let num_input = NumberInput::new(&num_state).small().suffix(
                        div().text_xs().text_color(cx.theme().muted_foreground).child(Text::new("행", "rows").s()),
                    );
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(div().w(px(120.)).child(num_input))
                        .child(render_chips_only(key, &current_val, &[("24", "24"), ("30", "30"), ("40", "40"), ("50", "50")], cx.entity(), cx))
                        .into_any_element()
                }
                "font-thicken-strength" => render_bounded_slider_number(
                    this,
                    key,
                    &current_val,
                    0.0,
                    bound_min,
                    bound_max,
                    16.0,
                    false,
                    None,
                    window,
                    cx,
                ),
                "working-directory" => {
                    let state = this.get_or_create_input(key, opt.hint.s(), window, cx);
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .w(px(200.))
                                .child(Input::new(&state).small()),
                        )
                        .child(
                            Button::new("browse-working-dir")
                                .outline()
                                .small()
                                .icon(IconName::Folder)
                                .label(Text::new("찾아보기", "Browse").s())
                                .tooltip(Text::new("시스템 폴더 선택기로 디렉터리 찾아보기", "Browse for a directory with the system folder picker").s())
                                .on_click(cx.listener(|_this, _, _, cx| {
                                    cx.spawn(async move |this, cx| {
                                        let result = cx
                                            .background_spawn(async move { pick_folder() })
                                            .await;

                                        this.update(cx, |this, cx| {
                                            if let Some(path) = result {
                                                this.commit(
                                                    "working-directory",
                                                    Some(&path),
                                                    Kept::Nothing,
                                                    cx,
                                                );
                                            }
                                        })
                                        .ok();
                                    })
                                    .detach();
                                })),
                        )
                        .into_any_element()
                }
                "theme" => {
                    let theme_names = get_ghostty_themes();
                    let items_vec: Vec<SharedString> =
                        theme_names.iter().map(|s| s.clone().into()).collect();
                    let current = this.file.get("theme");
                    let selected = Stored::new(current.as_deref())
                        .index_in(&items_vec)
                        .map(IndexPath::new);
                    let (theme_select, created) =
                        this.editors
                            .select("theme", items_vec, selected, window, cx);
                    if created {
                        cx.subscribe(&theme_select, move |this, _, event, cx| {
                            let SelectEvent::Confirm(value) = event;
                            this.commit("theme", value.as_ref().map(|v| v.as_ref()), Kept::Select, cx);
                        })
                        .detach();
                    }
                    div()
                        .max_w(px(260.))
                        .child(Select::new(&theme_select).small())
                        .into_any_element()
                }
                "command" => {
                    let state = this.get_or_create_input(key, opt.hint.s(), window, cx);
                    render_input_with_chips(
                        &state,
                        key,
                        &current_val,
                        &[
                            ("/bin/zsh", "/bin/zsh"),
                            ("/bin/bash", "/bin/bash"),
                            ("fish", "/opt/homebrew/bin/fish"),
                            ("tmux", "tmux"),
                        ],
                        cx.entity(),
                        cx,
                    )
                }
                "background-blur" => {
                    let state = this.get_or_create_input(key, opt.hint.s(), window, cx);
                    render_input_with_chips(
                        &state,
                        key,
                        &current_val,
                        &[
                            (Text::new("끔 (false)", "Off (false)").s(), "false"),
                            (Text::new("은은하게 (10)", "Subtle (10)").s(), "10"),
                            (Text::new("기본 (20)", "Default (20)").s(), "20"),
                            (Text::new("강하게 (40)", "Strong (40)").s(), "40"),
                            ("Glass Regular", "macos-glass-regular"),
                            ("Glass Clear", "macos-glass-clear"),
                        ],
                        cx.entity(),
                        cx,
                    )
                }
                "scrollback-limit" => {
                    let state = this.get_or_create_input(key, opt.hint.s(), window, cx);
                    render_input_with_chips(
                        &state,
                        key,
                        &current_val,
                        &[
                            ("10MB", "10000000"),
                            ("50MB", "50000000"),
                            ("100MB", "100000000"),
                            ("500MB", "500000000"),
                            ("1GB", "1000000000"),
                            (Text::new("무제한 (0)", "Unlimited (0)").s(), "0"),
                        ],
                        cx.entity(),
                        cx,
                    )
                }
                "window-padding-x" | "window-padding-y" => {
                    let state = this.get_or_create_input(key, opt.hint.s(), window, cx);
                    render_input_with_chips(
                        &state,
                        key,
                        &current_val,
                        &[
                            ("0", "0"),
                            ("4", "4"),
                            ("8", "8"),
                            ("12", "12"),
                            ("16", "16"),
                            ("24", "24"),
                        ],
                        cx.entity(),
                        cx,
                    )
                }
                "mouse-scroll-multiplier" => {
                    let state = this.get_or_create_input(key, opt.hint.s(), window, cx);
                    render_input_with_chips(
                        &state,
                        key,
                        &current_val,
                        &[
                            (Text::new("1x (느림)", "1x (slow)").s(), "1"),
                            ("2x", "2"),
                            (Text::new("3x (기본)", "3x (default)").s(), "3"),
                            (Text::new("5x (빠름)", "5x (fast)").s(), "5"),
                        ],
                        cx.entity(),
                        cx,
                    )
                }
                "adjust-cell-width" | "adjust-cell-height" => {
                    let state = this.get_or_create_input(key, opt.hint.s(), window, cx);
                    render_input_with_chips(
                        &state,
                        key,
                        &current_val,
                        &[
                            ("-1", "-1"),
                            ("0", "0"),
                            ("+1", "1"),
                            ("+2", "2"),
                            ("-5%", "-5%"),
                            ("+5%", "5%"),
                            ("+10%", "10%"),
                        ],
                        cx.entity(),
                        cx,
                    )
                }
                "selection-word-chars" => {
                    let state = this.get_or_create_input(key, opt.hint.s(), window, cx);
                    render_input_with_chips(
                        &state,
                        key,
                        &current_val,
                        &[(Text::new("기본값 복원", "Restore default").s(), "\\\\t'\\\"│`|:;,()[]{}<>$")],
                        cx.entity(),
                        cx,
                    )
                }
                _ => {
                    let state = this.get_or_create_input(key, opt.hint.s(), window, cx);
                    div()
                        .max_w(px(260.))
                        .child(Input::new(&state).small())
                        .into_any_element()
                }
            }
        }
        Kind::Color { .. } => {
            let current = Stored::new(this.file.get(opt.key).as_deref()).color();
            let (state, created) = this.editors.color(opt.key, current, window, cx);
            if created {
                let key = opt.key;
                cx.subscribe(&state, move |this, _, event, cx| {
                    if let ColorPickerEvent::Change(Some(color)) = event {
                        this.commit(key, Some(&value::hex(*color)), Kept::Color, cx);
                    }
                })
                .detach();
            }
            let current_text = this.file.get(opt.key).unwrap_or_default();
            h_flex()
                .items_center()
                .gap_2()
                .child(ColorPicker::new(&state).small())
                .children(if !current_text.is_empty() {
                    Some(
                        div()
                            .px_2()
                            .py(px(2.))
                            .rounded_md()
                            .bg(cx.theme().muted)
                            .border_1()
                            .border_color(cx.theme().border)
                            .text_xs()
                            .font_family("Menlo")
                            .text_color(cx.theme().foreground)
                            .child(current_text),
                    )
                } else {
                    None
                })
                .into_any_element()
        }
        Kind::List => {
            let all = this.file.get_all(opt.key);
            let key = opt.key;
            let view = cx.entity();
            let is_keybind = key == "keybind";

            h_flex()
                .gap_2()
                .items_center()
                .child(
                    Button::new(format!("edit-{key}"))
                        .outline()
                        .xsmall()
                        .icon(if is_keybind { IconName::Keyboard } else { IconName::Pencil })
                        .label(if all.is_empty() {
                            Text::new("항목 추가…", "Add item…").s().to_string()
                        } else {
                            Text::new("편집 ({}개)", "Edit ({})").fill(&[&all.len().to_string()])
                        })
                        .on_click(move |_, window, cx| {
                            view.update(cx, |this, cx| {
                                let items = this.file.get_all(key);
                                let recorder_focus = cx.focus_handle();
                                let new_item_input = cx.new(|cx| InputState::new(window, cx).placeholder(Text::new("새 항목 입력", "Enter new item").s()));
                                let action_select = if is_keybind {
                                    let action_items: Vec<SharedString> = GHOSTTY_ACTIONS
                                        .iter()
                                        .map(|(act, desc)| format!("{act} · {}", desc.s()).into())
                                        .collect();
                                    let state = cx.new(|cx| {
                                        SelectState::new(
                                            SearchableVec::new(action_items),
                                            Some(IndexPath::new(0)),
                                            window,
                                            cx,
                                        )
                                    });
                                    cx.subscribe(&state, |this, _, event, cx| {
                                        let SelectEvent::Confirm(value) = event;
                                        if let Some(val) = value {
                                            let act = val.split(" · ").next().unwrap_or(val.as_ref()).to_string();
                                            if let Some(modal) = this.list_editor() {
                                                modal.selected_action = act;
                                                cx.notify();
                                            }
                                        }
                                    }).detach();
                                    Some(state)
                                } else {
                                    None
                                };
                                let font_select = if key == "font-family" {
                                    let fonts = get_system_fonts();
                                    let font_items: Vec<SharedString> =
                                        fonts.iter().map(|f| f.clone().into()).collect();
                                    let state = cx.new(|cx| {
                                        SelectState::new(
                                            SearchableVec::new(font_items),
                                            Some(IndexPath::new(0)),
                                            window,
                                            cx,
                                        )
                                    });
                                    cx.subscribe(&state, |this, _, event, cx| {
                                        let SelectEvent::Confirm(value) = event;
                                        if let Some(val) = value {
                                            if let Some(modal) = this.list_editor() {
                                                modal.selected_font = val.to_string();
                                                cx.notify();
                                            }
                                        }
                                    }).detach();
                                    Some(state)
                                } else {
                                    None
                                };
                                let selected_font = if key == "font-family" {
                                    get_system_fonts().first().cloned().unwrap_or_else(|| "JetBrains Mono".into())
                                } else {
                                    String::new()
                                };
                                let selected_action = if is_keybind {
                                    GHOSTTY_ACTIONS[0].0.to_string()
                                } else {
                                    String::new()
                                };
                                this.active_modal = Some(ActiveModal::ListEditor(ListEditorModal {
                                    key,
                                    items: ListItems::from_values(items),
                                    recorded_trigger: String::new(),
                                    selected_action,
                                    action_select,
                                    font_select,
                                    selected_font,
                                    is_recording: false,
                                    recorder_focus,
                                    new_item_input,
                                }));
                                cx.notify();
                            });
                        }),
                )
                .child(
                    if all.is_empty() {
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(Text::new("미설정", "Not set").s())
                            .into_any_element()
                    } else if is_keybind {
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .children(all.iter().take(2).map(|item| {
                                let parts: Vec<&str> = item.splitn(2, '=').collect();
                                let trigger = parts[0];
                                let action = parts.get(1).unwrap_or(&"");
                                let pretty = ghostty_trigger_to_pretty(trigger);
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .px_2()
                                    .py(px(1.))
                                    .rounded_md()
                                    .bg(cx.theme().muted)
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .text_xs()
                                    .child(
                                        div()
                                            .font_family("Menlo")
                                            .font_semibold()
                                            .child(pretty),
                                    )
                                    .child(Icon::new(IconName::ArrowRight).xsmall().text_color(cx.theme().muted_foreground))
                                    .child(
                                        h_flex()
                                            .gap_1p5()
                                            .items_center()
                                            .child(div().font_medium().child(action.to_string()))
                                            .children(action_description(action).map(|desc| {
                                                div()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(format!("({desc})"))
                                            })),
                                    )
                            }))
                            .children(if all.len() > 2 {
                                Some(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(Text::new("외 {}개", "+{} more").fill(&[&(all.len() - 2).to_string()])),
                                )
                            } else {
                                None
                            })
                            .into_any_element()
                    } else {
                        h_flex()
                            .gap_1()
                            .flex_wrap()
                            .items_center()
                            .children(all.iter().take(3).map(|item| {
                                div()
                                    .px_2()
                                    .py(px(1.))
                                    .rounded_md()
                                    .bg(cx.theme().muted)
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .text_xs()
                                    .font_family("Menlo")
                                    .text_color(cx.theme().foreground)
                                    .child(item.clone())
                            }))
                            .children(if all.len() > 3 {
                                Some(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!("+{}", all.len() - 3)),
                                )
                            } else {
                                None
                            })
                            .into_any_element()
                    },
                )
                .into_any_element()
        }
        Kind::Flags(allowed_items) => {
            let current_flags: Vec<String> = Stored::new(this.file.get(opt.key).as_deref())
                .flags()
                .iter()
                .map(|flag| flag.to_string())
                .collect();
            let key = opt.key;
            let view = cx.entity();

            h_flex()
                .gap_1p5()
                .flex_wrap()
                .items_center()
                .children(allowed_items.iter().map(|&flag| {
                    let is_active = current_flags.iter().any(|f| f == flag);
                    let view = view.clone();
                    let current_flags = current_flags.clone();
                    div()
                        .id(format!("{key}-{flag}"))
                        .cursor_pointer()
                        .px_2()
                        .py(px(2.))
                        .rounded_md()
                        .border_1()
                        .text_xs()
                        .font_medium()
                        .when(is_active, |s| {
                            s.bg(cx.theme().primary.opacity(0.12))
                                .border_color(cx.theme().primary)
                                .text_color(cx.theme().primary)
                        })
                        .when(!is_active, |s| {
                            s.bg(cx.theme().muted)
                                .border_color(cx.theme().border)
                                .text_color(cx.theme().muted_foreground)
                                .hover(|s| s.border_color(cx.theme().muted_foreground))
                        })
                        .child(
                            h_flex()
                                .gap_1()
                                .items_center()
                                .children(if is_active {
                                    Some(Icon::new(IconName::Check).xsmall())
                                } else {
                                    None
                                })
                                .child(flag),
                        )
                        .on_click(move |_, _, cx| {
                            view.update(cx, |this, cx| {
                                let mut new_flags = current_flags.clone();
                                if is_active {
                                    new_flags.retain(|x| x != flag);
                                } else {
                                    new_flags.push(flag.to_string());
                                }
                                let value = value::flags_value(&new_flags);
                                this.commit(key, value.as_deref(), Kept::Nothing, cx);
                            });
                        })
                }))
                .into_any_element()
        }
    }
}

