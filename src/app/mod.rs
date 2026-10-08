//! Settings workspace: sidebar navigation beside the option detail view.
//!
//! Task: find one option among ~68, change its value, save. The composition
//! follows the kit guides — semantic components (`Sidebar`, `Button`,
//! `Input`, `StatusBar`, `Badge`, `Label`), theme tokens only, rem-based
//! geometry, one scroll owner (the option list). Config semantics still live
//! in the unchanged line-preserving [`crate::config`] core.
//!
//! Deliberate design choices:
//! - `Bool` rows use `Switch`, with the "해제" action resetting to default/unset.
//! - `Enum` rows use `Select` dropdown, reset clears to default.
//! - `Int`, `Float`, `Text` rows use `Input` with `opt.hint` placeholder.
//! - `Color` rows provide `ColorPicker` with current hex value display.
//! - `List` and `Flags` rows display clean read-only summaries.
//! - Option label column includes platform badge and full `opt.doc` tooltip.
//! - 4-lane aligned layout (option, value, state, reset action).

use std::path::PathBuf;
use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, TitleBar,
    IndexPath,
    button::{Button, ButtonVariants as _},
    color_picker::{ColorPicker, ColorPickerEvent},
    h_flex,
    hover_card::HoverCard,
    link::Link,
    v_flex,
    input::{Input, InputEvent, InputState, NumberInput},
    scroll::{ScrollableElement as _, Scrollbar, ScrollbarMode},
    select::{Select, SelectEvent, SelectState},
    searchable_list::SearchableVec,
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
    slider::{Slider, SliderEvent},
    status_bar::StatusBar,
    switch::Switch,
    tooltip::Tooltip,
    Icon,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    Anchor, AppContext as _, Context, Entity, FocusHandle, Focusable as _, IntoElement,
    InteractiveElement as _, KeyDownEvent, ParentElement as _, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window, actions,
    div, px,
};
use gpui_kit::base::Disableable as _;

mod editor;
mod editors;
mod list_editor;
mod preview;
mod value;

use editors::{EditorCache, Kept};

use crate::config::linefile::LineFile;
use crate::config::schema::{CATEGORIES, Kind, Opt, lookup};
use crate::config::{self};
use crate::i18n::{self, Lang, Text};

/// Icon for a sidebar destination, keyed by `Category::id`.
///
/// Keying off identity rather than position is what makes reordering
/// `CATEGORIES` a data-only change.
fn category_icon(id: &str) -> IconName {
    match id {
        "font" => IconName::Type,
        "theme" => IconName::Palette,
        "window" => IconName::AppWindow,
        "tabs" => IconName::PanelsTopLeft,
        "cursor" => IconName::MousePointer,
        "keys" => IconName::Keyboard,
        "selection" => IconName::Clipboard,
        "shell" => IconName::Terminal,
        "system" => IconName::Settings,
        _ => IconName::Zap,
    }
}

/// Editing ranges the UI narrows below the range Ghostty accepts.
///
/// The schema owns what is valid; a control may offer less so the useful part
/// is reachable. Every entry must stay inside `Kind::bounds()` —
/// `ui_ranges_stay_inside_schema_bounds` enforces that.
const UI_RANGES: &[(&str, f64, f64)] = &[
    ("font-size", 8.0, 72.0),
    ("window-width", 20.0, 500.0),
    ("window-height", 10.0, 200.0),
];

