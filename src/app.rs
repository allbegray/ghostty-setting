//! Kit-based settings view.
//!
//! Shell (TitleBar, Button, search Input, StatusBar) comes from gpui-kit;
//! config semantics still live in the unchanged line-preserving
//! [`crate::config`] core. Bool/Enum rows are click-to-cycle for now —
//! per-row Switch/Select/InputState entities are the next step.

use std::path::PathBuf;

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, TitleBar,
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    status_bar::StatusBar,
};
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, InteractiveElement as _, ParentElement as _,
    Render, StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, px,
};

use crate::config::linefile::LineFile;
use crate::config::schema::{CATEGORIES, Kind, Opt, lookup};
use crate::config::{self};

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
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("옵션 검색"));
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
        self.notice = Some("파일 내용을 다시 불러왔습니다".to_string());
        cx.notify();
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
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dirty = self.dirty();

        let titlebar = TitleBar::new()
            .child("Ghostty 설정")
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("save")
                            .primary()
                            .label(if dirty { "저장 ●" } else { "저장" })
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.dirty() {
                                    this.save(cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("revert")
                            .label("되돌리기")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.dirty() {
                                    this.revert(cx);
                                }
                            })),
                    ),
            );

        let sidebar = div()
            .flex()
            .flex_col()
            .w(px(230.0))
            .p_2()
            .gap_1()
            .child(Input::new(&self.search_input).small())
            .children(CATEGORIES.iter().enumerate().map(|(i, cat)| {
                let set = cat
                    .keys
                    .iter()
                    .filter(|k| lookup(k).is_some_and(|o| self.is_set(o)))
                    .count();
                let label = if set > 0 {
                    format!("{} ({})", cat.label, set)
                } else {
                    cat.label.to_string()
                };
                let selected = self.search.is_empty() && i == self.category;
                let mut item = div()
                    .id(format!("cat-{i}"))
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.category = i;
                        cx.notify();
                    }));
                if selected {
                    item = item.bg(cx.theme().muted);
                }
                item
            }));

        let heading = if self.search.is_empty() {
            let cat = &CATEGORIES[self.category];
            format!("{} — {}", cat.label, cat.desc)
        } else {
            format!("검색 결과 {}개", self.visible_opts().len())
        };

        let border = cx.theme().border;
        let rows = div().flex().flex_col().gap_2().children(
            self.visible_opts().into_iter().map(|opt| {
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

                let mut row = div()
                    .id(key)
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_2()
                    .rounded_md()
                    .border_1()
                    .border_color(border)
                    .child(div().w(px(150.0)).child(opt.label.to_string()))
                    .child(div().flex_1().text_sm().child(format!("`{key}`")))
                    .child(div().flex_1().child(value))
                    .child(div().w(px(90.0)).text_sm().child(if set {
                        "설정됨"
                    } else {
                        "기본값"
                    }));
                if clickable {
                    row = row
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.cycle_value(key, cx);
                        }));
                }
                if set {
                    let key2 = opt.key;
                    row = row.child(
                        div()
                            .id(format!("reset-{key2}"))
                            .px_2()
                            .py_1()
                            .text_sm()
                            .cursor_pointer()
                            .child("해제")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.reset_key(key2, cx);
                            })),
                    );
                }
                row
            }),
        );

        let content = div()
            .flex()
            .flex_col()
            .flex_1()
            .p_4()
            .gap_3()
            .child(div().text_lg().child(heading))
            .child(rows);

        let status = StatusBar::new()
            .left(
                div().text_sm().child(
                    self.notice.clone().unwrap_or_else(|| {
                        "클릭으로 켜기/끄기 · 검색은 왼쪽 입력欄".to_string()
                    }),
                ),
            )
            .right(div().text_sm().child(if dirty {
                "● 저장되지 않은 변경"
            } else {
                "○ 변경 없음"
            }));

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(titlebar)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .child(sidebar)
                    .child(content),
            )
            .child(status)
    }
}
