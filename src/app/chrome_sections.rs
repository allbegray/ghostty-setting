//! The standing sections that draw from a snapshot.
//!
//! These take what they draw and the actions they can ask for, and never
//! the view — which is why they are their own module: whatever a
//! `use super::*` hands them cannot be a view field to reach for. The
//! option table and the rows it builds still take the view, because
//! building a row creates that row's editors; they stay in the chrome
//! module until the row's own seam lands.

use super::*;

use std::path::Path;

use super::chrome::TitleBarActions;

pub(super) struct TitleBarInput<'a> {
    pub(super) path: &'a Path,
    pub(super) dirty: bool,
    pub(super) preview_open: bool,
    pub(super) lang_select: &'a Entity<SelectState<SearchableVec<SharedString>>>,
}

/// The title bar: which file is open, and the actions that act on the whole file.
///
/// Receives what it draws and the actions it can ask for. It cannot open a
/// modal, save, or toggle a flag itself: which modal is open is view state,
/// and asking the view to show the diff viewer goes through
/// [`TitleBarActions::review_changes`].
pub(super) fn title_bar(
    input: &TitleBarInput<'_>,
    actions: &TitleBarActions,
    cx: &mut App,
) -> gpui_kit::AnyElement {
    let TitleBarInput {
        path,
        dirty,
        preview_open,
        lang_select,
    } = *input;
    let short_path = path
        .strip_prefix(std::env::var("HOME").unwrap_or_default())
        .map(|path| format!("~/{}", path.display()))
        .unwrap_or_else(|_| path.display().to_string());
    let full_path = path.display().to_string();

    let reload = actions.reload.clone();
    let discard = actions.discard.clone();
    let save = actions.save.clone();
    let review_changes = actions.review_changes.clone();
    let toggle_preview = actions.toggle_preview.clone();

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
                        .child(div().w(px(108.)).child(Select::new(lang_select).small())),
                )
                .child(div().w(px(1.)).h(px(18.)).bg(cx.theme().border))
                .child(
                    Button::new("reload")
                        .ghost()
                        .small()
                        .icon(IconName::RefreshCw)
                        .tooltip(Text::new("파일 다시 불러오기", "Reload file").s())
                        .on_click(move |_, window, cx| {
                            reload(window, cx);
                        }),
                )
                .child(
                    if dirty {
                        Button::new("revert")
                            .outline()
                            .small()
                            .label(Text::new("변경 취소", "Discard changes").s())
                            .on_click(move |_, window, cx| {
                                discard(window, cx);
                            })
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
                            .on_click(move |_, window, cx| {
                                review_changes(window, cx);
                            })
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
                            .on_click(move |_, window, cx| {
                                save(window, cx);
                            })
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
                        .label(if preview_open { Text::new("미리보기 닫기", "Close preview").s() } else { Text::new("미리보기", "Preview").s() })
                        .on_click(move |_, window, cx| {
                            toggle_preview(window, cx);
                        }),
                ),
        );

    titlebar.into_any_element()
}

/// One destination in the sidebar's list: its label, its icon, and how many of
/// the category's options the file has set.
pub(super) struct NavItem {
    pub(super) label: SharedString,
    pub(super) icon: IconName,
    pub(super) set_count: usize,
}

/// What the sidebar draws: the categories as snapshot rows, which one is active,
/// whether a search is filtering the table, and the search field as an entity
/// handle.
pub(super) struct NavSidebarInput<'a> {
    pub(super) items: &'a [NavItem],
    pub(super) active: usize,
    pub(super) searching: bool,
    pub(super) search_input: &'a Entity<InputState>,
}

/// The standing navigation: the search field, then one entry per category.
///
/// Receives a snapshot of what it draws and one action to ask with. It never
/// learns which view field holds the selected category, or that the file is
/// what says how many options a category has set.
pub(super) fn nav_sidebar(
    input: &NavSidebarInput<'_>,
    on_pick: Rc<dyn Fn(&usize, &mut Window, &mut App)>,
    cx: &mut App,
) -> gpui_kit::AnyElement {
    let NavSidebarInput {
        items,
        active,
        searching,
        search_input,
    } = *input;
    let menu = SidebarMenu::new().children(items.iter().enumerate().map(|(i, item)| {
        let on_pick = on_pick.clone();
        let label = item.label.clone();
        let set = item.set_count;
        SidebarMenuItem::new(label)
            .icon(item.icon)
            .active(!searching && i == active)
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
            .on_click(move |_, window, cx| {
                on_pick(&i, window, cx);
            })
    }));

    let sidebar = Sidebar::new("nav")
        .collapsible(false)
        .header(
            Input::new(search_input)
                .small()
                .cleanable(true)
                .prefix(Icon::new(IconName::Search).small().text_color(cx.theme().muted_foreground)),
        )
        .child(menu);

    sidebar.into_any_element()
}

/// The status bar: the last notice, or the shortcut hints, and the saved/
/// unsaved indicator.
pub(super) fn status_bar(notice: Option<&str>, dirty: bool, cx: &App) -> gpui_kit::AnyElement {
        let status = StatusBar::new()
            .left(
                if let Some(notice) = notice {
                    h_flex()
                        .gap_1p5()
                        .items_center()
                        .text_xs()
                        .text_color(cx.theme().foreground)
                        .child(Icon::new(IconName::RefreshCw).xsmall().text_color(cx.theme().primary))
                        .child(notice.to_string())
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

        status.into_any_element()
    }

