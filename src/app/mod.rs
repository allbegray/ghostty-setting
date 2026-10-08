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
use std::rc::Rc;
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
    Anchor, App, AppContext as _, Context, Entity, FocusHandle, Focusable as _, IntoElement,
    InteractiveElement as _, KeyDownEvent, ParentElement as _, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window, actions,
    div, px,
};
use gpui_kit::base::Disableable as _;

mod chrome;
mod controls;
mod diff_modal;
mod editor;
mod editors;
mod list_editor;
mod list_items;
mod preview;
mod query;
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
                let fonts = parse_fonts(&String::from_utf8_lossy(&output.stdout));
                if !fonts.is_empty() {
                    return fonts;
                }
            }
        }
    }
    vec![
        controls::DEFAULT_FONT.to_string(),
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

/// Font families from `ghostty +list-fonts`.
///
/// The command prints a name per line and indents stylistic variants beneath
/// the family they belong to, so an indented line is not a family.
fn parse_fonts(output: &str) -> Vec<String> {
    let mut fonts: Vec<String> = output
        .lines()
        .filter(|line| !line.starts_with(' ') && !line.starts_with('\t') && !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .collect();
    fonts.sort();
    fonts.dedup();
    fonts
}

/// Theme names from `ghostty +list-themes`.
///
/// Each line is `name (source)`, and the panel only wants the name.
fn parse_themes(output: &str) -> Vec<String> {
    let mut themes: Vec<String> = output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.split(" (").next().unwrap_or(trimmed).to_string())
            }
        })
        .collect();
    themes.sort();
    themes.dedup();
    themes
}

pub fn get_system_fonts() -> &'static [String] {
    &SYSTEM_FONTS
}

static GHOSTTY_THEMES: std::sync::LazyLock<Vec<String>> = std::sync::LazyLock::new(|| {
    for path in ["/Applications/Ghostty.app/Contents/MacOS/ghostty", "ghostty"] {
        if let Ok(output) = std::process::Command::new(path).arg("+list-themes").output() {
            if output.status.success() {
                let themes = parse_themes(&String::from_utf8_lossy(&output.stdout));
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
    /// The list editor owns its own state (see `list_editor::ListEditorModal`).
    ListEditor(list_editor::ListEditorModal),
    DiffViewer,
}

impl SettingsView {
    /// The list editor, when it is the open modal.
    ///
    /// Every handler that changes the modal's state goes through here, so the
    /// reach-in pattern has one shape and one place to change.
    fn list_editor(&mut self) -> Option<&mut list_editor::ListEditorModal> {
        match &mut self.active_modal {
            Some(ActiveModal::ListEditor(modal)) => Some(modal),
            _ => None,
        }
    }
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
        query::dirty(&self.file, &self.original)
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

    /// The one path that opens the diff viewer.
    ///
    /// Reviewing changes is a view-level action: which modal is open is view
    /// state, and this is the door a section has to it.
    fn review_changes(&mut self, cx: &mut Context<Self>) {
        self.active_modal = Some(ActiveModal::DiffViewer);
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
        query::is_set(&self.file, opt)
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
        query::visible(&self.search, self.category)
    }

    fn set_count(&self) -> usize {
        query::set_count(&self.file, &self.search, self.category)
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


/// A title-bar action: the view, doing one thing.
///
/// The render body is the only place holding the view's own handle, so it is
/// the only place that can build these. A section receives the closure and
/// nothing else.
fn view_action(
    view: &Entity<SettingsView>,
    action: impl Fn(&mut SettingsView, &mut Context<SettingsView>) + 'static,
) -> Rc<dyn Fn(&mut Window, &mut App)> {
    let view = view.clone();
    Rc::new(move |_: &mut Window, cx: &mut App| view.update(cx, |this, cx| action(this, cx)))
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dirty = self.dirty();
        let view = cx.entity();
        let actions = chrome::TitleBarActions {
            reload: view_action(&view, |this, cx| this.revert(cx)),
            discard: view_action(&view, |this, cx| this.revert(cx)),
            save: view_action(&view, |this, cx| this.save(cx)),
            review_changes: view_action(&view, |this, cx| this.review_changes(cx)),
            toggle_preview: view_action(&view, |this, cx| {
                this.show_preview = !this.show_preview;
                cx.notify();
            }),
        };
        let titlebar = chrome::title_bar(
            &chrome::TitleBarInput {
                path: &self.path,
                dirty,
                preview_open: self.show_preview,
                lang_select: &self.lang_select,
            },
            &actions,
            cx,
        );
        let nav_items: Vec<chrome::NavItem> = CATEGORIES
            .iter()
            .map(|cat| chrome::NavItem {
                label: cat.label.s().into(),
                icon: category_icon(cat.id),
                set_count: cat
                    .keys
                    .iter()
                    .filter(|k| lookup(k).is_some_and(|o| self.is_set(o)))
                    .count(),
            })
            .collect();
        let on_pick_category: Rc<dyn Fn(&usize, &mut Window, &mut App)> = Rc::new(cx.listener(
            |this: &mut SettingsView, ix: &usize, _: &mut Window, cx: &mut Context<SettingsView>| {
                this.category = *ix;
                cx.notify();
            },
        ));
        let sidebar = chrome::nav_sidebar(
            &chrome::NavSidebarInput {
                items: &nav_items,
                active: self.category,
                searching: self.search.is_empty(),
                search_input: &self.search_input,
            },
            on_pick_category,
            cx,
        );
        let content = self.option_table(window, cx);
        let status = chrome::status_bar(self.notice.as_deref(), dirty, cx);

        let modal_overlay = match &self.active_modal {
            Some(ActiveModal::ListEditor(modal)) => Some(self.render_list_editor_modal(modal, cx)),
            Some(ActiveModal::DiffViewer) => Some(self.render_diff_modal(cx)),
            None => None,
        };

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

    /// The catalog parsers are the part that can be wrong about the command's
    /// output, so they are checked against captured output rather than against
    /// whatever the machine happens to answer. The previous versions of these
    /// tests passed whether the subprocess ran, failed, or returned nonsense.
    #[test]
    fn fonts_are_read_from_the_command_output() {
        let output = "JetBrains Mono\n  JetBrains Mono NL\nMenlo\nMenlo\n\nFira Code\n";
        assert_eq!(parse_fonts(output), vec!["Fira Code", "JetBrains Mono", "Menlo"]);
        assert!(parse_fonts("").is_empty());
    }

    #[test]
    fn themes_are_read_without_their_source() {
        let output = "tokyo-night (built-in)\nnord (built-in)\n\ndracula (user)\n";
        assert_eq!(parse_themes(output), vec!["dracula", "nord", "tokyo-night"]);
        assert!(parse_themes("").is_empty());
    }

    /// Whichever path the catalog takes, the app needs something to offer.
    #[test]
    fn the_catalogs_are_never_empty() {
        assert!(!get_system_fonts().is_empty());
        assert!(!get_ghostty_themes().is_empty());
    }

}