/// The range a numeric control offers: the narrowed UI range when one is
/// declared, otherwise the schema's valid range.
fn edit_bounds(opt: &Opt) -> (f64, f64) {
    if let Some((_, min, max)) = UI_RANGES.iter().find(|(key, _, _)| *key == opt.key) {
        return (*min, *max);
    }
    opt.kind.bounds().unwrap_or((0.0, 100.0))
}
actions!(settings, [Save, FocusSearch]);
pub const GHOSTTY_ACTIONS: &[(&str, Text)] = &[
    ("copy_to_clipboard", Text::new("클립보드에 복사", "Copy to clipboard")),
    ("paste_from_clipboard", Text::new("클립보드에서 붙여넣기", "Paste from clipboard")),
    ("paste_from_selection", Text::new("선택 영역 붙여넣기", "Paste from selection")),
    ("copy_url_to_clipboard", Text::new("마지막 URL 복사", "Copy last URL")),
    ("copy_title_to_clipboard", Text::new("창 제목 복사", "Copy window title")),
    ("select_all", Text::new("전체 선택", "Select all")),
    ("adjust_selection:left", Text::new("선택 영역 왼쪽 확장", "Extend selection left")),
    ("adjust_selection:right", Text::new("선택 영역 오른쪽 확장", "Extend selection right")),
    ("new_window", Text::new("새 창 열기", "Open new window")),
    ("new_tab", Text::new("새 탭 열기", "Open new tab")),
    ("close_tab", Text::new("현재 탭 닫기", "Close current tab")),
    ("close_surface", Text::new("현재 서피스(분할) 닫기", "Close current surface (split)")),
    ("close_window", Text::new("현재 창 닫기", "Close current window")),
    ("close_all_windows", Text::new("모든 창 닫기", "Close all windows")),
    ("previous_tab", Text::new("이전 탭으로 이동", "Go to previous tab")),
    ("next_tab", Text::new("다음 탭으로 이동", "Go to next tab")),
    ("last_tab", Text::new("마지막 탭으로 이동", "Go to last tab")),
    ("goto_tab:1", Text::new("1번 탭으로 이동", "Go to tab 1")),
    ("goto_tab:2", Text::new("2번 탭으로 이동", "Go to tab 2")),
    ("goto_tab:3", Text::new("3번 탭으로 이동", "Go to tab 3")),
    ("goto_tab:4", Text::new("4번 탭으로 이동", "Go to tab 4")),
    ("goto_tab:5", Text::new("5번 탭으로 이동", "Go to tab 5")),
    ("toggle_tab_overview", Text::new("탭 개요(오버뷰) 토글", "Toggle tab overview")),
    ("new_split:right", Text::new("오른쪽에 분할 창 생성", "Create split on the right")),
    ("new_split:down", Text::new("아래쪽에 분할 창 생성", "Create split below")),
    ("goto_split:next", Text::new("다음 분할 창으로 이동", "Go to next split")),
    ("goto_split:previous", Text::new("이전 분할 창으로 이동", "Go to previous split")),
    ("goto_split:top", Text::new("위쪽 분할 창으로 이동", "Go to split above")),
    ("goto_split:bottom", Text::new("아래쪽 분할 창으로 이동", "Go to split below")),
    ("goto_split:left", Text::new("왼쪽 분할 창으로 이동", "Go to split on the left")),
    ("goto_split:right", Text::new("오른쪽 분할 창으로 이동", "Go to split on the right")),
    ("toggle_split_zoom", Text::new("분할 창 확대/축소 토글", "Toggle split zoom")),
    ("equalize_splits", Text::new("분할 창 크기 균등화", "Equalize splits")),
    ("clear_screen", Text::new("화면 지우기", "Clear screen")),
    ("scroll_to_top", Text::new("맨 위로 스크롤", "Scroll to top")),
    ("scroll_to_bottom", Text::new("맨 아래로 스크롤", "Scroll to bottom")),
    ("scroll_page_up", Text::new("한 페이지 위로 스크롤", "Scroll page up")),
    ("scroll_page_down", Text::new("한 페이지 아래로 스크롤", "Scroll page down")),
    ("jump_to_prompt:1", Text::new("다음 쉘 프롬프트로 이동", "Go to next shell prompt")),
    ("jump_to_prompt:-1", Text::new("이전 쉘 프롬프트로 이동", "Go to previous shell prompt")),
    ("increase_font_size:1", Text::new("글꼴 크기 확대 (+1)", "Increase font size (+1)")),
    ("decrease_font_size:1", Text::new("글꼴 크기 축소 (-1)", "Decrease font size (-1)")),
    ("reset_font_size", Text::new("글꼴 크기 기본값 초기화", "Reset font size to default")),
    ("toggle_fullscreen", Text::new("전체화면 토글", "Toggle fullscreen")),
    ("toggle_maximize", Text::new("창 최대화 토글", "Toggle maximized window")),
    ("toggle_quick_terminal", Text::new("퀵 터미널 토글", "Toggle quick terminal")),
    ("toggle_command_palette", Text::new("커맨드 팔레트 열기", "Open command palette")),
    ("toggle_window_decorations", Text::new("창 프레임/장식 토글", "Toggle window decorations")),
    ("toggle_window_float_on_top", Text::new("항상 위에 표시 토글", "Toggle always on top")),
    ("toggle_visibility", Text::new("창 보이기/숨기기 토글", "Toggle window visibility")),
    ("toggle_background_opacity", Text::new("배경 불투명도 토글", "Toggle background opacity")),
    ("open_config", Text::new("설정 파일 열기", "Open config file")),
    ("reload_config", Text::new("설정 파일 다시 로드", "Reload config file")),
    ("inspector", Text::new("Ghostty 인스펙터 열기", "Open Ghostty inspector")),
    ("reset", Text::new("터미널 세션 리셋", "Reset terminal session")),
    ("quit", Text::new("Ghostty 종료", "Quit Ghostty")),
];

fn action_description(action: &str) -> Option<&'static str> {
    let base = action.split(':').next().unwrap_or(action);
    GHOSTTY_ACTIONS
        .iter()
        .find(|(a, _)| *a == action || a.split(':').next() == Some(base))
        .map(|(_, d)| d.s())
}

#[derive(Clone, Debug, PartialEq)]
enum DiffLine {
    Same(String),
    Added(String),
    Removed(String),
}

fn compute_line_diff(old_text: &str, new_text: &str) -> Vec<DiffLine> {
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();

    let m = old_lines.len();
    let n = new_lines.len();
    let mut dp = vec![vec![0; n + 1]; m + 1];

    for i in 0..m {
        for j in 0..n {
            if old_lines[i] == new_lines[j] {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i][j + 1].max(dp[i + 1][j]);
            }
        }
    }

    let mut diff = Vec::new();
    let mut i = m;
    let mut j = n;
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_lines[i - 1] == new_lines[j - 1] {
            diff.push(DiffLine::Same(old_lines[i - 1].to_string()));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            diff.push(DiffLine::Added(new_lines[j - 1].to_string()));
            j -= 1;
        } else if i > 0 {
            diff.push(DiffLine::Removed(old_lines[i - 1].to_string()));
            i -= 1;
        }
    }
    diff.reverse();
    diff
}

fn is_modifier_key_name(key: &str) -> bool {
    matches!(
        key.to_lowercase().as_str(),
        "ctrl"
            | "control"
            | "alt"
            | "opt"
            | "option"
            | "shift"
            | "cmd"
            | "command"
            | "super"
            | "fn"
            | "capslock"
            | "caps_lock"
    )
}

fn keystroke_to_ghostty_trigger(keystroke: &gpui_kit::Keystroke) -> String {
    let mut parts = Vec::new();
    if keystroke.modifiers.control {
        parts.push("ctrl");
    }
    if keystroke.modifiers.alt {
        parts.push("alt");
    }
    if keystroke.modifiers.shift {
        parts.push("shift");
    }
    if keystroke.modifiers.platform {
        parts.push("super");
    }

    let lower = keystroke.key.to_lowercase();
    let key = match lower.as_str() {
        "escape" | "esc" => "esc",
        "return" | "enter" => "enter",
        "tab" => "tab",
        "space" => "space",
        "backspace" => "backspace",
        "up" | "arrowup" => "up",
        "down" | "arrowdown" => "down",
        "left" | "arrowleft" => "left",
        "right" | "arrowright" => "right",
        other => other,
    };
    parts.push(key);
    parts.join("+")
}

