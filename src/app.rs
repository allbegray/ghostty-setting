//! Settings workspace: sidebar navigation beside the option detail view.
//!
//! Task: find one option among ~68, change its value, save. The composition
//! follows the kit guides — semantic components (`Sidebar`, `Button`,
//! `Input`, `StatusBar`, `Badge`, `Label`), theme tokens only, rem-based
//! geometry, one scroll owner (the option list). Config semantics still live
//! in the unchanged line-preserving [`crate::config`] core.
//!
//! Deliberate deviations, with reasons:
//! - Rows are custom lanes instead of `Form`/`Field`: every row needs the
//!   same four lanes (identity / value / state / trailing action) and `Form`
//!   owns a label column that would fight the state and action lanes.
//! - Bool/Enum rows still cycle on click: the model is tri-state
//!   (unset/true/false) and `Switch` can only express two. A tri-state
//!   control is a separate design decision, not part of this rewrite.

use std::path::PathBuf;

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, TitleBar,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
    input::{Input, InputEvent, InputState},
    label::Label,
    scroll::ScrollableElement as _,
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
    status_bar::StatusBar,
    tooltip::Tooltip,
};
use gpui_kit::{
    AppContext as _, Context, Entity, Focusable as _, IntoElement, InteractiveElement as _,
    ParentElement as _, Render, StatefulInteractiveElement as _, Styled as _, Subscription,
    Window, actions, div, px,
};
use gpui_kit::base::Disableable as _;

use crate::config::linefile::LineFile;
use crate::config::schema::{CATEGORIES, Kind, Opt, lookup};
use crate::config::{self};

actions!(settings, [Save, FocusSearch]);

