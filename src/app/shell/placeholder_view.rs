//! Coming-soon placeholder panel for Editor and Terminal modes.

use gpui::{FontWeight, IntoElement, ParentElement, SharedString, Styled, div, px};
use gpui_component::v_flex;

use crate::shared::theme::{ForegroundToken, OpenCoreTheme, TypeRole};

use super::mode_placeholder::ModePlaceholder;

pub fn render_mode_placeholder(
    theme: OpenCoreTheme,
    placeholder: ModePlaceholder,
) -> impl IntoElement {
    let primary = theme.foreground(ForegroundToken::Primary);
    let muted = theme.foreground(ForegroundToken::Muted);
    let heading = SharedString::from(placeholder.heading);
    let description = SharedString::from(placeholder.description);

    div()
        .flex_1()
        .min_h_0()
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            v_flex()
                .gap_2()
                .max_w(px(420.))
                .px(px(24.))
                .child(
                    div()
                        .text_size(px(TypeRole::DisplayMd.size()))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(primary)
                        .child(heading),
                )
                .child(
                    div()
                        .text_size(px(TypeRole::LabelMd.size()))
                        .text_color(muted)
                        .child(description),
                ),
        )
}