fn ghostty_trigger_to_pretty(trigger: &str) -> String {
    let parts: Vec<&str> = trigger.split('+').collect();
    let mut out = String::new();
    for &part in &parts {
        match part.to_lowercase().as_str() {
            "super" | "cmd" => out.push('⌘'),
            "ctrl" | "control" => out.push('⌃'),
            "alt" | "opt" | "option" => out.push('⌥'),
            "shift" => out.push('⇧'),
            "enter" | "return" => out.push_str("↵"),
            "esc" | "escape" => out.push_str("⎋"),
            "backspace" => out.push_str("⌫"),
            "tab" => out.push_str("⇥"),
            "space" => out.push_str("␣"),
            "up" | "arrowup" => out.push('↑'),
            "down" | "arrowdown" => out.push('↓'),
            "left" | "arrowleft" => out.push('←'),
            "right" | "arrowright" => out.push('→'),
            other => {
                out.push_str(&other.to_uppercase());
            }
        }
    }
    out
}

fn pick_folder() -> Option<String> {
    let prompt = Text::new("Ghostty 작업 디렉터리 선택", "Select Ghostty working directory").s();
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(format!(
            "POSIX path of (choose folder with prompt \"{prompt}\")"
        ))
        .output()
        .ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }
    None
}

fn pick_file() -> Option<String> {
    let prompt = Text::new("Ghostty 설정 파일 선택", "Select Ghostty config file").s();
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(format!("POSIX path of (choose file with prompt \"{prompt}\")"))
        .output()
        .ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }
    None
}

static SYSTEM_FONTS: std::sync::LazyLock<Vec<String>> = std::sync::LazyLock::new(|| {
    for path in ["/Applications/Ghostty.app/Contents/MacOS/ghostty", "ghostty"] {
        if let Ok(output) = std::process::Command::new(path).arg("+list-fonts").output() {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                let mut fonts: Vec<String> = text
                    .lines()
                    .filter(|l| !l.starts_with(' ') && !l.starts_with('\t') && !l.trim().is_empty())
                    .map(|l| l.trim().to_string())
                    .collect();
                fonts.sort();
                fonts.dedup();
                if !fonts.is_empty() {
                    return fonts;
                }
            }
        }
    }
    vec![
        "JetBrains Mono".into(),
        "SF Mono".into(),
        "Menlo".into(),
        "Monaco".into(),
        "Courier New".into(),
        "Fira Code".into(),
        "Cascadia Code".into(),
        "Hack".into(),
        "MesloLGS NF".into(),
        "Source Code Pro".into(),
    ]
});

pub fn get_system_fonts() -> &'static [String] {
    &SYSTEM_FONTS
}

static GHOSTTY_THEMES: std::sync::LazyLock<Vec<String>> = std::sync::LazyLock::new(|| {
    for path in ["/Applications/Ghostty.app/Contents/MacOS/ghostty", "ghostty"] {
        if let Ok(output) = std::process::Command::new(path).arg("+list-themes").output() {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                let mut themes: Vec<String> = text
                    .lines()
                    .filter_map(|l| {
                        let trimmed = l.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.split(" (").next().unwrap_or(trimmed).to_string())
                        }
                    })
                    .collect();
                themes.sort();
                themes.dedup();
                if !themes.is_empty() {
                    return themes;
                }
            }
        }
    }
    vec![
        "catppuccin-mocha".into(),
        "catppuccin-macchiato".into(),
        "catppuccin-latte".into(),
        "tokyo-night".into(),
        "nord".into(),
        "dracula".into(),
        "rose-pine".into(),
        "gruvbox-dark".into(),
        "solarized-dark".into(),
        "solarized-light".into(),
        "one-dark".into(),
        "monokai".into(),
    ]
});

pub fn get_ghostty_themes() -> &'static [String] {
    &GHOSTTY_THEMES
}
const POPULAR_FONTS: &[&str] = &[
    "JetBrains Mono",
    "SF Mono",
    "Menlo",
    "Monaco",
    "Fira Code",
    "Cascadia Code",
];

pub enum ActiveModal {
    ListEditor {
        key: &'static str,
        items: Vec<String>,
        recorded_trigger: String,
        selected_action: String,
        action_select: Option<Entity<SelectState<SearchableVec<SharedString>>>>,
        font_select: Option<Entity<SelectState<SearchableVec<SharedString>>>>,
        selected_font: String,
        is_recording: bool,
        recorder_focus: FocusHandle,
        new_item_input: Entity<InputState>,
    },
    DiffViewer,
}

pub struct SettingsView {
    path: PathBuf,
    file: LineFile,
    original: String,
    category: usize,
    search: String,
    search_input: Entity<InputState>,
    notice: Option<String>,
    /// Per-row editors, created lazily on first render. What they hold is the
    /// editing surface; the file stays the source of truth.
    editors: EditorCache,
    /// Interface-language picker. Kept out of the cache because it must
    /// survive the editor reset that a language switch performs.
    lang_select: Entity<SelectState<SearchableVec<SharedString>>>,
    _subscriptions: Vec<Subscription>,
    active_modal: Option<ActiveModal>,
    show_preview: bool,
    scroll_handle: ScrollHandle,
}

