//! The live preview panel and the model behind it.
//!
//! The panel draws a terminal mockup, a palette strip and two quick-switch
//! rows. What it draws is derived from the config file by [`PreviewModel`],
//! whose rules — which key wins, what an unset key falls back to, how far
//! opacity may go — used to sit inline between element constructors, where the
//! only way to check them was to look at the panel.

use super::*;

/// What the preview draws for a file with nothing set.
///
/// The font family and size read their defaults from the control policy, so
/// the preview and the editor's fallbacks cannot drift apart. This is also
/// where the two font-size writers used to be: the preview rested at 13.5
/// while the editor's slider rested at 13.0.
fn default_font_size() -> f32 {
    controls::slider("font-size")
        .expect("font-size has a slider")
        .default as f32
}

fn default_font() -> String {
    controls::DEFAULT_FONT.to_string()
}
const DEFAULT_THEME: &str = "tokyo-night";
const DEFAULT_CURSOR_STYLE: &str = "block";
const DEFAULT_BG: &str = "#1a1b26";
const DEFAULT_FG: &str = "#c0caf5";

/// A window cannot be drawn more transparent than this, so the preview clamps
/// rather than showing an impossible value.
const OPACITY_RANGE: (f32, f32) = (0.2, 1.0);

/// Everything the preview panel draws, read from the file.
pub struct PreviewModel {
    pub font_family: String,
    pub font_size: f32,
    pub theme_name: String,
    pub cursor_style: String,
    /// The opacity the file asks for, before clamping — what the panel reports.
    pub opacity: f32,
    /// The configured background, which the panel prints beside the opacity.
    pub background_hex: String,
    pub background: gpui_kit::Hsla,
    pub foreground: gpui_kit::Hsla,
    pub cursor: gpui_kit::Hsla,
    pub palette: &'static [&'static str],
}

impl PreviewModel {
    /// Read a file into what the panel draws.
    ///
    /// The precedence lives here and nowhere else: a theme Ghostty knows wins
    /// over the file's own `background`, `foreground` and `cursor-color`,
    /// because a theme is a complete palette rather than a single override.
    /// Every other key falls back to the value the preview shows for an
    /// unconfigured file, and opacity is clamped to what a window can draw.
    pub fn from(file: &LineFile) -> Self {
        let font_family = file.get("font-family").unwrap_or_else(default_font);
        let font_size = file
            .get("font-size")
            .and_then(|size| size.parse().ok())
            .unwrap_or_else(default_font_size);
        let theme_name = file.get("theme").unwrap_or_else(|| DEFAULT_THEME.to_string());
        let cursor_style = file
            .get("cursor-style")
            .unwrap_or_else(|| DEFAULT_CURSOR_STYLE.to_string());
        let opacity: f32 = file
            .get("background-opacity")
            .and_then(|value| value.parse().ok())
            .unwrap_or(1.0);

        let theme = lookup_theme_colors(&theme_name);
        let file_background = file.get("background");
        let file_foreground = file.get("foreground");
        let file_cursor = file.get("cursor-color");
        let background = theme
            .map(|theme| theme.bg)
            .or_else(|| file_background.as_deref())
            .unwrap_or(DEFAULT_BG);
        let foreground = theme
            .map(|theme| theme.fg)
            .or_else(|| file_foreground.as_deref())
            .unwrap_or(DEFAULT_FG);
        let cursor = theme
            .map(|theme| theme.cursor)
            .or_else(|| file_cursor.as_deref())
            .unwrap_or(foreground);

        let color = |hex: &str, fallback: gpui_kit::Hsla| {
            value::Stored::new(Some(hex)).color().unwrap_or(fallback)
        };
        let (min, max) = OPACITY_RANGE;
        Self {
            font_family,
            font_size,
            theme_name,
            cursor_style,
            opacity,
            background_hex: background.to_string(),
            background: color(background, gpui_kit::hsla(0.0, 0.0, 0.1, 1.0))
                .alpha(opacity.clamp(min, max)),
            foreground: color(foreground, gpui_kit::hsla(0.0, 0.0, 0.9, 1.0)),
            cursor: color(cursor, gpui_kit::hsla(0.0, 0.0, 0.9, 1.0)),
            palette: theme.map(|theme| theme.palette.as_slice()).unwrap_or(FALLBACK_PALETTE),
        }
    }
}

/// Palette the preview shows when the theme is not one it knows.
const FALLBACK_PALETTE: &[&str] = &[
    "#15161e", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#a9b1d6",
];

/// Colours the preview knows without running Ghostty.
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

impl SettingsView {
    pub(super) fn render_preview_panel(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let view = cx.entity();

        let PreviewModel {
            font_family,
            font_size,
            theme_name,
            cursor_style,
            opacity,
            background_hex: bg_hex,
            background: bg_color,
            foreground: fg_color,
            cursor: cursor_color,
            palette,
        } = PreviewModel::from(&self.file);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::linefile::LineFile;

    fn file(lines: &[&str]) -> LineFile {
        LineFile::parse(&lines.join("\n"))
    }

    #[test]
    fn lookup_finds_known_themes_only() {
        let tokyo = lookup_theme_colors("tokyo-night").expect("tokyo-night is known");
        assert_eq!(tokyo.bg, "#1a1b26");
        assert!(lookup_theme_colors("non_existent_theme").is_none());
    }

    /// An unconfigured file still previews: every key falls back.
    #[test]
    fn an_empty_file_gets_the_defaults() {
        let model = PreviewModel::from(&file(&[]));
        assert_eq!(model.font_family, default_font());
        assert_eq!(model.font_size, default_font_size());
        assert_eq!(model.theme_name, DEFAULT_THEME);
        assert_eq!(model.cursor_style, DEFAULT_CURSOR_STYLE);
        assert_eq!(model.background_hex, DEFAULT_BG);
        assert_eq!(model.palette.len(), 8);
    }

    /// A theme is a whole palette, so it beats the file's single overrides.
    #[test]
    fn a_known_theme_beats_the_files_own_colors() {
        let model = PreviewModel::from(&file(&[
            "theme = tokyo-night",
            "background = #ff0000",
            "foreground = #00ff00",
        ]));
        let tokyo = lookup_theme_colors("tokyo-night").unwrap();
        assert_eq!(model.background_hex, tokyo.bg);
        assert_ne!(model.background_hex, "#ff0000");
    }

    /// Without a known theme, the file's own colors are what is drawn.
    #[test]
    fn file_colors_win_when_the_theme_is_unknown() {
        let model = PreviewModel::from(&file(&["theme = nope", "background = #ff0000"]));
        assert_eq!(model.background_hex, "#ff0000");
    }

    /// Opacity below what a window can draw is clamped, but the panel still
    /// reports what the file asked for.
    #[test]
    fn opacity_is_clamped_for_drawing_but_reported_as_written() {
        let model = PreviewModel::from(&file(&["background-opacity = 0.05"]));
        assert_eq!(model.opacity, 0.05);
        assert_eq!(model.background.a, OPACITY_RANGE.0);

        let model = PreviewModel::from(&file(&["background-opacity = 1.5"]));
        assert_eq!(model.background.a, OPACITY_RANGE.1);
    }

    /// The cursor falls back to the foreground, not to a fixed colour.
    #[test]
    fn an_unset_cursor_takes_the_foreground() {
        let model = PreviewModel::from(&file(&["foreground = #123456"]));
        assert_eq!(model.cursor, model.foreground);
    }
}
