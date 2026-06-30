//! Missing-credentials banner above the chat composer.

use gpui::{ClickEvent, ParentElement, Styled, Window, div, px};
use gpui_component::IconName;
use gpui_component::Sizable;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::h_flex;
use gpui_component::v_flex;

use crate::shared::theme::LegacyTypeRole;

pub(crate) fn credentials_banner(
    border: gpui::Hsla,
    background: gpui::Hsla,
    foreground: gpui::Hsla,
    muted: gpui::Hsla,
    label: LegacyTypeRole,
    on_open_settings: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    on_dismiss: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl gpui::IntoElement {
    h_flex()
        .w_full()
        .gap_2()
        .items_start()
        .px(px(12.))
        .py(px(10.))
        .rounded_md()
        .border_1()
        .border_color(border)
        .bg(background)
        .child(
            v_flex()
                .flex_1()
                .gap_1()
                .child(
                    div()
                        .text_size(px(label.size_px as f32))
                        .text_color(foreground)
                        .child("OpenRouter credentials are not configured"),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(muted)
                        .child(
                            "Set OPENROUTER_API_KEY or OPENROUTER_KEY in your environment, \
                             or save a key locally. Sending is disabled until credentials \
                             are available.",
                        ),
                ),
        )
        .child(
            h_flex()
                .gap_1()
                .flex_shrink_0()
                .child(
                    Button::new("open-credentials-from-banner")
                        .label("Open settings")
                        .xsmall()
                        .ghost()
                        .on_click(on_open_settings),
                )
                .child(
                    Button::new("dismiss-credentials-banner")
                        .icon(IconName::Close)
                        .ghost()
                        .xsmall()
                        .on_click(on_dismiss),
                ),
        )
}