pub struct SettingsView {
    path: PathBuf,
    file: LineFile,
    original: String,
    category: usize,
    search: String,
    search_input: Entity<InputState>,
    notice: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl SettingsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, path: Option<PathBuf>) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("옵션 검색  ( / )"));
        let subscription = cx.subscribe_in(&search_input, window, |this, state, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.search = state.read(cx).value().to_string();
                cx.notify();
            }
        });
        let path = path.unwrap_or_else(config::default_path);
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        Self {
            path,
            file: LineFile::parse(&text),
            original: text,
            category: 0,
            search: String::new(),
            search_input,
            notice: None,
            _subscriptions: vec![subscription],
        }
    }

    fn dirty(&self) -> bool {
        self.file.render() != self.original
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        if let Some(parent) = self.path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                self.notice = Some(format!("디렉터리 생성 실패: {e}"));
                cx.notify();
                return;
            }
        }
        let rendered = self.file.render();
        match std::fs::write(&self.path, &rendered) {
            Ok(()) => {
                self.original = rendered;
                self.notice = Some(format!("저장됨 · {}", self.path.display()));
            }
            Err(e) => self.notice = Some(format!("저장 실패: {e}")),
        }
        cx.notify();
    }

    fn revert(&mut self, cx: &mut Context<Self>) {
        let text = std::fs::read_to_string(&self.path).unwrap_or_default();
        self.file = LineFile::parse(&text);
        self.original = text;
        self.notice = Some("파일 내용을 다시 불러왔습니다.".to_string());
        cx.notify();
    }

    fn clear_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_input.update(cx, |input, cx| {
            input.set_value("", window, cx);
        });
        // set_value emits no Change event, so mirror the state here.
        self.search.clear();
        cx.notify();
    }

    fn commit_save(&mut self, _: &Save, _: &mut Window, cx: &mut Context<Self>) {
        if self.dirty() {
            self.save(cx);
        }
    }

    fn focus_search(&mut self, _: &FocusSearch, window: &mut Window, cx: &mut Context<Self>) {
        let handle = self.search_input.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
    }

    fn is_set(&self, opt: &Opt) -> bool {
        if opt.repeatable() {
            self.file.get_all(opt.key).iter().any(|v| !v.is_empty())
        } else {
            self.file.get(opt.key).is_some()
        }
    }

    /// Bool rows cycle unset → true → false → unset;
    /// Enum rows cycle unset → each value → unset.
    fn cycle_value(&mut self, key: &'static str, cx: &mut Context<Self>) {
        let Some(opt) = lookup(key) else { return };
        match opt.kind {
            Kind::Bool => {
                let next = match self.file.get(key).as_deref() {
                    None => Some("true".to_string()),
                    Some("true") => Some("false".to_string()),
                    _ => None,
                };
                match next {
                    Some(v) => self.file.set(key, &v),
                    None => self.file.remove(key),
                }
                cx.notify();
            }
            Kind::Enum(items) => {
                let cur = self.file.get(key);
                let next = match cur.as_deref() {
                    None => Some(Some(items[0].to_string())),
                    Some(c) => match items.iter().position(|i| *i == c) {
                        Some(i) if i + 1 < items.len() => Some(Some(items[i + 1].to_string())),
                        _ => Some(None),
                    },
                };
                match next {
                    Some(Some(v)) => self.file.set(key, &v),
                    _ => self.file.remove(key),
                }
                cx.notify();
            }
            _ => {}
        }
    }

    fn reset_key(&mut self, key: &str, cx: &mut Context<Self>) {
        self.file.remove(key);
        cx.notify();
    }

    fn visible_opts(&self) -> Vec<&'static Opt> {
        let q = self.search.trim().to_lowercase();
        if q.is_empty() {
            CATEGORIES[self.category]
                .keys
                .iter()
                .filter_map(|k| lookup(k))
                .collect()
        } else {
            crate::config::schema::OPTS
                .iter()
                .filter(|o| {
                    o.key.contains(&q)
                        || o.label.to_lowercase().contains(&q)
                        || o.doc.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    fn set_count(&self) -> usize {
        self.visible_opts()
            .iter()
            .filter(|o| self.is_set(o))
            .count()
    }
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dirty = self.dirty();

        let titlebar = TitleBar::new()
            .child(
                h_flex()
                    .gap_2()
                    .child("Ghostty 설정")
                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child(
                        self.path
                            .strip_prefix(std::env::var("HOME").unwrap_or_default())
                            .map(|p| format!("~{}", p.display()))
                            .unwrap_or_else(|_| self.path.display().to_string()),
                    )),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("revert")
                            .small()
                            .label("되돌리기")
                            .disabled(!dirty)
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.dirty() {
                                    this.revert(cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("save")
                            .small()
                            .primary()
                            .label(if dirty { "저장 ●" } else { "저장" })
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.dirty() {
                                    this.save(cx);
                                }
                            })),
                    ),
            );

        let view = cx.entity();
        let menu = SidebarMenu::new().children(CATEGORIES.iter().enumerate().map(|(i, cat)| {
            let set = cat
                .keys
                .iter()
                .filter(|k| lookup(k).is_some_and(|o| self.is_set(o)))
                .count();
            let selected = self.search.is_empty() && i == self.category;
            let view = view.clone();
            // Keep counts neutral: a number needs no semantic Badge variant.
            SidebarMenuItem::new(cat.label)
                .active(selected)
                .suffix(move |_, _| {
                    // Neutral count: a number does not earn a Badge variant.
                    if set > 0 {
                        div().text_sm().child(set.to_string()).into_any_element()
                    } else {
                        div().into_any_element()
                    }
                })
                .on_click(move |_, _, cx| {
                    view.update(cx, |this, cx| {
                        this.category = i;
                        cx.notify();
                    });
                })
        }));
        let sidebar = Sidebar::new("nav")
            .collapsible(false)
            .header(Input::new(&self.search_input).small())
            .child(menu);

        let heading = if self.search.is_empty() {
            let cat = &CATEGORIES[self.category];
            (cat.label.to_string(), cat.desc.to_string())
        } else {
            (
                format!("검색 결과 {}개", self.visible_opts().len()),
                "키, 이름, 설명에서 찾습니다.".to_string(),
            )
        };
        let counts = format!("{}/{} 설정됨", self.set_count(), self.visible_opts().len());
        let header = v_flex()
            .gap_1()
            .child(
                h_flex()
                    .gap_2()
                    .items_baseline()
                    .child(div().text_lg().child(heading.0))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(counts),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(heading.1),
            )
            .child(
                // Column titles share the row grid below: one lane, one title.
                h_flex()
                    .gap_3()
                    .pt_2()
                    .pb_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .w(px(240.))
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("옵션"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("값"),
                    )
                    .child(
                        div()
                            .w(px(72.))
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("상태"),
                    )
                    .child(div().w(px(56.))),
            );

        let hover_bg = cx.theme().muted;
        let border = cx.theme().border;
        let list = if self.visible_opts().is_empty() {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_3()
                .py_16()
                .child(div().text_color(cx.theme().muted_foreground).child("일치하는 옵션이 없습니다."))
                .child(
                    Button::new("clear-search")
                        .ghost()
                        .label("검색어 지우기")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.clear_search(window, cx);
                        })),
                )
        } else {
            v_flex().children(self.visible_opts().into_iter().map(|opt| {
                let value = if opt.repeatable() {
                    let all = self.file.get_all(opt.key);
                    if all.is_empty() {
                        "미설정".to_string()
                    } else {
                        all.join(" · ")
                    }
                } else {
                    self.file.get(opt.key).unwrap_or_else(|| "기본값".to_string())
                };
                let set = self.is_set(opt);
                let clickable = matches!(opt.kind, Kind::Bool | Kind::Enum(_));
                let key = opt.key;
                let doc = opt.doc;

                let mut row = h_flex()
                    .id(key)
                    .items_center()
                    .gap_3()
                    .py_2()
                    .border_b_1()
                    .border_color(border)
                    .child(
                        v_flex()
                            .w(px(240.))
                            .gap_0()
                            .child(Label::new(opt.label.to_string()))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("`{key}`")),
                            ),
                    )
                    .child(div().flex_1().child(value))
                    .child(div().w(px(72.)).text_sm().child(if set {
                        "설정됨"
                    } else {
                        "기본값"
                    }))
                    .child(
                        div().w(px(56.)).child(if set {
                            let key2 = opt.key;
                            Button::new(format!("reset-{key2}"))
                                .outline()
                                .xsmall()
                                .label("해제")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.reset_key(key2, cx);
                                }))
                                .into_any_element()
                        } else {
                            div().into_any_element()
                        }),
                    );
                if clickable {
                    row = row
                        .cursor_pointer()
                        .hover(move |s| s.bg(hover_bg))
                        .tooltip(move |window, cx| Tooltip::new(doc).build(window, cx))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.cycle_value(key, cx);
                        }));
                }
                row
            }))
        };

        let content = v_flex()
            .flex_1()
            .min_w_0()
            .child(div().px_4().pt_4().child(header))
            .child(
                div()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .id("option-list")
                    .px_4()
                    .pb_4()
                    .child(list),
            );

        let status = StatusBar::new()
            .left(div().text_sm().child(
                self.notice
                    .clone()
                    .unwrap_or_else(|| "클릭으로 켜기/끄기 · / 로 검색 이동 · ⌘S 저장".to_string()),
            ))
            .right(div().text_sm().child(if dirty {
                "● 저장되지 않은 변경"
            } else {
                "○ 변경 없음"
            }));

        v_flex()
            .size_full()
            .key_context("Settings")
            .on_action(cx.listener(Self::commit_save))
            .on_action(cx.listener(Self::focus_search))
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(titlebar)
            .child(h_flex().items_stretch().flex_1().child(sidebar).child(content))
            .child(status)
    }
}
