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

use std::collections::HashMap;
use std::path::PathBuf;

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, TitleBar,
    IndexPath,
    button::{Button, ButtonVariants as _},
    color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState},
    h_flex, v_flex,
    input::{Input, InputEvent, InputState, NumberInput},
    scroll::ScrollableElement as _,
    select::{Select, SelectEvent, SelectState},
    searchable_list::SearchableVec,
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
    slider::{Slider, SliderEvent, SliderState},
    status_bar::StatusBar,
    switch::Switch,
    tooltip::Tooltip,
    Icon,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AppContext as _, Context, Entity, FocusHandle, Focusable as _, IntoElement,
    InteractiveElement as _, KeyDownEvent, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, actions, div, px,
};
use gpui_kit::base::Disableable as _;

use crate::config::linefile::LineFile;
use crate::config::schema::{CATEGORIES, Kind, Opt, lookup};
use crate::config::{self};

fn category_icon(idx: usize) -> IconName {
    match idx {
        0 => IconName::Type,
        1 => IconName::Palette,
        2 => IconName::MousePointer,
        3 => IconName::Clipboard,
        4 => IconName::AppWindow,
        5 => IconName::Terminal,
        6 => IconName::SlidersHorizontal,
        _ => IconName::Zap,
    }
}
actions!(settings, [Save, FocusSearch]);
pub const GHOSTTY_ACTIONS: &[(&str, &str)] = &[
    ("copy_to_clipboard", "클립보드에 복사"),
    ("paste_from_clipboard", "클립보드에서 붙여넣기"),
    ("paste_from_selection", "선택 영역 붙여넣기"),
    ("copy_url_to_clipboard", "마지막 URL 복사"),
    ("copy_title_to_clipboard", "창 제목 복사"),
    ("select_all", "전체 선택"),
    ("adjust_selection:left", "선택 영역 왼쪽 확장"),
    ("adjust_selection:right", "선택 영역 오른쪽 확장"),
    ("new_window", "새 창 열기"),
    ("new_tab", "새 탭 열기"),
    ("close_tab", "현재 탭 닫기"),
    ("close_surface", "현재 서피스(분할) 닫기"),
    ("close_window", "현재 창 닫기"),
    ("close_all_windows", "모든 창 닫기"),
    ("previous_tab", "이전 탭으로 이동"),
    ("next_tab", "다음 탭으로 이동"),
    ("last_tab", "마지막 탭으로 이동"),
    ("goto_tab:1", "1번 탭으로 이동"),
    ("goto_tab:2", "2번 탭으로 이동"),
    ("goto_tab:3", "3번 탭으로 이동"),
    ("goto_tab:4", "4번 탭으로 이동"),
    ("goto_tab:5", "5번 탭으로 이동"),
    ("toggle_tab_overview", "탭 개요(오버뷰) 토글"),
    ("new_split:right", "오른쪽에 분할 창 생성"),
    ("new_split:down", "아래쪽에 분할 창 생성"),
    ("goto_split:next", "다음 분할 창으로 이동"),
    ("goto_split:previous", "이전 분할 창으로 이동"),
    ("goto_split:top", "위쪽 분할 창으로 이동"),
    ("goto_split:bottom", "아래쪽 분할 창으로 이동"),
    ("goto_split:left", "왼쪽 분할 창으로 이동"),
    ("goto_split:right", "오른쪽 분할 창으로 이동"),
    ("toggle_split_zoom", "분할 창 확대/축소 토글"),
    ("equalize_splits", "분할 창 크기 균등화"),
    ("clear_screen", "화면 지우기"),
    ("scroll_to_top", "맨 위로 스크롤"),
    ("scroll_to_bottom", "맨 아래로 스크롤"),
    ("scroll_page_up", "한 페이지 위로 스크롤"),
    ("scroll_page_down", "한 페이지 아래로 스크롤"),
    ("jump_to_prompt:1", "다음 쉘 프롬프트로 이동"),
    ("jump_to_prompt:-1", "이전 쉘 프롬프트로 이동"),
    ("increase_font_size:1", "글꼴 크기 확대 (+1)"),
    ("decrease_font_size:1", "글꼴 크기 축소 (-1)"),
    ("reset_font_size", "글꼴 크기 기본값 초기화"),
    ("toggle_fullscreen", "전체화면 토글"),
    ("toggle_maximize", "창 최대화 토글"),
    ("toggle_quick_terminal", "퀵 터미널 토글"),
    ("toggle_command_palette", "커맨드 팔레트 열기"),
    ("toggle_window_decorations", "창 프레임/장식 토글"),
    ("toggle_window_float_on_top", "항상 위에 표시 토글"),
    ("toggle_visibility", "창 보이기/숨기기 토글"),
    ("toggle_background_opacity", "배경 불투명도 토글"),
    ("open_config", "설정 파일 열기"),
    ("reload_config", "설정 파일 다시 로드"),
    ("inspector", "Ghostty 인스펙터 열기"),
    ("reset", "터미널 세션 리셋"),
    ("quit", "Ghostty 종료"),
];