impl SettingsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, path: Option<PathBuf>) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder(Text::new("옵션 검색  ( / )", "Search options ( / )").s()));
        let subscription = cx.subscribe_in(&search_input, window, |this, state, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.search = state.read(cx).value().to_string();
                cx.notify();
            }
        });
        let lang_items: Vec<SharedString> = Lang::ALL
            .iter()
            .map(|lang| SharedString::from(lang.native_label()))
            .collect();
        let lang_selected = Lang::ALL
            .iter()
            .position(|lang| *lang == i18n::current())
            .map(IndexPath::new);
        let lang_select = cx
            .new(|cx| SelectState::new(SearchableVec::new(lang_items), lang_selected, window, cx));
        let lang_subscription = cx.subscribe_in(
            &lang_select,
            window,
            |this, _, event, window, cx| {
                let SelectEvent::Confirm(Some(value)) = event else {
                    return;
                };
                if let Some(lang) = Lang::from_native_label(value.as_ref()) {
                    this.apply_language(lang, window, cx);
                }
            },
        );
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
            editors: EditorCache::default(),
            lang_select,
            _subscriptions: vec![subscription, lang_subscription],
            active_modal: None,
            show_preview: true,
            scroll_handle: ScrollHandle::default(),
        }
    }

    /// Switch the interface language.
    ///
    /// Copy resolved while rendering follows on its own; widgets that captured
    /// a localized placeholder when they were created do not. Those are
    /// dropped here and rebuilt from the file on the next frame, which loses
    /// no edits: every keystroke is already written through to the file.
    fn apply_language(&mut self, lang: Lang, window: &mut Window, cx: &mut Context<Self>) {
        if i18n::current() == lang {
            return;
        }
        i18n::set(lang);
        self.editors.forget_inputs();
        self.search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(Text::new("옵션 검색  ( / )", "Search options ( / )").s())
                .default_value(self.search.clone())
        });
        let subscription =
            cx.subscribe_in(&self.search_input, window, |this, state, event, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.search = state.read(cx).value().to_string();
                    cx.notify();
                }
            });
        self._subscriptions.push(subscription);
        self.notice = None;
        cx.notify();
    }

    fn dirty(&self) -> bool {
        self.file.render() != self.original
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        if let Some(parent) = self.path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                self.notice = Some(
                    Text::new("디렉터리 생성 실패: {}", "Couldn't create directory: {}")
                        .fill(&[&e.to_string()]),
                );
                cx.notify();
                return;
            }
        }
        let rendered = self.file.render();
        match std::fs::write(&self.path, &rendered) {
            Ok(()) => {
                self.original = rendered;
                self.notice = Some(
                    Text::new("저장됨 · {}", "Saved · {}").fill(&[&self.path.display().to_string()]),
                );
            }
            Err(e) => {
                self.notice =
                    Some(Text::new("저장 실패: {}", "Couldn't save: {}").fill(&[&e.to_string()]))
            }
        }
        cx.notify();
    }

    fn revert(&mut self, cx: &mut Context<Self>) {
        let text = std::fs::read_to_string(&self.path).unwrap_or_default();
        self.file = LineFile::parse(&text);
        self.original = text;
        self.notice = Some(Text::new("파일 내용을 다시 불러왔습니다.", "File reloaded.").s().to_string());
        // Row editors mirror the file, so drop them and let render rebuild.
        self.editors.forget_all();
        self.active_modal = None;
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

    /// The one path that changes an option's value.
    ///
    /// `value` is the new value, or `None` to clear the key. `kept` names the
    /// editor that produced the change, so the cache can leave it standing
    /// while the editors that would show a stale value are dropped. No call
    /// site has to know which of the four caches holds its key.
    fn commit(
        &mut self,
        key: &'static str,
        value: Option<&str>,
        kept: Kept,
        cx: &mut Context<Self>,
    ) {
        match value {
            Some(value) => self.file.set(key, value),
            None => self.file.remove(key),
        }
        self.settle(key, kept, cx);
    }

    /// Replace every value of a repeatable option.
    fn commit_all(&mut self, key: &'static str, values: &[String], cx: &mut Context<Self>) {
        self.file.set_all(key, values);
        self.settle(key, Kept::Nothing, cx);
    }

    /// Drop the editors that would show a stale value, then re-render.
    fn settle(&mut self, key: &'static str, kept: Kept, cx: &mut Context<Self>) {
        self.editors.invalidate(key, kept);
        self.notice = None;
        cx.notify();
    }

    fn reset_key(&mut self, key: &'static str, cx: &mut Context<Self>) {
        self.commit(key, None, Kept::Nothing, cx);
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
                    // Search every language, so a Korean user who knows the
                    // English term (or the reverse) still finds the option.
                    o.key.contains(&q)
                        || Lang::ALL.iter().any(|lang| {
                            o.label.get(*lang).to_lowercase().contains(&q)
                                || o.doc.get(*lang).to_lowercase().contains(&q)
                        })
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

    fn get_or_create_input(
        &mut self,
        key: &'static str,
        hint: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        let seed = self.file.get(key).unwrap_or_default();
        let placeholder = if !hint.is_empty() { hint } else { Text::new("값 입력", "Enter value").s() };
        let (state, created) = self.editors.input(key, &seed, placeholder, window, cx);
        if created {
            let sub = cx.subscribe_in(&state, window, move |this, state, event, _, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = state.read(cx).value().trim().to_string();
                    let value = if value.is_empty() { None } else { Some(value.as_str()) };
                    this.commit(key, value, Kept::Input, cx);
                }
            });
            self._subscriptions.push(sub);
        }
        state
    }

    fn get_or_create_number_input(
        &mut self,
        key: &'static str,
        default_val: &str,
        min: f64,
        max: f64,
        step: f64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        let seed = self.file.get(key).unwrap_or_else(|| default_val.to_string());
        let (state, created) = self
            .editors
            .number_input(key, &seed, min, max, step, window, cx);
        if created {
            let sub = cx.subscribe_in(&state, window, move |this, state, event, _, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = state.read(cx).value().trim().to_string();
                    let value = if value.is_empty() { None } else { Some(value.as_str()) };
                    this.commit(key, value, Kept::Input, cx);
                }
            });
            self._subscriptions.push(sub);
        }
        state
    }


    fn render_diff_modal(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let diff = compute_line_diff(&self.original, &self.file.render());

        let diff_body = div()
            .max_h(px(380.))
            .overflow_y_scrollbar()
            .p_2()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().muted.opacity(0.15))
            .child(
                v_flex()
                    .gap_0p5()
                    .children(diff.into_iter().enumerate().map(|(ix, line)| {
                        match line {
                            DiffLine::Same(text) => div()
                                .id(format!("diff-same-{ix}"))
                                .px_2()
                                .py(px(1.))
                                .font_family("Menlo")
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("  {text}")),
                            DiffLine::Added(text) => div()
                                .id(format!("diff-add-{ix}"))
                                .px_2()
                                .py(px(1.))
                                .rounded_sm()
                                .bg(cx.theme().success.opacity(0.15))
                                .text_color(cx.theme().success)
                                .font_family("Menlo")
                                .font_semibold()
                                .text_xs()
                                .child(format!("+ {text}")),
                            DiffLine::Removed(text) => div()
                                .id(format!("diff-rem-{ix}"))
                                .px_2()
                                .py(px(1.))
                                .rounded_sm()
                                .bg(cx.theme().warning.opacity(0.15))
                                .text_color(cx.theme().warning)
                                .font_family("Menlo")
                                .font_semibold()
                                .text_xs()
                                .child(format!("- {text}")),
                        }
                    }))
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
                    .w(px(680.))
                    .max_h(px(580.))
                    .rounded_xl()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .p_4()
                    .gap_3()
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .pb_2()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(Icon::new(IconName::FileText).small().text_color(cx.theme().primary))
                                    .child(div().text_base().font_semibold().child(Text::new("변경 사항 미리보기 (Diff)", "Preview changes (Diff)").s())),
                            )
                            .child(
                                Button::new("close-diff")
                                    .ghost()
                                    .xsmall()
                                    .icon(IconName::X)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.active_modal = None;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(diff_body)
                    .child(
                        h_flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .pt_2()
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .child(
                                Button::new("cancel-diff-btn")
                                    .ghost()
                                    .small()
                                    .label(Text::new("닫기", "Close").s())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.active_modal = None;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("save-diff-btn")
                                    .primary()
                                    .small()
                                    .icon(IconName::Check)
                                    .label(Text::new("이대로 저장", "Save as is").s())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.save(cx);
                                        this.active_modal = None;
                                        cx.notify();
                                    })),
                            ),
                    )
            )
            .into_any_element()
    }

}

