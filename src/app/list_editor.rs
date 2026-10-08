//! The modal that edits a repeatable option's list.
//!
//! One option key drives five different editors — a plain list, a font list, a
//! key-binding recorder, an OpenType feature list and the extra config files.
//! The shape is derived once from the key into [`ListKind`], and every arm
//! reads that one value instead of re-testing which key it holds. The list's
//! own rules live in [`super::list_items`]; the modal's state is
//! [`ListEditorModal`], reached through the view's `list_editor` accessor.

use super::*;
use super::list_items::ListItems;
use super::row_rules;

/// The state of the list-editor modal.
///
/// A named module of its own rather than eleven fields on an enum variant:
/// the renderer takes one of these, the open site constructs one, and the
/// handlers reach in through the view's accessor. `items` carries its own
/// rules (see [`super::list_items`]).
pub(crate) struct ListEditorModal {
    pub(crate) key: &'static str,
    pub(crate) items: ListItems,
    pub(crate) recorded_trigger: String,
    pub(crate) selected_action: String,
    pub(crate) action_select: Option<Entity<SelectState<SearchableVec<SharedString>>>>,
    pub(crate) font_select: Option<Entity<SelectState<SearchableVec<SharedString>>>>,
    pub(crate) selected_font: String,
    pub(crate) is_recording: bool,
    pub(crate) recorder_focus: FocusHandle,
    pub(crate) new_item_input: Entity<InputState>,
}

/// Which list a key holds, derived once from the key.
impl ListEditorModal {
    /// Add what the typed input holds, then clear it.
    ///
    /// The three text-input shapes (feature, config path, generic item) share
    /// this: read the field, add through the list, clear the field, re-render.
    /// A blank value adds nothing and leaves the field alone.
    pub(crate) fn add_typed_item(
        &mut self,
        window: &mut Window,
        cx: &mut Context<SettingsView>,
    ) -> bool {
        let value = self.new_item_input.read(cx).value().to_string();
        if !self.items.add(&value) {
            return false;
        }
        self.new_item_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        true
    }
}

pub(crate) enum ListKind {
    Plain,
    Font,
    Keybind,
    Feature,
    Config,
}

impl ListKind {
    /// The shape a key's list editor takes.
    pub(crate) fn of(key: &str) -> Self {
        match key {
            "keybind" => ListKind::Keybind,
            "font-family" => ListKind::Font,
            "font-feature" => ListKind::Feature,
            "config-file" => ListKind::Config,
            _ => ListKind::Plain,
        }
    }
}