fn action_description(action: &str) -> Option<&'static str> {
    let base = action.split(':').next().unwrap_or(action);
    GHOSTTY_ACTIONS
        .iter()
        .find(|(a, _)| *a == action || a.split(':').next() == Some(base))
        .map(|(_, d)| *d)
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
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg("POSIX path of (choose folder with prompt \"Ghostty 작업 디렉터리 선택\")")
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
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg("POSIX path of (choose file with prompt \"Ghostty 설정 파일 선택\")")
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
    /// Per-row retained component state, created lazily on first render.
    /// Values held here are the editing surface; the file is only written
    /// once the row's subscription has both the key and the new text.
    text_inputs: HashMap<&'static str, Entity<InputState>>,
    selects: HashMap<&'static str, Entity<SelectState<SearchableVec<SharedString>>>>,
    colors: HashMap<&'static str, Entity<ColorPickerState>>,
    sliders: HashMap<&'static str, Entity<SliderState>>,
    _subscriptions: Vec<Subscription>,
    active_modal: Option<ActiveModal>,
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
            text_inputs: HashMap::new(),
            selects: HashMap::new(),
            colors: HashMap::new(),
            sliders: HashMap::new(),
            _subscriptions: vec![subscription],
            active_modal: None,
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
        // Row states mirror the file, so drop them and let render rebuild.
        self.text_inputs.clear();
        self.selects.clear();
        self.colors.clear();
        self.sliders.clear();
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

    fn reset_key(&mut self, key: &str, cx: &mut Context<Self>) {
        self.file.remove(key);
        self.text_inputs.remove(key);
        self.selects.remove(key);
        self.colors.remove(key);
        self.sliders.remove(key);
        self.notice = None;
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

    fn get_or_create_input(
        &mut self,
        key: &'static str,
        hint: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if !self.text_inputs.contains_key(key) {
            let seed = self.file.get(key).unwrap_or_default();
            let placeholder = if !hint.is_empty() { hint } else { "값 입력" };
            let state = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(placeholder)
                    .default_value(seed)
            });
            let sub = cx.subscribe_in(&state, window, move |this, state, event, _, cx| {
                if matches!(event, InputEvent::Change) {
                    let v = state.read(cx).value().to_string();
                    if v.trim().is_empty() {
                        this.file.remove(key);
                    } else {
                        this.file.set(key, v.trim());
                    }
                    this.notice = None;
                    cx.notify();
                }
            });
            self._subscriptions.push(sub);
            self.text_inputs.insert(key, state);
        }
        self.text_inputs.get(key).unwrap().clone()
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
        if !self.text_inputs.contains_key(key) {
            let seed = self.file.get(key).unwrap_or_else(|| default_val.to_string());
            let state = cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(seed)
                    .min(min)
                    .max(max)
                    .step(step)
            });
            let sub = cx.subscribe_in(&state, window, move |this, state, event, _, cx| {
                if matches!(event, InputEvent::Change) {
                    let v = state.read(cx).value().to_string();
                    if v.trim().is_empty() {
                        this.file.remove(key);
                    } else {
                        this.file.set(key, v.trim());
                    }
                    this.sliders.remove(key);
                    this.notice = None;
                    cx.notify();
                }
            });
            self._subscriptions.push(sub);
            self.text_inputs.insert(key, state);
        }
        self.text_inputs.get(key).unwrap().clone()
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
        let opt_label = lookup(key).map(|o| o.label).unwrap_or(key);
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
            "키 바인딩 설정 (`keybind`)".to_string()
        } else if is_font {
            "글꼴 우선순위 설정 (`font-family`)".to_string()
        } else if is_feature {
            "OpenType 기능 설정 (`font-feature`)".to_string()
        } else if is_config {
            "추가 설정 파일 불러오기 (`config-file`)".to_string()
        } else {
            format!("{opt_label} 목록 편집 (`{key}`)")
        };

        let subtitle = if is_keybind {
            "단축키 입력을 녹음하고 실행할 Ghostty 동작을 지정합니다."
        } else if is_font {
            "시스템에 설치된 폰트를 선택하거나 인기 코딩 폰트를 추가하여 우선순위를 구성합니다."
        } else if is_feature {
            "폰트의 프로그래밍 합자(Ligatures) 및 특수 글리프 기능을 켜고 끕니다."
        } else if is_config {
            "파일 탐색기로 추가 설정 파일을 찾아보거나 직접 경로를 추가합니다."
        } else {
            "설정 파일에 반복 지정되는 항목 목록을 관리합니다."
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
                        .child("등록된 항목이 없습니다.")
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
                                            .tooltip("삭제")
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
                                            .tooltip("삭제")
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
                    div().text_xs().font_semibold().text_color(cx.theme().foreground).child("새 키 바인딩 추가")
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
                                            .child(div().text_xs().font_medium().child("키보드 입력 대기 중... (Esc: 취소)"))
                                    } else if trigger_val.is_empty() {
                                        h_flex()
                                            .gap_1p5()
                                            .items_center()
                                            .child(Icon::new(IconName::Keyboard).xsmall().text_color(cx.theme().muted_foreground))
                                            .child(div().text_xs().text_color(cx.theme().muted_foreground).child("클릭하여 단축키 입력..."))
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
                                            .child(div().text_xs().text_color(cx.theme().muted_foreground).child(format!("({trigger_val}) - 재입력 클릭")))
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
                                .label("추가")
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
                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child("자주 쓰는 동작 빠른 선택 (클릭 시 자동 선택):"))
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
                                        .child(format!("{desc} ({act})"))
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
                    div().text_xs().font_semibold().text_color(cx.theme().foreground).child("새 글꼴 추가 (시스템 설치 폰트 선택)")
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
                                .label("글꼴 추가")
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
                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child("인기 코딩 폰트 빠른 추가:"))
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
                        .label("파일 찾아보기")
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
                        .label("경로 추가")
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
            const POPULAR_FEATURES: &[(&str, &str)] = &[
                ("-calt", "합자 끄기"),
                ("+calt", "합자 켜기"),
                ("+liga", "기본 합자"),
                ("+dlig", "임의 합자"),
                ("+zero", "슬래시 0"),
                ("+ss01", "스타일셋 1"),
                ("+ss02", "스타일셋 2"),
                ("+cv01", "문자변형 1"),
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
                    div().text_xs().font_semibold().text_color(cx.theme().foreground).child("새 OpenType 기능 추가")
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
                                .label("추가")
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
                        .child(div().text_xs().text_color(cx.theme().muted_foreground).child("자주 쓰는 기능 빠른 추가 (1클릭):"))
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
                                        .child(format!("{feat} ({desc})"))
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
                        .label("항목 추가")
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
                    .label("취소")
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
                    .label("적용하기")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(ActiveModal::ListEditor { key, items, .. }) = &this.active_modal {
                            let k = *key;
                            let items_clone = items.clone();
                            this.file.set_all(k, &items_clone);
                            this.active_modal = None;
                            this.notice = Some(format!("'{k}' 설정이 반영되었습니다."));
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
                                    .child(div().text_base().font_semibold().child("변경 사항 미리보기 (Diff)")),
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
                                    .label("닫기")
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
                                    .label("이대로 저장")
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
fn parse_hex_to_hsla(s: &str) -> Option<gpui_kit::Hsla> {
    let h = s.trim().strip_prefix('#')?;
    let (r, g, b) = match h.len() {
        3 => (
            u8::from_str_radix(&h[0..1].repeat(2), 16).ok()?,
            u8::from_str_radix(&h[1..2].repeat(2), 16).ok()?,
            u8::from_str_radix(&h[2..3].repeat(2), 16).ok()?,
        ),
        6 => (
            u8::from_str_radix(&h[0..2], 16).ok()?,
            u8::from_str_radix(&h[2..4], 16).ok()?,
            u8::from_str_radix(&h[4..6], 16).ok()?,
        ),
        _ => return None,
    };
    Some(
        gpui_kit::Rgba {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
        .into(),
    )
}

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
    presets: &'static [(&'static str, &'static str)],
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

    if !this.sliders.contains_key(key) {
        let cur: f32 = val_str.parse().unwrap_or(default_val as f32);
        let state = cx.new(|_cx| {
            SliderState::new()
                .min(min as f32)
                .max(max as f32)
                .step(step as f32)
                .default_value(cur)
        });
        cx.subscribe(&state, move |this, _, event, cx| {
            let (SliderEvent::Change(val) | SliderEvent::Release(val)) = event;
            let new_val = if is_float {
                format!("{:.2}", val.start())
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            } else {
                format!("{}", val.start().round() as i64)
            };
            this.file.set(key, &new_val);
            this.text_inputs.remove(key);
            this.notice = None;
            cx.notify();
        })
        .detach();
        this.sliders.insert(key, state);
    }

    let slider_state = this.sliders.get(key).unwrap().clone();
    let view = cx.entity();

    h_flex()
        .gap_2()
        .items_center()
        .child(
            div()
                .w(px(140.))
                .child(Slider::new(&slider_state))
        )
        .child(
            div()
                .w(px(110.))
                .child(num_input)
        )
        .children(if !presets.is_empty() {
            Some(
                h_flex()
                    .gap_1()
                    .flex_wrap()
                    .children(presets.iter().map(|(label, target)| {
                        let is_active = val_str == *target;
                        let view = view.clone();
                        let target_str = target.to_string();
                        div()
                            .id(format!("{key}-preset-{label}"))
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
                                    this.file.set(key, &target_str);
                                    this.text_inputs.remove(key);
                                    this.sliders.remove(key);
                                    this.notice = None;
                                    cx.notify();
                                });
                            })
                    }))
            )
        } else {
            None
        })
        .into_any_element()
}