/// Parse `#rgb` / `#rrggbb` / X11 hex strings accepted by Ghostty into Hsla,
/// so the color picker can be seeded with the file's current value.

/// Where Ghostty documents one option. The reference page anchors every option
/// by its own configuration key.
fn docs_url(key: &str) -> String {
    format!("https://ghostty.org/docs/config/reference#{key}")
}

/// Hover card for an option row: what the option does, then a link to the same
/// option on Ghostty's reference page.
///
/// This is a `HoverCard` and not a `Tooltip` because the link has to be
/// reachable: a tooltip is dismissed as soon as the pointer leaves its trigger,
/// so nothing inside one can be clicked. The card stays open while the pointer
/// is inside it. The text keeps a reading measure instead of stretching to a
/// window-wide line, and `min_w_0` lets the flex item actually shrink to it.
fn doc_card(doc: &'static str, key: &'static str) -> impl IntoElement {
    v_flex()
        .max_w(px(380.))
        .gap_2()
        .child(div().min_w_0().whitespace_normal().child(doc))
        .child(
            Link::new(format!("opt-doc-link-{key}"))
                .href(docs_url(key))
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(Icon::new(IconName::ExternalLink).xsmall())
                        .child(Text::new("Ghostty 문서", "Ghostty docs").s()),
                ),
        )
}

