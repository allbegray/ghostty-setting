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
struct ThemeColors {
    name: &'static str,
    bg: &'static str,
    fg: &'static str,
    cursor: &'static str,
    palette: [&'static str; 8],
}

const THEME_PREVIEWS: &[ThemeColors] = &[
    ThemeColors {
        name: "catppuccin-mocha",
        bg: "#1e1e2e",
        fg: "#cdd6f4",
        cursor: "#f5e0dc",
        palette: ["#45475a", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#bac2de"],
    },
    ThemeColors {
        name: "tokyo-night",
        bg: "#1a1b26",
        fg: "#c0caf5",
        cursor: "#c0caf5",
        palette: ["#15161e", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#a9b1d6"],
    },
    ThemeColors {
        name: "nord",
        bg: "#2e3440",
        fg: "#d8dee9",
        cursor: "#d8dee9",
        palette: ["#3b4252", "#bf616a", "#a3be8c", "#ebcb8b", "#81a1c1", "#b48ead", "#88c0d0", "#e5e9f0"],
    },
    ThemeColors {
        name: "dracula",
        bg: "#282a36",
        fg: "#f8f8f2",
        cursor: "#f8f8f2",
        palette: ["#21222c", "#ff5555", "#50fa7b", "#f1fa8c", "#bd93f9", "#ff79c6", "#8be9fd", "#f8f8f2"],
    },
    ThemeColors {
        name: "rose-pine",
        bg: "#191724",
        fg: "#e0def4",
        cursor: "#524f67",
        palette: ["#26233a", "#eb6f92", "#31748f", "#f6c177", "#9ccfd8", "#c4a7e7", "#ebbcba", "#e0def4"],
    },
    ThemeColors {
        name: "gruvbox-dark",
        bg: "#282828",
        fg: "#ebdbb2",
        cursor: "#ebdbb2",
        palette: ["#282828", "#cc241d", "#98971a", "#d79921", "#458588", "#b16286", "#689d6a", "#a89984"],
    },
    ThemeColors {
        name: "solarized-dark",
        bg: "#002b36",
        fg: "#839496",
        cursor: "#93a1a1",
        palette: ["#073642", "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198", "#eee8d5"],
    },
    ThemeColors {
        name: "one-dark",
        bg: "#282c34",
        fg: "#abb2bf",
        cursor: "#528bff",
        palette: ["#1e2127", "#e06c75", "#98c379", "#d19a66", "#61afef", "#c678dd", "#56b6c2", "#abb2bf"],
    },
];

fn lookup_theme_colors(theme_name: &str) -> Option<&'static ThemeColors> {
    THEME_PREVIEWS.iter().find(|t| t.name == theme_name)
}

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

    fn render_list_editor_modal(
        &self,
        key: &'static str,
        items: &[String],
        recorded_trigger: &str,
        selected_action: &str,
        action_select: Option<&Entity<SelectState<SearchableVec<SharedString>>>>,
        font_select: Option<&Entity<SelectState<SearchableVec<SharedString>>>>,
        selected_font: &str,
        is_recording: bool,
        recorder_focus: &FocusHandle,
        new_item_input: &Entity<InputState>,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let is_keybind = key == "keybind";
        let is_font = key == "font-family";
        let is_config = key == "config-file";
        let is_feature = key == "font-feature";
        let opt_label = lookup(key).map(|o| o.label.s()).unwrap_or(key);
        let view = cx.entity();

        let icon = if is_keybind {
            IconName::Keyboard
        } else if is_font || is_feature {
            IconName::Type
        } else if is_config {
            IconName::FileText
        } else {
            IconName::Pencil
        };

        let title = if is_keybind {
            Text::new("키 바인딩 설정 (`keybind`)", "Configure key bindings (`keybind`)").s().to_string()
        } else if is_font {
            Text::new("글꼴 우선순위 설정 (`font-family`)", "Configure font priority (`font-family`)").s().to_string()
        } else if is_feature {
            Text::new("OpenType 기능 설정 (`font-feature`)", "Configure OpenType features (`font-feature`)").s().to_string()
        } else if is_config {
            Text::new("추가 설정 파일 불러오기 (`config-file`)", "Load additional config files (`config-file`)").s().to_string()
        } else {
            Text::new("{} 목록 편집 (`{}`)", "Edit {} list (`{}`)").fill(&[opt_label, key])
        };

        let subtitle = if is_keybind {
            Text::new("단축키 입력을 녹음하고 실행할 Ghostty 동작을 지정합니다.", "Record a shortcut and choose the Ghostty action to run.").s()
        } else if is_font {
            Text::new("시스템에 설치된 폰트를 선택하거나 인기 코딩 폰트를 추가하여 우선순위를 구성합니다.", "Select a font installed on your system or add a popular coding font to set the priority order.").s()
        } else if is_feature {
            Text::new("폰트의 프로그래밍 합자(Ligatures) 및 특수 글리프 기능을 켜고 끕니다.", "Turn programming ligatures and special glyph features on or off.").s()
        } else if is_config {
            Text::new("파일 탐색기로 추가 설정 파일을 찾아보거나 직접 경로를 추가합니다.", "Browse for additional config files or add a path directly.").s()
        } else {
            Text::new("설정 파일에 반복 지정되는 항목 목록을 관리합니다.", "Manage the list of items that can be repeated in the config file.").s()
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
                        .children(items.iter().enumerate().map(|(ix, item)| {
                            let view = view.clone();
                            if is_keybind {
                                let parts: Vec<&str> = item.splitn(2, '=').collect();
                                let trigger = parts[0];
                                let action = parts.get(1).unwrap_or(&"");
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
                                                    if let Some(ActiveModal::ListEditor { items, .. }) = &mut this.active_modal {
                                                        if ix < items.len() {
                                                            items.remove(ix);
                                                            cx.notify();
                                                        }
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
                                                    if let Some(ActiveModal::ListEditor { items, .. }) = &mut this.active_modal {
                                                        if ix < items.len() {
                                                            items.remove(ix);
                                                            cx.notify();
                                                        }
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

        let add_section = if is_keybind {
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
                                    if let Some(ActiveModal::ListEditor { is_recording, recorder_focus, .. }) = &mut this.active_modal {
                                        *is_recording = true;
                                        window.focus(recorder_focus, cx);
                                        cx.notify();
                                    }
                                }))
                                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                    if let Some(ActiveModal::ListEditor { is_recording, recorded_trigger, .. }) = &mut this.active_modal {
                                        if *is_recording {
                                            let key_name = &event.keystroke.key;
                                            if key_name == "escape" {
                                                *is_recording = false;
                                                cx.notify();
                                                return;
                                            }
                                            if !is_modifier_key_name(key_name) {
                                                *recorded_trigger = keystroke_to_ghostty_trigger(&event.keystroke);
                                                *is_recording = false;
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
                                    if let Some(ActiveModal::ListEditor { recorded_trigger, items, selected_action, .. }) = &mut this.active_modal {
                                        if !recorded_trigger.is_empty() && !selected_action.trim().is_empty() {
                                            let entry = format!("{recorded_trigger}={}", selected_action.trim());
                                            items.push(entry);
                                            recorded_trigger.clear();
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
                                                if let Some(ActiveModal::ListEditor { selected_action, .. }) = &mut this.active_modal {
                                                    *selected_action = act_str.clone();
                                                    cx.notify();
                                                }
                                            });
                                        })
                                }))
                        )
                )
                .into_any_element()
        } else if is_font {
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
                                    if let Some(ActiveModal::ListEditor { items, selected_font, .. }) = &mut this.active_modal {
                                        if !selected_font.trim().is_empty() {
                                            items.push(selected_font.trim().to_string());
                                            cx.notify();
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
                                .children(POPULAR_FONTS.iter().map(|&font_name| {
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
                                                if let Some(ActiveModal::ListEditor { items, .. }) = &mut this.active_modal {
                                                    if !items.iter().any(|f| f == font_name) {
                                                        items.push(font_name.to_string());
                                                        cx.notify();
                                                    }
                                                }
                                            });
                                        })
                                }))
                        )
                )
                .into_any_element()
        } else if is_config {
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
                                        if let Some(ActiveModal::ListEditor { items, .. }) = &mut this.active_modal {
                                            items.push(path);
                                            cx.notify();
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
                            if let Some(ActiveModal::ListEditor { items, new_item_input, .. }) = &mut this.active_modal {
                                let val = new_item_input.read(cx).value().to_string();
                                if !val.trim().is_empty() {
                                    items.push(val.trim().to_string());
                                    new_item_input.update(cx, |inp, cx| {
                                        inp.set_value("", window, cx);
                                    });
                                    cx.notify();
                                }
                            }
                        }))
                )
                .into_any_element()
        } else if is_feature {
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
                                    if let Some(ActiveModal::ListEditor { items, new_item_input, .. }) = &mut this.active_modal {
                                        let val = new_item_input.read(cx).value().to_string();
                                        if !val.trim().is_empty() {
                                            items.push(val.trim().to_string());
                                            new_item_input.update(cx, |inp, cx| {
                                                inp.set_value("", window, cx);
                                            });
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
                                                if let Some(ActiveModal::ListEditor { items, .. }) = &mut this.active_modal {
                                                    if !items.iter().any(|f| f == feat) {
                                                        items.push(feat.to_string());
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
                            if let Some(ActiveModal::ListEditor { items, new_item_input, .. }) = &mut this.active_modal {
                                let val = new_item_input.read(cx).value().to_string();
                                if !val.trim().is_empty() {
                                    items.push(val.trim().to_string());
                                    new_item_input.update(cx, |inp, cx| {
                                        inp.set_value("", window, cx);
                                    });
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
                        if let Some(ActiveModal::ListEditor { key, items, .. }) = &this.active_modal {
                            let k = *key;
                            let items_clone = items.clone();
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

    fn render_preview_panel(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let view = cx.entity();

        let font_family = self
            .file
            .get("font-family")
            .unwrap_or_else(|| "JetBrains Mono".to_string());
        let font_size: f32 = self
            .file
            .get("font-size")
            .and_then(|s| s.parse().ok())
            .unwrap_or(13.5);
        let theme_name = self
            .file
            .get("theme")
            .unwrap_or_else(|| "tokyo-night".to_string());
        let opacity: f32 = self
            .file
            .get("background-opacity")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.0);
        let cursor_style = self
            .file
            .get("cursor-style")
            .unwrap_or_else(|| "block".to_string());
        let file_bg = self.file.get("background");
        let file_fg = self.file.get("foreground");
        let file_cursor = self.file.get("cursor-color");

        let theme_colors = lookup_theme_colors(&theme_name);
        let bg_hex = theme_colors
            .map(|t| t.bg)
            .or_else(|| file_bg.as_deref())
            .unwrap_or("#1a1b26");
        let fg_hex = theme_colors
            .map(|t| t.fg)
            .or_else(|| file_fg.as_deref())
            .unwrap_or("#c0caf5");
        let cursor_hex = theme_colors
            .map(|t| t.cursor)
            .or_else(|| file_cursor.as_deref())
            .unwrap_or(fg_hex);
        let bg_color = value::Stored::new(Some(bg_hex)).color()
            .unwrap_or_else(|| gpui_kit::hsla(0.0, 0.0, 0.1, 1.0))
            .alpha(opacity.clamp(0.2, 1.0));
        let fg_color =
            value::Stored::new(Some(fg_hex)).color().unwrap_or_else(|| gpui_kit::hsla(0.0, 0.0, 0.9, 1.0));
        let cursor_color =
            value::Stored::new(Some(cursor_hex)).color().unwrap_or_else(|| gpui_kit::hsla(0.0, 0.0, 0.9, 1.0));

        let palette: &[&str] = if let Some(t) = theme_colors {
            &t.palette
        } else {
            &[
                "#15161e", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff",
                "#a9b1d6",
            ]
        };

        let panel_header = h_flex()
            .items_center()
            .justify_between()
            .pb_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Icon::new(IconName::Terminal)
                            .small()
                            .text_color(cx.theme().primary),
                    )
                    .child(div().font_semibold().text_sm().child(Text::new("라이브 미리보기", "Live preview").s()))
                    .child(
                        div()
                            .text_xs()
                            .px_1p5()
                            .py(px(1.))
                            .rounded_md()
                            .bg(cx.theme().primary.opacity(0.12))
                            .text_color(cx.theme().primary)
                            .font_medium()
                            .child(Text::new("실시간", "Live").s()),
                    ),
            )
            .child(
                Button::new("close-preview-panel-btn")
                    .ghost()
                    .xsmall()
                    .icon(IconName::X)
                    .tooltip(Text::new("패널 닫기", "Close panel").s())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_preview = false;
                        cx.notify();
                    })),
            );

        let cursor_elem = match cursor_style.as_str() {
            "bar" => div().w(px(2.)).h(px(font_size + 2.0)).bg(cursor_color),
            "underline" => div()
                .w(px(font_size * 0.6))
                .h(px(2.))
                .mt(px(font_size))
                .bg(cursor_color),
            _ => div()
                .w(px(font_size * 0.58))
                .h(px(font_size + 2.0))
                .bg(cursor_color),
        };

        let terminal_window = div()
            .rounded_xl()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .shadow_md()
            .overflow_hidden()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py(px(6.))
                    .bg(cx.theme().muted.opacity(0.5))
                    .border_b_1()
                    .border_color(cx.theme().border.opacity(0.5))
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(div().size(px(10.)).rounded_full().bg(gpui_kit::rgb(0xff5f56)))
                            .child(div().size(px(10.)).rounded_full().bg(gpui_kit::rgb(0xffbd2e)))
                            .child(div().size(px(10.)).rounded_full().bg(gpui_kit::rgb(0x27c93f))),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .font_family("Menlo")
                            .child(format!("{font_family} · {font_size:.1}pt")),
                    )
                    .child(div().w(px(32.))),
            )
            .child(
                v_flex()
                    .p_3()
                    .bg(bg_color)
                    .font_family(SharedString::from(font_family.clone()))
                    .text_size(px(font_size.clamp(10.0, 18.0)))
                    .text_color(fg_color)
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(
                                div()
                                    .text_color(value::Stored::new(Some(palette[4])).color().unwrap_or(fg_color))
                                    .child("~"),
                            )
                            .child(
                                div()
                                    .text_color(value::Stored::new(Some(palette[2])).color().unwrap_or(fg_color))
                                    .child("❯"),
                            )
                            .child(div().child("cargo test --quiet")),
                    )
                    .child(
                        div()
                            .text_color(value::Stored::new(Some(palette[2])).color().unwrap_or(fg_color))
                            .child("   Compiling ghostty v1.2.3"),
                    )
                    .child(
                        div()
                            .text_color(value::Stored::new(Some(palette[3])).color().unwrap_or(fg_color))
                            .child("    Finished dev profile"),
                    )
                    .child(div().child("test result: ok. 25 passed; 0 failed"))
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(
                                div()
                                    .text_color(value::Stored::new(Some(palette[4])).color().unwrap_or(fg_color))
                                    .child("~"),
                            )
                            .child(
                                div()
                                    .text_color(value::Stored::new(Some(palette[2])).color().unwrap_or(fg_color))
                                    .child("❯"),
                            )
                            .child(div().child("git status"))
                            .child(cursor_elem),
                    ),
            );

        let color_swatches = v_flex()
            .gap_1p5()
            .child(
                div()
                    .text_xs()
                    .font_medium()
                    .text_color(cx.theme().muted_foreground)
                    .child(Text::new("테마 ANSI 팔레트:", "Theme ANSI palette:").s()),
            )
            .child(
                h_flex()
                    .gap_1()
                    .children(palette.iter().enumerate().map(|(ix, &hex)| {
                        let c = value::Stored::new(Some(hex)).color().unwrap_or(fg_color);
                        div()
                            .id(format!("preview-palette-{ix}"))
                            .flex_1()
                            .h(px(16.))
                            .rounded_sm()
                            .bg(c)
                            .border_1()
                            .border_color(cx.theme().border.opacity(0.3))
                    })),
            );

        let specs_card = v_flex()
            .gap_1p5()
            .p(px(10.))
            .rounded_lg()
            .bg(cx.theme().muted.opacity(0.2))
            .border_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .justify_between()
                    .text_xs()
                    .child(div().text_color(cx.theme().muted_foreground).child(Text::new("글꼴", "Font").s()))
                    .child(
                        div()
                            .font_family("Menlo")
                            .child(format!("{font_family} ({font_size:.1}pt)")),
                    ),
            )
            .child(
                h_flex()
                    .justify_between()
                    .text_xs()
                    .child(div().text_color(cx.theme().muted_foreground).child(Text::new("테마", "Theme").s()))
                    .child(div().font_family("Menlo").child(theme_name.clone())),
            )
            .child(
                h_flex()
                    .justify_between()
                    .text_xs()
                    .child(
                        div()
                            .text_color(cx.theme().muted_foreground)
                            .child(Text::new("배경 / 불투명도", "Background / opacity").s()),
                    )
                    .child(
                        div()
                            .font_family("Menlo")
                            .child(format!("{bg_hex} · {:.0}%", opacity * 100.0)),
                    ),
            )
            .child(
                h_flex()
                    .justify_between()
                    .text_xs()
                    .child(
                        div()
                            .text_color(cx.theme().muted_foreground)
                            .child(Text::new("커서 스타일", "Cursor style").s()),
                    )
                    .child(div().font_family("Menlo").child(cursor_style)),
            );

        let theme_chips = v_flex()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .font_medium()
                    .text_color(cx.theme().muted_foreground)
                    .child(Text::new("테마 빠른 변경:", "Quick theme switch:").s()),
            )
            .child(
                h_flex()
                    .gap_1()
                    .flex_wrap()
                    .children(THEME_PREVIEWS.iter().map(|t| {
                        let view = view.clone();
                        let name = t.name;
                        let is_active = theme_name == name;
                        div()
                            .id(format!("quick-theme-{name}"))
                            .cursor_pointer()
                            .px_2()
                            .py(px(2.))
                            .rounded_md()
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
                            .child(name)
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.commit("theme", Some(name), Kept::Nothing, cx);
                                });
                            })
                    })),
            );

        let font_chips = v_flex()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .font_medium()
                    .text_color(cx.theme().muted_foreground)
                    .child(Text::new("글꼴 빠른 변경:", "Quick font switch:").s()),
            )
            .child(
                h_flex()
                    .gap_1()
                    .flex_wrap()
                    .children(POPULAR_FONTS.iter().map(|&font| {
                        let view = view.clone();
                        let is_active = font_family == font;
                        div()
                            .id(format!("quick-font-preview-{font}"))
                            .cursor_pointer()
                            .px_2()
                            .py(px(2.))
                            .rounded_md()
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
                            .child(font)
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| {
                                    this.commit("font-family", Some(font), Kept::Nothing, cx);
                                });
                            })
                    })),
            );

        v_flex()
            .w(px(380.))
            .border_l_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .p_4()
            .gap_3()
            .overflow_y_scrollbar()
            .child(panel_header)
            .child(terminal_window)
            .child(color_swatches)
            .child(specs_card)
            .child(theme_chips)
            .child(font_chips)
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

    #[test]
    fn test_lookup_theme_colors() {
        let tokyo = lookup_theme_colors("tokyo-night");
        assert!(tokyo.is_some());
        assert_eq!(tokyo.unwrap().bg, "#1a1b26");
        assert!(lookup_theme_colors("non_existent_theme").is_none());
    }
}