fn render_chips_only(
    key: &'static str,
    current_val: &str,
    chips: &'static [(&'static str, &'static str)],
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
                        this.file.set(key, &target_str);
                        this.text_inputs.remove(key);
                        this.sliders.remove(key);
                        this.notice = None;
                        cx.notify();
                    });
                })
        }))
        .into_any_element()
}
fn render_input_with_chips(
    state: &Entity<InputState>,
    key: &'static str,
    current_val: &str,
    chips: &'static [(&'static str, &'static str)],
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
                                this.file.set(key, &target_str);
                                this.text_inputs.remove(key);
                                this.notice = None;
                                cx.notify();
                            });
                        })
                }))
        )
        .into_any_element()
}

fn value_widget(
    this: &mut SettingsView,
    opt: &'static Opt,
    window: &mut Window,
    cx: &mut Context<SettingsView>,
) -> gpui_kit::AnyElement {
    match opt.kind {
        Kind::Bool => {
            let on = matches!(this.file.get(opt.key).as_deref(), Some("true"));
            let view = cx.entity();
            let key = opt.key;
            Switch::new(opt.key)
                .checked(on)
                .on_change(move |&value, _, cx| {
                    view.update(cx, |this, cx| {
                        if value {
                            this.file.set(key, "true");
                        } else {
                            this.file.set(key, "false");
                        }
                        this.notice = None;
                        cx.notify();
                    });
                })
                .into_any_element()
        }
        Kind::Enum(items) => {
            if !this.selects.contains_key(opt.key) {
                let items_vec: Vec<SharedString> =
                    items.iter().map(|s| s.to_string().into()).collect();
                let current = this.file.get(opt.key);
                let selected = current
                    .as_ref()
                    .and_then(|v| items_vec.iter().position(|x| x.as_ref() == v))
                    .map(IndexPath::new);
                let state = cx.new(|cx| {
                    SelectState::new(SearchableVec::new(items_vec), selected, window, cx)
                });
                let key = opt.key;
                cx.subscribe(&state, move |this, _, event, cx| {
                    let SelectEvent::Confirm(value) = event;
                    match value {
                        Some(v) => this.file.set(key, v.as_ref()),
                        None => this.file.remove(key),
                    }
                    this.notice = None;
                    cx.notify();
                })
                .detach();
                this.selects.insert(opt.key, state);
            }
            let state = this.selects.get(opt.key).unwrap();
            div()
                .max_w(px(260.))
                .child(Select::new(state).small())
                .into_any_element()
        }
        Kind::Int { .. } | Kind::Float { .. } | Kind::Text => {
            let key = opt.key;
            let current_val = this.file.get(key).unwrap_or_default();

            match key {
                "font-size" => render_bounded_slider_number(
                    this,
                    key,
                    &current_val,
                    13.0,
                    8.0,
                    72.0,
                    1.0,
                    true,
                    Some("pt"),
                    &[
                        ("11", "11"),
                        ("12", "12"),
                        ("13", "13"),
                        ("14", "14"),
                        ("15", "15"),
                        ("16", "16"),
                        ("18", "18"),
                        ("20", "20"),
                    ],
                    window,
                    cx,
                ),
                "background-opacity" => render_bounded_slider_number(
                    this,
                    key,
                    &current_val,
                    1.0,
                    0.0,
                    1.0,
                    0.05,
                    true,
                    None,
                    &[
                        ("100%", "1.0"),
                        ("95%", "0.95"),
                        ("90%", "0.9"),
                        ("85%", "0.85"),
                        ("80%", "0.8"),
                        ("70%", "0.7"),
                        ("50%", "0.5"),
                    ],
                    window,
                    cx,
                ),
                "cursor-opacity" => render_bounded_slider_number(
                    this,
                    key,
                    &current_val,
                    1.0,
                    0.0,
                    1.0,
                    0.05,
                    true,
                    None,
                    &[
                        ("100%", "1.0"),
                        ("80%", "0.8"),
                        ("60%", "0.6"),
                        ("40%", "0.4"),
                    ],
                    window,
                    cx,
                ),
                "minimum-contrast" => render_bounded_slider_number(
                    this,
                    key,
                    &current_val,
                    1.0,
                    1.0,
                    21.0,
                    0.5,
                    true,
                    None,
                    &[
                        ("1.0 (끔)", "1.0"),
                        ("3.0 (최소)", "3.0"),
                        ("4.5 (권장)", "4.5"),
                        ("7.0 (강화)", "7.0"),
                    ],
                    window,
                    cx,
                ),
                "window-width" => {
                    let num_state = this.get_or_create_number_input(key, "80", 20.0, 500.0, 10.0, window, cx);
                    let num_input = NumberInput::new(&num_state).small().suffix(
                        div().text_xs().text_color(cx.theme().muted_foreground).child("열"),
                    );
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(div().w(px(120.)).child(num_input))
                        .child(render_chips_only(key, &current_val, &[("80", "80"), ("100", "100"), ("120", "120"), ("140", "140")], cx.entity(), cx))
                        .into_any_element()
                }
                "window-height" => {
                    let num_state = this.get_or_create_number_input(key, "24", 10.0, 200.0, 5.0, window, cx);
                    let num_input = NumberInput::new(&num_state).small().suffix(
                        div().text_xs().text_color(cx.theme().muted_foreground).child("행"),
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
                    0.0,
                    255.0,
                    16.0,
                    false,
                    None,
                    &[
                        ("0 (보통)", "0"),
                        ("64", "64"),
                        ("128 (중간)", "128"),
                        ("192", "192"),
                        ("255 (최대)", "255"),
                    ],
                    window,
                    cx,
                ),
                "working-directory" => {
                    let state = this.get_or_create_input(key, opt.hint, window, cx);
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
                                .label("찾아보기")
                                .tooltip("시스템 폴더 선택기로 디렉터리 찾아보기")
                                .on_click(cx.listener(|_this, _, _, cx| {
                                    cx.spawn(async move |this, cx| {
                                        let result = cx
                                            .background_spawn(async move { pick_folder() })
                                            .await;

                                        this.update(cx, |this, cx| {
                                            if let Some(path) = result {
                                                this.file.set("working-directory", &path);
                                                this.text_inputs.remove("working-directory");
                                                this.notice = None;
                                                cx.notify();
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
                    if !this.selects.contains_key("theme") {
                        let theme_names = get_ghostty_themes();
                        let items_vec: Vec<SharedString> =
                            theme_names.iter().map(|s| s.clone().into()).collect();
                        let current = this.file.get("theme");
                        let selected = current
                            .as_ref()
                            .and_then(|v| items_vec.iter().position(|x| x.as_ref() == v))
                            .map(IndexPath::new);
                        let state = cx.new(|cx| {
                            SelectState::new(SearchableVec::new(items_vec), selected, window, cx)
                        });
                        cx.subscribe(&state, move |this, _, event, cx| {
                            let SelectEvent::Confirm(value) = event;
                            match value {
                                Some(v) => this.file.set("theme", v.as_ref()),
                                None => this.file.remove("theme"),
                            }
                            this.notice = None;
                            cx.notify();
                        })
                        .detach();
                        this.selects.insert("theme", state);
                    }
                    let theme_select = this.selects.get("theme").unwrap();
                    div()
                        .max_w(px(260.))
                        .child(Select::new(theme_select).small())
                        .into_any_element()
                }
                "command" => {
                    let state = this.get_or_create_input(key, opt.hint, window, cx);
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
                    let state = this.get_or_create_input(key, opt.hint, window, cx);
                    render_input_with_chips(
                        &state,
                        key,
                        &current_val,
                        &[
                            ("끔 (false)", "false"),
                            ("은은하게 (10)", "10"),
                            ("기본 (20)", "20"),
                            ("강하게 (40)", "40"),
                            ("Glass Regular", "macos-glass-regular"),
                            ("Glass Clear", "macos-glass-clear"),
                        ],
                        cx.entity(),
                        cx,
                    )
                }
                "scrollback-limit" => {
                    let state = this.get_or_create_input(key, opt.hint, window, cx);
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
                            ("무제한 (0)", "0"),
                        ],
                        cx.entity(),
                        cx,
                    )
                }
                "window-padding-x" | "window-padding-y" => {
                    let state = this.get_or_create_input(key, opt.hint, window, cx);
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
                    let state = this.get_or_create_input(key, opt.hint, window, cx);
                    render_input_with_chips(
                        &state,
                        key,
                        &current_val,
                        &[
                            ("1x (느림)", "1"),
                            ("2x", "2"),
                            ("3x (기본)", "3"),
                            ("5x (빠름)", "5"),
                        ],
                        cx.entity(),
                        cx,
                    )
                }
                "adjust-cell-width" | "adjust-cell-height" => {
                    let state = this.get_or_create_input(key, opt.hint, window, cx);
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
                    let state = this.get_or_create_input(key, opt.hint, window, cx);
                    render_input_with_chips(
                        &state,
                        key,
                        &current_val,
                        &[("기본값 복원", "\\t'\"│`|:;,()[]{}<>$")],
                        cx.entity(),
                        cx,
                    )
                }
                _ => {
                    let state = this.get_or_create_input(key, opt.hint, window, cx);
                    div()
                        .max_w(px(260.))
                        .child(Input::new(&state).small())
                        .into_any_element()
                }
            }
        }
        Kind::Color { .. } => {
            if !this.colors.contains_key(opt.key) {
                let current = this.file.get(opt.key);
                let state = cx.new(|cx| {
                    let mut s = ColorPickerState::new(window, cx);
                    if let Some(val) = current.as_deref() {
                        if let Some(h) = parse_hex_to_hsla(val) {
                            s.set_value(h, window, cx);
                        }
                    }
                    s
                });
                let key = opt.key;
                cx.subscribe(&state, move |this, _, event, cx| {
                    if let ColorPickerEvent::Change(Some(color)) = event {
                        let rgba = color.to_rgb();
                        let hex = format!(
                            "#{:02x}{:02x}{:02x}",
                            (rgba.r * 255.0).round() as u8,
                            (rgba.g * 255.0).round() as u8,
                            (rgba.b * 255.0).round() as u8
                        );
                        this.file.set(key, &hex);
                        this.notice = None;
                        cx.notify();
                    }
                })
                .detach();
                this.colors.insert(opt.key, state);
            }
            let state = this.colors.get(opt.key).unwrap();
            let current_text = this.file.get(opt.key).unwrap_or_default();
            h_flex()
                .items_center()
                .gap_2()
                .child(ColorPicker::new(state).small())
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
                            "항목 추가…".to_string()
                        } else {
                            format!("편집 ({}개)", all.len())
                        })
                        .on_click(move |_, window, cx| {
                            view.update(cx, |this, cx| {
                                let items = this.file.get_all(key);
                                let recorder_focus = cx.focus_handle();
                                let new_item_input = cx.new(|cx| InputState::new(window, cx).placeholder("새 항목 입력"));
                                let action_select = if is_keybind {
                                    let action_items: Vec<SharedString> = GHOSTTY_ACTIONS
                                        .iter()
                                        .map(|(act, desc)| format!("{act} · {desc}").into())
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
                                            if let Some(ActiveModal::ListEditor { selected_action, .. }) = &mut this.active_modal {
                                                *selected_action = act;
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
                                            if let Some(ActiveModal::ListEditor { selected_font, .. }) = &mut this.active_modal {
                                                *selected_font = val.to_string();
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
                                this.active_modal = Some(ActiveModal::ListEditor {
                                    key,
                                    items,
                                    recorded_trigger: String::new(),
                                    selected_action,
                                    action_select,
                                    font_select,
                                    selected_font,
                                    is_recording: false,
                                    recorder_focus,
                                    new_item_input,
                                });
                                cx.notify();
                            });
                        }),
                )
                .child(
                    if all.is_empty() {
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("미설정")
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
                                        .child(format!("외 {}개", all.len() - 2)),
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
            let current_flags: Vec<String> = this
                .file
                .get(opt.key)
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
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
                                if new_flags.is_empty() {
                                    this.file.remove(key);
                                } else {
                                    this.file.set(key, &new_flags.join(","));
                                }
                                this.notice = None;
                                cx.notify();
                            });
                        })
                }))
                .into_any_element()
        }
    }
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
    let doc = opt.doc;
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
        div().w(px(36.)).flex().justify_center().child(
            Button::new(format!("reset-{key2}"))
                .ghost()
                .xsmall()
                .icon(IconName::RotateCcw)
                .tooltip("기본값으로 되돌리기")
                .on_click(move |_, _, cx| {
                    view.update(cx, |this, cx| this.reset_key(key2, cx));
                }),
        )
    } else {
        div().w(px(36.))
    };

    row.child(
        v_flex()
            .id(opt_id)
            .w(px(250.))
            .gap_0()
            .tooltip(move |window, cx| Tooltip::new(doc).build(window, cx))
            .child(
                h_flex()
                    .gap_1p5()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .text_color(cx.theme().foreground)
                            .child(opt.label),
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
    .child(
        div()
            .flex_1()
            .child(value_widget(this, opt, window, cx)),
    )
    .child(
        div()
            .w(px(72.))
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
                    .child("설정됨")
                    .into_any_element()
            } else {
                div()
                    .px_2()
                    .py(px(2.))
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("기본값")
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
                                    .child("설정"),
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
                        Button::new("reload")
                            .ghost()
                            .small()
                            .icon(IconName::RefreshCw)
                            .tooltip("파일 다시 불러오기")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.revert(cx);
                            })),
                    )
                    .child(
                        if dirty {
                            Button::new("revert")
                                .outline()
                                .small()
                                .label("변경 취소")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.revert(cx);
                                }))
                        } else {
                            Button::new("revert")
                                .ghost()
                                .small()
                                .label("변경 취소")
                                .disabled(true)
                        },
                    )
                    .child(
                        if dirty {
                            Button::new("diff")
                                .outline()
                                .small()
                                .icon(IconName::FileText)
                                .label("변경 미리보기")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.active_modal = Some(ActiveModal::DiffViewer);
                                    cx.notify();
                                }))
                        } else {
                            Button::new("diff")
                                .ghost()
                                .small()
                                .icon(IconName::FileText)
                                .label("변경 미리보기")
                                .disabled(true)
                        },
                    )
                    .child(
                        if dirty {
                            Button::new("save")
                                .primary()
                                .small()
                                .icon(IconName::Check)
                                .label("저장")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.save(cx);
                                }))
                        } else {
                            Button::new("save")
                                .outline()
                                .small()
                                .icon(IconName::Check)
                                .label("저장됨")
                                .disabled(true)
                        },
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
            SidebarMenuItem::new(cat.label)
                .icon(category_icon(i))
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
            (cat.label.to_string(), cat.desc.to_string(), Some(category_icon(self.category)))
        } else {
            (
                format!("검색 결과 ({}개)", self.visible_opts().len()),
                "키, 이름, 설명에서 일치하는 옵션입니다.".to_string(),
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
                            .child(format!("{}/{} 설정됨", self.set_count(), self.visible_opts().len())),
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
                        .child("일치하는 옵션이 없습니다"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("다른 키워드로 검색하거나 검색어를 지워보세요."),
                )
                .child(
                    Button::new("clear-search")
                        .outline()
                        .small()
                        .icon(IconName::X)
                        .label("검색어 지우기")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.clear_search(window, cx);
                        })),
                )
        } else {
            let opts = self.visible_opts();
            let total = opts.len();
            v_flex()
                .w_full()
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
                                .w(px(250.))
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().muted_foreground)
                                .child("옵션"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().muted_foreground)
                                .child("값"),
                        )
                        .child(
                            div()
                                .w(px(72.))
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().muted_foreground)
                                .child("상태"),
                        )
                        .child(div().w(px(36.))),
                )
                .children(opts.into_iter().enumerate().map(|(i, opt)| {
                    row(self, opt, i == total - 1, window, cx)
                }))
        };

        let content = v_flex()
            .flex_1()
            .min_w_0()
            .child(div().px_6().pt_5().pb_3().child(header))
            .child(
                div()
                    .flex_1()
                    .overflow_y_scrollbar()
                    .id("option-list")
                    .px_6()
                    .pb_6()
                    .child(list),
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
                                .child("검색 포커스"),
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
                                .child("저장"),
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
                        "저장되지 않은 변경사항"
                    } else {
                        "동기화됨"
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

        let mut root = v_flex()
            .size_full()
            .relative()
            .key_context("Settings")
            .on_action(cx.listener(Self::commit_save))
            .on_action(cx.listener(Self::focus_search))
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(titlebar)
            .child(h_flex().items_stretch().flex_1().child(sidebar).child(content))
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
    fn parse_hex_colors() {
        assert!(parse_hex_to_hsla("#fff").is_some());
        assert!(parse_hex_to_hsla("#ffffff").is_some());
        assert!(parse_hex_to_hsla("#123456").is_some());
        assert!(parse_hex_to_hsla("not-a-color").is_none());
        assert!(parse_hex_to_hsla("#abcd").is_none());
        assert!(parse_hex_to_hsla("#xyz").is_none());
    }

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
        assert_eq!(action_description("copy_to_clipboard"), Some("클립보드에 복사"));
        assert_eq!(action_description("new_tab"), Some("새 탭 열기"));
        assert_eq!(action_description("increase_font_size:1"), Some("글꼴 크기 확대 (+1)"));
        assert_eq!(action_description("unknown_action_xyz"), None);
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
