//! The diff viewer: what saving would write, beside what the file holds now.
//!
//! A section like the list editor: the view owns which modal is open, this
//! section draws it.

use super::*;

impl SettingsView {
    pub(super) fn render_diff_modal(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
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