fn row(
    this: &mut SettingsView,
    opt: &'static Opt,
    is_last: bool,
    window: &mut Window,
    cx: &mut Context<SettingsView>,
) -> gpui_kit::AnyElement {
    let set = this.is_set(opt);
    let key = opt.key;
    let doc = opt.doc.s();
    let opt_id = format!("opt-{key}");

    let mut row = h_flex()
        .id(key)
        .items_center()
        .gap_4()
        .px_4()
        .py(px(10.))
        .hover(|s| s.bg(cx.theme().muted.opacity(0.35)));

    if !is_last {
        row = row.border_b_1().border_color(cx.theme().border.opacity(0.6));
    }

    let action_lane = if set {
        let key2 = opt.key;
        let view = cx.entity();
        div()
            .w(px(36.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(
                Button::new(format!("reset-{key2}"))
                    .ghost()
                    .xsmall()
                    .icon(IconName::RotateCcw)
                    .tooltip(Text::new("기본값으로 되돌리기", "Reset to default").s())
                    .on_click(move |_, _, cx| {
                        view.update(cx, |this, cx| this.reset_key(key2, cx));
                    }),
            )
    } else {
        div().w(px(36.)).flex_none()
    };

    row.child(
        HoverCard::new(format!("opt-doc-{key}"))
            .anchor(Anchor::TopLeft)
            .open_delay(Duration::from_millis(400))
            .trigger(
                v_flex()
                    .id(opt_id)
                    .w(px(240.))
                    .flex_none()
                    .gap_0()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child(opt.label.s()),
                            )
                            .children(if !opt.platform.is_empty() {
                                Some(
                                    div()
                                        .text_xs()
                                        .px_1p5()
                                        .py(px(1.))
                                        .rounded_md()
                                        .bg(cx.theme().muted)
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .text_color(cx.theme().muted_foreground)
                                        .font_family("Menlo")
                                        .child(opt.platform),
                                )
                            } else {
                                None
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family("Menlo")
                            .text_color(cx.theme().muted_foreground)
                            .child(key),
                    ),
            )
            .content(move |_, _, _| doc_card(doc, key)),
    )
    .child(
        div()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .child(editor::value_editor(this, opt, window, cx)),
    )
    .child(
        div()
            .w(px(80.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(if set {
                div()
                    .px_2()
                    .py(px(2.))
                    .rounded_full()
                    .bg(cx.theme().primary.opacity(0.12))
                    .text_color(cx.theme().primary)
                    .text_xs()
                    .font_medium()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(div().size(px(5.)).rounded_full().bg(cx.theme().primary))
                    .child(Text::new("설정됨", "Set").s())
                    .into_any_element()
            } else {
                div()
                    .px_2()
                    .py(px(2.))
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(Text::new("기본값", "Default").s())
                    .into_any_element()
            }),
    )
    .child(action_lane)
    .into_any_element()
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dirty = self.dirty();

        let short_path = self
            .path
            .strip_prefix(std::env::var("HOME").unwrap_or_default())
            .map(|p| format!("~/{}", p.display()))
            .unwrap_or_else(|_| self.path.display().to_string());
        let full_path = self.path.display().to_string();

        let titlebar = TitleBar::new()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .size(px(22.))
                                    .rounded_md()
                                    .bg(cx.theme().primary)
                                    .text_color(cx.theme().primary_foreground)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(Icon::new(IconName::Terminal).xsmall()),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .child("Ghostty"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("/"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_medium()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(Text::new("설정", "Settings").s()),
                            ),
                    )
                    .child(
                        div()
                            .id("titlebar-path")
                            .tooltip(move |window, cx| Tooltip::new(full_path.clone()).build(window, cx))
                            .child(
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .px_2()
                                    .py(px(2.))
                                    .rounded_md()
                                    .bg(cx.theme().muted)
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .child(Icon::new(IconName::FileText).xsmall().text_color(cx.theme().muted_foreground))
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_family("Menlo")
                                            .text_color(cx.theme().muted_foreground)
                                            .child(short_path),
                                    ),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(
                                Icon::new(IconName::Languages)
                                    .xsmall()
                                    .text_color(cx.theme().muted_foreground),
                            )
                            // Sized for the longest language name in
                            // `Lang::ALL` plus the caret; revisit when a
                            // longer native name joins the picker.
                            .child(div().w(px(108.)).child(Select::new(&self.lang_select).small())),
                    )
                    .child(div().w(px(1.)).h(px(18.)).bg(cx.theme().border))
                    .child(
                        Button::new("reload")
                            .ghost()
                            .small()
                            .icon(IconName::RefreshCw)
                            .tooltip(Text::new("파일 다시 불러오기", "Reload file").s())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.revert(cx);
                            })),
                    )
                    .child(
                        if dirty {
                            Button::new("revert")
                                .outline()
                                .small()
                                .label(Text::new("변경 취소", "Discard changes").s())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.revert(cx);
                                }))
                        } else {
                            Button::new("revert")
                                .ghost()
                                .small()
                                .label(Text::new("변경 취소", "Discard changes").s())
                                .disabled(true)
                        },
                    )
                    .child(
                        if dirty {
                            Button::new("diff")
                                .outline()
                                .small()
                                .icon(IconName::FileText)
                                .label(Text::new("변경 미리보기", "Review changes").s())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.active_modal = Some(ActiveModal::DiffViewer);
                                    cx.notify();
                                }))
                        } else {
                            Button::new("diff")
                                .ghost()
                                .small()
                                .icon(IconName::FileText)
                                .label(Text::new("변경 미리보기", "Review changes").s())
                                .disabled(true)
                        },
                    )
                    .child(
                        if dirty {
                            Button::new("save")
                                .primary()
                                .small()
                                .icon(IconName::Check)
                                .label(Text::new("저장", "Save").s())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.save(cx);
                                }))
                        } else {
                            Button::new("save")
                                .outline()
                                .small()
                                .icon(IconName::Check)
                                .label(Text::new("저장됨", "Saved").s())
                                .disabled(true)
                        },
                    )
                    .child(
                        Button::new("toggle-preview")
                            .outline()
                            .small()
                            .icon(IconName::PanelRight)
                            .label(if self.show_preview { Text::new("미리보기 닫기", "Close preview").s() } else { Text::new("미리보기", "Preview").s() })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_preview = !this.show_preview;
                                cx.notify();
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
            SidebarMenuItem::new(cat.label.s())
                .icon(category_icon(cat.id))
                .active(selected)
                .suffix(move |_, cx| {
                    if set > 0 {
                        div()
                            .px_1p5()
                            .py(px(1.))
                            .rounded_full()
                            .text_xs()
                            .font_medium()
                            .bg(cx.theme().primary.opacity(0.12))
                            .text_color(cx.theme().primary)
                            .child(set.to_string())
                            .into_any_element()
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
            .header(
                Input::new(&self.search_input)
                    .small()
                    .cleanable(true)
                    .prefix(Icon::new(IconName::Search).small().text_color(cx.theme().muted_foreground)),
            )
            .child(menu);

        let heading = if self.search.is_empty() {
            let cat = &CATEGORIES[self.category];
            (
                cat.label.s().to_string(),
                cat.desc.s().to_string(),
                Some(category_icon(cat.id)),
            )
        } else {
            (
                Text::new("검색 결과 ({}개)", "Search results ({})")
                    .fill(&[&self.visible_opts().len().to_string()]),
                Text::new("키, 이름, 설명에서 일치하는 옵션입니다.", "Options that match by key, name, or description.").s().to_string(),
                Some(IconName::Search),
            )
        };

        let header = h_flex()
            .items_center()
            .justify_between()
            .pb_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .size(px(36.))
                            .rounded_lg()
                            .bg(cx.theme().muted)
                            .border_1()
                            .border_color(cx.theme().border)
                            .text_color(cx.theme().foreground)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(heading.2.unwrap_or(IconName::SlidersHorizontal)).small()),
                    )
                    .child(
                        v_flex()
                            .gap_0()
                            .child(
                                div()
                                    .text_lg()
                                    .font_semibold()
                                    .child(heading.0),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(heading.1),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_1p5()
                    .items_center()
                    .px_2p5()
                    .py_1()
                    .rounded_full()
                    .bg(cx.theme().muted)
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .size(px(6.))
                            .rounded_full()
                            .bg(if self.set_count() > 0 {
                                cx.theme().primary
                            } else {
                                cx.theme().muted_foreground
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_medium()
                            .text_color(cx.theme().foreground)
                            .child(
                                Text::new("{}/{} 설정됨", "{}/{} set").fill(&[
                                    &self.set_count().to_string(),
                                    &self.visible_opts().len().to_string(),
                                ]),
                            ),
                    ),
            );

        let total_visible = self.visible_opts().len();
        let list = if total_visible == 0 {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_3()
                .py_16()
                .child(
                    div()
                        .size(px(48.))
                        .rounded_full()
                        .bg(cx.theme().muted)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Icon::new(IconName::Search).large().text_color(cx.theme().muted_foreground)),
                )
                .child(
                    div()
                        .text_sm()
                        .font_medium()
                        .text_color(cx.theme().foreground)
                        .child(Text::new("일치하는 옵션이 없습니다", "No matching options").s()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(Text::new("다른 키워드로 검색하거나 검색어를 지워보세요.", "Try a different keyword or clear the search.").s()),
                )
                .child(
                    Button::new("clear-search")
                        .outline()
                        .small()
                        .icon(IconName::X)
                        .label(Text::new("검색어 지우기", "Clear search").s())
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.clear_search(window, cx);
                        })),
                )
        } else {
            let opts = self.visible_opts();
            let total = opts.len();
            v_flex()
                .w_full()
                .flex_none()
                .h_auto()
                .rounded_xl()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().background)
                .overflow_hidden()
                .child(
                    h_flex()
                        .items_center()
                        .gap_4()
                        .px_4()
                        .py(px(8.))
                        .bg(cx.theme().muted.opacity(0.35))
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(
                            div()
                                .w(px(240.))
                                .flex_none()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().muted_foreground)
                                .child(Text::new("옵션", "Option").s()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().muted_foreground)
                                .child(Text::new("값", "Value").s()),
                        )
                        .child(
                            div()
                                .w(px(80.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().muted_foreground)
                                .child(Text::new("상태", "State").s()),
                        )
                        .child(div().w(px(36.)).flex_none()),
                )
                .children(opts.into_iter().enumerate().map(|(i, opt)| {
                    row(self, opt, i == total - 1, window, cx)
                }))
        };
        let scroll_area = div()
            .id("option-scroll-area")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .px_6()
            .pb_6()
            .child(list);

        let thumb_bg = cx.theme().muted_foreground.opacity(0.75);
        let thumb_hover = cx.theme().primary;
        let track_bg = cx.theme().muted.opacity(0.4);
        let track_border = cx.theme().border;

        let content = v_flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .h_full()
            .child(div().px_6().pt_5().pb_3().child(header))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .child(scroll_area)
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .child(
                                Scrollbar::vertical(&self.scroll_handle)
                                    .mode(ScrollbarMode::Always)
                                    .viewport_from_layout()
                                    .styles(move |styles| {
                                        styles
                                            .track(move |t| {
                                                t.bg(track_bg)
                                                    .border_color(track_border)
                                                    .width(px(12.))
                                            })
                                            .thumb(move |t| {
                                                t.bg(thumb_bg)
                                                    .width(px(10.))
                                                    .inset(px(1.))
                                                    .radius(px(5.))
                                            })
                                            .thumb_hover(move |t| {
                                                t.bg(thumb_hover)
                                                    .width(px(10.))
                                                    .inset(px(1.))
                                                    .radius(px(5.))
                                            })
                                            .thumb_active(move |t| {
                                                t.bg(thumb_hover)
                                                    .width(px(10.))
                                                    .inset(px(1.))
                                                    .radius(px(5.))
                                            })
                                    }),
                            ),
                    ),
            );

        let status = StatusBar::new()
            .left(
                if let Some(notice) = &self.notice {
                    h_flex()
                        .gap_1p5()
                        .items_center()
                        .text_xs()
                        .text_color(cx.theme().foreground)
                        .child(Icon::new(IconName::RefreshCw).xsmall().text_color(cx.theme().primary))
                        .child(notice.clone())
                } else {
                    h_flex()
                        .gap_3()
                        .items_center()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(
                            h_flex()
                                .gap_1()
                                .items_center()
                                .child(
                                    div()
                                        .px_1p5()
                                        .py(px(1.))
                                        .rounded_sm()
                                        .bg(cx.theme().muted)
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .text_xs()
                                        .font_family("Menlo")
                                        .child("/"),
                                )
                                .child(Text::new("검색 포커스", "Focus search").s()),
                        )
                        .child(
                            h_flex()
                                .gap_1()
                                .items_center()
                                .child(
                                    div()
                                        .px_1p5()
                                        .py(px(1.))
                                        .rounded_sm()
                                        .bg(cx.theme().muted)
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .text_xs()
                                        .font_family("Menlo")
                                        .child("⌘S"),
                                )
                                .child(Text::new("저장", "Save").s()),
                        )
                },
            )
            .right(
                h_flex()
                    .gap_2()
                    .items_center()
                    .text_xs()
                    .font_medium()
                    .child(
                        div()
                            .size(px(6.))
                            .rounded_full()
                            .bg(if dirty {
                                cx.theme().warning
                            } else {
                                cx.theme().success
                            }),
                    )
                    .child(if dirty {
                        Text::new("저장되지 않은 변경사항", "Unsaved changes").s()
                    } else {
                        Text::new("동기화됨", "Synced").s()
                    }),
            );

        let modal = self.active_modal.take();
        let modal_overlay = match &modal {
            Some(ActiveModal::ListEditor {
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
            }) => Some(self.render_list_editor_modal(
                key,
                items,
                recorded_trigger,
                selected_action,
                action_select.as_ref(),
                font_select.as_ref(),
                selected_font.as_str(),
                *is_recording,
                recorder_focus,
                new_item_input,
                cx,
            )),
            Some(ActiveModal::DiffViewer) => Some(self.render_diff_modal(cx)),
            None => None,
        };
        self.active_modal = modal;

        let mut main_area = h_flex()
            .items_stretch()
            .flex_1()
            .min_h_0()
            .child(sidebar)
            .child(content);
        if self.show_preview {
            main_area = main_area.child(self.render_preview_panel(cx));
        }

        let mut root = v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .relative()
            .key_context("Settings")
            .on_action(cx.listener(Self::commit_save))
            .on_action(cx.listener(Self::focus_search))
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(titlebar)
            .child(main_area)
            .child(status);
        if let Some(overlay) = modal_overlay {
            root = root.child(overlay);
        }

        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ghostty_trigger_formatting() {
        assert_eq!(ghostty_trigger_to_pretty("super+c"), "⌘C");
        assert_eq!(ghostty_trigger_to_pretty("super+shift+k"), "⌘⇧K");
        assert_eq!(ghostty_trigger_to_pretty("ctrl+tab"), "⌃⇥");
    }

    #[test]
    fn test_compute_line_diff() {
        let old = "a\nb\nc\n";
        let new = "a\nb2\nc\nd\n";
        let diff = compute_line_diff(old, new);
        assert!(diff.iter().any(|d| matches!(d, DiffLine::Added(s) if s == "b2")));
        assert!(diff.iter().any(|d| matches!(d, DiffLine::Removed(s) if s == "b")));
        assert!(diff.iter().any(|d| matches!(d, DiffLine::Added(s) if s == "d")));
    }

    #[test]
    fn test_action_description() {
        let restore = i18n::current();

        i18n::set(Lang::Ko);
        assert_eq!(action_description("copy_to_clipboard"), Some("클립보드에 복사"));
        assert_eq!(action_description("new_tab"), Some("새 탭 열기"));
        assert_eq!(action_description("increase_font_size:1"), Some("글꼴 크기 확대 (+1)"));
        assert_eq!(action_description("unknown_action_xyz"), None);

        // The same table answers in English without being rebuilt.
        i18n::set(Lang::En);
        assert_eq!(action_description("copy_to_clipboard"), Some("Copy to clipboard"));
        assert_eq!(action_description("new_tab"), Some("Open new tab"));
        assert_eq!(action_description("increase_font_size:1"), Some("Increase font size (+1)"));
        assert_eq!(action_description("unknown_action_xyz"), None);

        i18n::set(restore);
    }

    /// The UI may offer less than Ghostty accepts, but never more: a narrowed
    /// range that escapes the valid range would let a control write a value
    /// the config rejects.
    #[test]
    fn ui_ranges_stay_inside_schema_bounds() {
        for (key, min, max) in UI_RANGES {
            let opt = lookup(key).unwrap_or_else(|| panic!("{key} is not an option"));
            let (valid_min, valid_max) = opt
                .kind
                .bounds()
                .unwrap_or_else(|| panic!("{key} is not numeric"));
            assert!(
                valid_min <= *min && *max <= valid_max,
                "{key}: UI range {min}..{max} escapes the valid range {valid_min}..{valid_max}"
            );
        }
    }

    /// A category that falls through to the fallback icon is a category whose
    /// id no longer matches the icon table.
    #[test]
    fn every_category_has_its_own_icon() {
        for cat in CATEGORIES {
            assert_ne!(
                category_icon(cat.id),
                IconName::Zap,
                "category '{}' (id '{}') fell through to the fallback icon",
                cat.label.s(),
                cat.id
            );
        }
    }

    /// Every row links to the option's own anchor on Ghostty's reference page,
    /// so a key that is not a clean URL fragment would produce a dead link.
    #[test]
    fn docs_url_anchors_every_option_key() {
        assert_eq!(
            docs_url("adjust-strikethrough-position"),
            "https://ghostty.org/docs/config/reference#adjust-strikethrough-position"
        );
        for opt in crate::config::schema::OPTS {
            assert!(
                opt.key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{} is not a usable anchor fragment",
                opt.key
            );
        }
    }

    /// The whole point of the switch: one view renders two sets of copy.
    #[test]
    fn test_option_labels_follow_the_language() {
        let restore = i18n::current();
        let font = lookup("font-family").expect("font-family is in the schema");

        i18n::set(Lang::Ko);
        assert_eq!(font.label.s(), "글꼴");
        i18n::set(Lang::En);
        assert_eq!(font.label.s(), "Font");

        i18n::set(restore);
    }

    #[test]
    fn test_system_fonts_discovery() {
        let fonts = get_system_fonts();
        assert!(!fonts.is_empty());
        assert!(fonts.iter().any(|f| f.contains("Mono") || f.contains("Courier") || f.contains("Menlo")));
    }

    #[test]
    fn test_ghostty_themes_discovery() {
        let themes = get_ghostty_themes();
        assert!(!themes.is_empty());
        assert!(themes.iter().any(|t| t.contains("catppuccin") || t.contains("nord") || t.contains("dracula") || t.contains("tokyo")));
    }

}
