//! The window's standing chrome: title bar, navigation sidebar, option table
//! and status bar.
//!
//! `Render::render` used to build all four in one 609-line body that read nine
//! of the view's fields, so changing one section meant reading past the other
//! three. Each section is a method here, taking only what it draws: the title
//! bar takes the dirty flag, the table takes the window because it builds rows,
//! and the sidebar and status bar take nothing but the context.

use super::*;

impl SettingsView {

    pub(super) fn title_bar(&self, dirty: bool, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let short_path = self
            .path
            .strip_prefix(std::env::var("HOME").unwrap_or_default())
            .map(|path| format!("~/{}", path.display()))
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

        titlebar.into_any_element()
    }

    pub(super) fn nav_sidebar(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
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

        sidebar.into_any_element()
    }

    pub(super) fn option_table(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
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

        content.into_any_element()
    }

    pub(super) fn status_bar(&self, dirty: bool, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
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

        status.into_any_element()
    }
}