impl SettingsView {
    pub(super) fn render_list_editor_modal(
        &self,
        modal: &ListEditorModal,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let ListEditorModal {
            key,
            items,
            recorded_trigger,
            selected_action,
            action_select,
            font_select,
            selected_font,
            is_recording,
            recorder_focus,
            new_item_input,
        } = modal;
        let key = *key;
        let is_recording = *is_recording;
        let action_select = action_select.as_ref();
        let font_select = font_select.as_ref();
        let selected_font = selected_font.as_str();
        let kind = ListKind::of(key);
        let opt_label = lookup(key).map(|o| o.label.s()).unwrap_or(key);
        let view = cx.entity();

        let icon = match kind {
            ListKind::Keybind => IconName::Keyboard,
            ListKind::Font | ListKind::Feature => IconName::Type,
            ListKind::Config => IconName::FileText,
            ListKind::Plain => IconName::Pencil,
        };

        let title = match kind {
            ListKind::Keybind => Text::new("키 바인딩 설정 (`keybind`)", "Configure key bindings (`keybind`)").s().to_string(),
            ListKind::Font => Text::new("글꼴 우선순위 설정 (`font-family`)", "Configure font priority (`font-family`)").s().to_string(),
            ListKind::Feature => Text::new("OpenType 기능 설정 (`font-feature`)", "Configure OpenType features (`font-feature`)").s().to_string(),
            ListKind::Config => Text::new("추가 설정 파일 불러오기 (`config-file`)", "Load additional config files (`config-file`)").s().to_string(),
            ListKind::Plain => Text::new("{} 목록 편집 (`{}`)", "Edit {} list (`{}`)").fill(&[opt_label, key]),
        };

        let subtitle = match kind {
            ListKind::Keybind => Text::new("단축키 입력을 녹음하고 실행할 Ghostty 동작을 지정합니다.", "Record a shortcut and choose the Ghostty action to run.").s(),
            ListKind::Font => Text::new("시스템에 설치된 폰트를 선택하거나 인기 코딩 폰트를 추가하여 우선순위를 구성합니다.", "Select a font installed on your system or add a popular coding font to set the priority order.").s(),
            ListKind::Feature => Text::new("폰트의 프로그래밍 합자(Ligatures) 및 특수 글리프 기능을 켜고 끕니다.", "Turn programming ligatures and special glyph features on or off.").s(),
            ListKind::Config => Text::new("파일 탐색기로 추가 설정 파일을 찾아보거나 직접 경로를 추가합니다.", "Browse for additional config files or add a path directly.").s(),
            ListKind::Plain => Text::new("설정 파일에 반복 지정되는 항목 목록을 관리합니다.", "Manage the list of items that can be repeated in the config file.").s(),
        };
        let header = h_flex()
            .items_center()
            .justify_between()
            .pb_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .size(px(32.))
                            .rounded_lg()
                            .bg(cx.theme().primary.opacity(0.12))
                            .text_color(cx.theme().primary)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(icon).small()),
                    )
                    .child(
                        v_flex()
                            .gap_0()
                            .child(
                                div()
                                    .font_semibold()
                                    .text_base()
                                    .child(title),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(subtitle),
                            ),
                    ),
            )
            .child(
                Button::new("close-list-modal")
                    .ghost()
                    .xsmall()
                    .icon(IconName::X)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.active_modal = None;
                        cx.notify();
                    })),
            );

        let items_list = div()
            .max_h(px(220.))
            .overflow_y_scrollbar()
            .p_1()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().muted.opacity(0.2))
            .child(
                if items.is_empty() {
                    div()
                        .py_6()
                        .flex()
                        .justify_center()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(Text::new("등록된 항목이 없습니다.", "No items yet.").s())
                        .into_any_element()
                } else {
                    v_flex()
                        .gap_1()
                        .children(items.as_slice().iter().enumerate().map(|(ix, item)| {
                            let view = view.clone();
                            if matches!(kind, ListKind::Keybind) {
                                let (trigger, action) = row_rules::binding_parts(item);
                                let pretty = ghostty_trigger_to_pretty(trigger);
                                h_flex()
                                    .id(format!("binding-item-{ix}"))
                                    .items_center()
                                    .justify_between()
                                    .px_3()
                                    .py(px(6.))
                                    .rounded_md()
                                    .bg(cx.theme().background)
                                    .border_1()
                                    .border_color(cx.theme().border.opacity(0.5))
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .items_center()
                                            .child(
                                                div()
                                                    .px_2()
                                                    .py(px(2.))
                                                    .rounded_md()
                                                    .bg(cx.theme().muted)
                                                    .border_1()
                                                    .border_color(cx.theme().border)
                                                    .font_family("Menlo")
                                                    .font_semibold()
                                                    .text_xs()
                                                    .child(pretty),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_family("Menlo")
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(format!("({trigger})")),
                                            )
                                            .child(Icon::new(IconName::ArrowRight).xsmall().text_color(cx.theme().muted_foreground))
                                            .child(
                                                h_flex()
                                                    .gap_1p5()
                                                    .items_center()
                                                    .child(div().text_xs().font_medium().child(action.to_string()))
                                                    .children(action_description(action).map(|desc| {
                                                        div()
                                                            .text_xs()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(format!("({desc})"))
                                                    })),
                                            ),
                                    )
                                    .child(
                                        Button::new(format!("del-binding-{ix}"))
                                            .ghost()
                                            .xsmall()
                                            .icon(IconName::Trash)
                                            .tooltip(Text::new("삭제", "Delete").s())
                                            .on_click(move |_, _, cx| {
                                                view.update(cx, |this, cx| {
                                                    if let Some(modal) = this.list_editor() {
                                                        modal.items.remove_at(ix);
                                                        cx.notify();
                                                    }
                                                });
                                            }),
                                    )
                                    .into_any_element()
                            } else {
                                h_flex()
                                    .id(format!("generic-item-{ix}"))
                                    .items_center()
                                    .justify_between()
                                    .px_3()
                                    .py(px(6.))
                                    .rounded_md()
                                    .bg(cx.theme().background)
                                    .border_1()
                                    .border_color(cx.theme().border.opacity(0.5))
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_family("Menlo")
                                            .child(item.clone()),
                                    )
                                    .child(
                                        Button::new(format!("del-item-{ix}"))
                                            .ghost()
                                            .xsmall()
                                            .icon(IconName::Trash)
                                            .tooltip(Text::new("삭제", "Delete").s())
                                            .on_click(move |_, _, cx| {
                                                view.update(cx, |this, cx| {
                                                    if let Some(modal) = this.list_editor() {
                                                        modal.items.remove_at(ix);
                                                        cx.notify();
                                                    }
                                                });
                                            }),
                                    )
                                    .into_any_element()
                            }
                        }))
                        .into_any_element()
                }
            );

        let add_section = if matches!(kind, ListKind::Keybind) {
            let trigger_val = recorded_trigger.to_string();
            let act_val = selected_action.to_string();
            let is_rec = is_recording;
            let focus = recorder_focus.clone();

            v_flex()
                .gap_2()
                .p_3()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().muted.opacity(0.15))
                .child(
                    div().text_xs().font_semibold().text_color(cx.theme().foreground).child(Text::new("새 키 바인딩 추가", "Add new key binding").s())
                )
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .id("keystroke-recorder")
                                .track_focus(&focus)
                                .cursor_pointer()
                                .flex_1()
                                .px_3()
                                .py(px(6.))
                                .rounded_md()
                                .border_1()
                                .when(is_rec, |s| {
                                    s.bg(cx.theme().primary.opacity(0.15))
                                        .border_color(cx.theme().primary)
                                        .text_color(cx.theme().primary)
                                })
                                .when(!is_rec, |s| {
                                    s.bg(cx.theme().background)
                                        .border_color(cx.theme().border)
                                        .text_color(cx.theme().foreground)
                                        .hover(|s| s.border_color(cx.theme().muted_foreground))
                                })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    if let Some(modal) = this.list_editor() {
                                        modal.is_recording = true;
                                        window.focus(&modal.recorder_focus, cx);
                                        cx.notify();
                                    }
                                }))
                                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                    if let Some(modal) = this.list_editor() {
                                        if modal.is_recording {
                                            let key_name = &event.keystroke.key;
                                            if key_name == "escape" {
                                                modal.is_recording = false;
                                                cx.notify();
                                                return;
                                            }
                                            if !is_modifier_key_name(key_name) {
                                                modal.recorded_trigger = keystroke_to_ghostty_trigger(&event.keystroke);
                                                modal.is_recording = false;
                                                cx.notify();
                                            }
                                        }
                                    }
                                }))
                                .child(
                                    if is_rec {
                                        h_flex()
                                            .gap_1p5()
                                            .items_center()
                                            .child(div().size(px(6.)).rounded_full().bg(cx.theme().warning))
                                            .child(div().text_xs().font_medium().child(Text::new("키보드 입력 대기 중... (Esc: 취소)", "Waiting for keyboard input… (Esc to cancel)").s()))
                                    } else if trigger_val.is_empty() {
                                        h_flex()
                                            .gap_1p5()
                                            .items_center()
                                            .child(Icon::new(IconName::Keyboard).xsmall().text_color(cx.theme().muted_foreground))
                                            .child(div().text_xs().text_color(cx.theme().muted_foreground).child(Text::new("클릭하여 단축키 입력...", "Click to enter a shortcut…").s()))
                                    } else {
                                        h_flex()
                                            .gap_1p5()
                                            .items_center()
                                            .child(
                                                div()
                                                    .px_1p5()
                                                    .py(px(1.))
                                                    .rounded_sm()
                                                    .bg(cx.theme().muted)
                                                    .font_family("Menlo")
                                                    .font_semibold()
                                                    .text_xs()
                                                    .child(ghostty_trigger_to_pretty(&trigger_val)),
                                            )
                                            .child(div().text_xs().text_color(cx.theme().muted_foreground).child(Text::new("({}) - 재입력 클릭", "({}) - click to re-enter").fill(&[&trigger_val])))
                                    }
                                )
                        )
                        .child(
                            if let Some(sel) = action_select {
                                div()
                                    .flex_1()
                                    .child(Select::new(sel).small())
                            } else {
                                div().flex_1()
                            }
                        )
                        .child(
                            Button::new("add-binding-btn")
                                .primary()
                                .small()
                                .icon(IconName::Plus)
                                .label(Text::new("추가", "Add").s())
                                .disabled(trigger_val.is_empty())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(modal) = this.list_editor() {
                                        if !modal.recorded_trigger.is_empty() && !modal.selected_action.trim().is_empty() {
                                            let entry = format!("{}={}", modal.recorded_trigger, modal.selected_action.trim());
                                            modal.items.add(&entry);
                                            modal.recorded_trigger.clear();
                                            cx.notify();
                                        }
                                    }
                                }))
                        )
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child(Text::new("자주 쓰는 동작 빠른 선택 (클릭 시 자동 선택):", "Common actions (click to select automatically):").s()))
                        .child(
                            h_flex()
                                .gap_1()
                                .flex_wrap()
                                .children(GHOSTTY_ACTIONS.iter().take(8).map(|(act, desc)| {
                                    let act_str = act.to_string();
                                    let view = view.clone();
                                    let is_sel = act_val == *act;
                                    div()
                                        .id(format!("quick-action-{act}"))
                                        .cursor_pointer()
                                        .px_1p5()
                                        .py(px(1.))
                                        .rounded_sm()
                                        .border_1()
                                        .text_xs()
                                        .when(is_sel, |s| {
                                            s.bg(cx.theme().primary.opacity(0.12))
                                                .border_color(cx.theme().primary)
                                                .text_color(cx.theme().primary)
                                        })
                                        .when(!is_sel, |s| {
                                            s.bg(cx.theme().background)
                                                .border_color(cx.theme().border)
                                                .text_color(cx.theme().muted_foreground)
                                                .hover(|s| s.text_color(cx.theme().foreground))
                                        })
                                        .child(format!("{} ({})", desc.s(), act))
                                        .on_click(move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                if let Some(modal) = this.list_editor() {
                                                    modal.selected_action = act_str.clone();
                                                    cx.notify();
                                                }
                                            });
                                        })
                                }))
                        )
                )
                .into_any_element()
        } else if matches!(kind, ListKind::Font) {
            let sel_font = selected_font.to_string();
            let view = view.clone();
            v_flex()
                .gap_2()
                .p_3()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().muted.opacity(0.15))
                .child(
                    div().text_xs().font_semibold().text_color(cx.theme().foreground).child(Text::new("새 글꼴 추가 (시스템 설치 폰트 선택)", "Add new font (pick from system fonts)").s())
                )
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            if let Some(font_sel) = font_select {
                                div()
                                    .flex_1()
                                    .child(Select::new(font_sel).small())
                            } else {
                                div().flex_1()
                            }
                        )
                        .child(
                            Button::new("add-font-btn")
                                .primary()
                                .small()
                                .icon(IconName::Plus)
                                .label(Text::new("글꼴 추가", "Add font").s())
                                .disabled(sel_font.is_empty())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(modal) = this.list_editor() {
                                        if !modal.selected_font.trim().is_empty() {
                                            if modal.items.add_unique(&modal.selected_font) {
                                                cx.notify();
                                            }
                                        }
                                    }
                                }))
                        )
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child(Text::new("인기 코딩 폰트 빠른 추가:", "Quick add popular coding fonts:").s()))
                        .child(
                            h_flex()
                                .gap_1()
                                .flex_wrap()
                                .children(controls::POPULAR_FONTS.iter().map(|&font_name| {
                                    let view = view.clone();
                                    div()
                                        .id(format!("popular-font-{font_name}"))
                                        .cursor_pointer()
                                        .px_1p5()
                                        .py(px(1.))
                                        .rounded_sm()
                                        .bg(cx.theme().background)
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .hover(|s| s.text_color(cx.theme().foreground))
                                        .child(format!("+ {font_name}"))
                                        .on_click(move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                if let Some(modal) = this.list_editor() {
                                                    if modal.items.add_unique(font_name) {
                                                        cx.notify();
                                                    }
                                                }
                                            });
                                        })
                                }))
                        )
                )
                .into_any_element()
        } else if matches!(kind, ListKind::Config) {
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .flex_1()
                        .child(Input::new(new_item_input).small())
                )
                .child(
                    Button::new("browse-config-file-btn")
                        .outline()
                        .small()
                        .icon(IconName::FileText)
                        .label(Text::new("파일 찾아보기", "Browse file").s())
                        .on_click(cx.listener(|_this, _, _, cx| {
                            cx.spawn(async move |this, cx| {
                                let result = cx.background_spawn(async move {
                                    pick_file()
                                }).await;

                                this.update(cx, |this, cx| {
                                    if let Some(path) = result {
                                        if let Some(modal) = this.list_editor() {
                                            if modal.items.add(&path) {
                                                cx.notify();
                                            }
                                        }
                                    }
                                }).ok();
                            }).detach();
                        }))
                )
                .child(
                    Button::new("add-config-file-btn")
                        .primary()
                        .small()
                        .icon(IconName::Plus)
                        .label(Text::new("경로 추가", "Add path").s())
                        .on_click(cx.listener(|this, _, window, cx| {
                            if let Some(modal) = this.list_editor() {
                                if modal.add_typed_item(window, cx) {
                                    cx.notify();
                                }
                            }
                        }))
                )
                .into_any_element()
        } else if matches!(kind, ListKind::Feature) {
            const POPULAR_FEATURES: &[(&str, Text)] = &[
                ("-calt", Text::new("합자 끄기", "Ligatures off")),
                ("+calt", Text::new("합자 켜기", "Ligatures on")),
                ("+liga", Text::new("기본 합자", "Standard ligatures")),
                ("+dlig", Text::new("임의 합자", "Discretionary ligatures")),
                ("+zero", Text::new("슬래시 0", "Slashed zero")),
                ("+ss01", Text::new("스타일셋 1", "Stylistic set 1")),
                ("+ss02", Text::new("스타일셋 2", "Stylistic set 2")),
                ("+cv01", Text::new("문자변형 1", "Character variant 1")),
            ];
            let view = view.clone();
            v_flex()
                .gap_2()
                .p_3()
                .rounded_lg()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().muted.opacity(0.15))
                .child(
                    div().text_xs().font_semibold().text_color(cx.theme().foreground).child(Text::new("새 OpenType 기능 추가", "Add new OpenType feature").s())
                )
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .child(Input::new(new_item_input).small())
                        )
                        .child(
                            Button::new("add-feature-btn")
                                .primary()
                                .small()
                                .icon(IconName::Plus)
                                .label(Text::new("추가", "Add").s())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    if let Some(modal) = this.list_editor() {
                                        if modal.add_typed_item(window, cx) {
                                            cx.notify();
                                        }
                                    }
                                }))
                        )
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child(Text::new("자주 쓰는 기능 빠른 추가 (1클릭):", "Quick add common features (1 click):").s()))
                        .child(
                            h_flex()
                                .gap_1()
                                .flex_wrap()
                                .children(POPULAR_FEATURES.iter().map(|(feat, desc)| {
                                    let view = view.clone();
                                    div()
                                        .id(format!("pop-feat-{feat}"))
                                        .cursor_pointer()
                                        .px_1p5()
                                        .py(px(1.))
                                        .rounded_sm()
                                        .bg(cx.theme().background)
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .hover(|s| s.text_color(cx.theme().foreground))
                                        .child(format!("{feat} ({})", desc.s()))
                                        .on_click(move |_, _, cx| {
                                            view.update(cx, |this, cx| {
                                                if let Some(modal) = this.list_editor() {
                                                    if modal.items.add_unique(feat) {
                                                        cx.notify();
                                                    }
                                                }
                                            });
                                        })
                                }))
                        )
                )
                .into_any_element()
        } else {
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .flex_1()
                        .child(Input::new(new_item_input).small())
                )
                .child(
                    Button::new("add-generic-item-btn")
                        .primary()
                        .small()
                        .icon(IconName::Plus)
                        .label(Text::new("항목 추가", "Add item").s())
                        .on_click(cx.listener(|this, _, window, cx| {
                            if let Some(modal) = this.list_editor() {
                                if modal.add_typed_item(window, cx) {
                                    cx.notify();
                                }
                            }
                        }))
                )
                .into_any_element()
        };

        let footer = h_flex()
            .items_center()
            .justify_end()
            .gap_2()
            .pt_3()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("cancel-list-modal")
                    .ghost()
                    .small()
                    .label(Text::new("취소", "Cancel").s())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.active_modal = None;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("apply-list-modal")
                    .primary()
                    .small()
                    .icon(IconName::Check)
                    .label(Text::new("적용하기", "Apply").s())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(ActiveModal::ListEditor(modal)) = &this.active_modal {
                            let k = modal.key;
                            let items_clone = modal.items.as_slice().to_vec();
                            this.commit_all(k, &items_clone, cx);
                            this.active_modal = None;
                            this.notice = Some(
                                Text::new("'{}' 설정이 반영되었습니다.", "'{}' settings applied.").fill(&[k]),
                            );
                            cx.notify();
                        }
                    })),
            );

        div()
            .absolute()
            .inset_0()
            .bg(cx.theme().background.opacity(0.8))
            .flex()
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .w(px(640.))
                    .max_h(px(580.))
                    .rounded_xl()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .p_4()
                    .gap_3()
                    .child(header)
                    .child(items_list)
                    .child(add_section)
                    .child(footer)
            )
            .into_any_element()
    }
}
