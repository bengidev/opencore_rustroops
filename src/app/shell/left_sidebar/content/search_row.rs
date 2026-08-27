//! Search row with clear affordance and new-thread action.

use gpui::{
    App, Entity, InteractiveElement, IntoElement, ParentElement, Styled, Window, div, px,
};
use gpui_component::{
    Icon, IconName, Sizable,
    button::{Button, ButtonRounded, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
};

use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, SpacingToken, TypeRole,
};

use super::super::tokens::{ICON_BUTTON_SIZE, SEARCH_ROW_HEIGHT};

pub fn sidebar_search_row(
    search: &Entity<InputState>,
    query: &str,
    theme: &OpenCoreTheme,
    on_clear: impl Fn(&mut Window, &mut App) + 'static,
    on_new_thread: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let surface = theme.surface(BackgroundToken::Secondary);
    let border = theme.border_token(BorderToken::Default);
    let muted = theme.foreground(ForegroundToken::Muted);
    let primary = theme.foreground(ForegroundToken::Primary);

    h_flex()
        .id("left-sidebar-search-row")
        .w_full()
        .min_w_0()
        .gap(px(SpacingToken::S1.value()))
        .items_center()
        .overflow_hidden()
        .child(
            h_flex()
                .flex_1()
                .min_w_0()
                .h(px(SEARCH_ROW_HEIGHT))
                .items_center()
                .gap(px(8.))
                .px(px(8.))
                .border_1()
                .border_color(border)
                .bg(surface)
                .overflow_hidden()
                .child(
                    Icon::new(IconName::Search)
                        .text_color(muted)
                        .small()
                        .flex_shrink_0(),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .child(
                            Input::new(search)
                                .w_full()
                                .h(px(SEARCH_ROW_HEIGHT - 4.))
                                .text_size(px(TypeRole::LabelMd.size()))
                                .bordered(false)
                                .appearance(false)
                                .cleanable(false),
                        ),
                )
                .children(if !query.is_empty() {
                    Some(
                        Button::new("left-sidebar-clear-search")
                            .ghost()
                            .rounded(ButtonRounded::None)
                            .tooltip("Clear search")
                            .icon(Icon::new(IconName::Close).text_color(muted))
                            .h(px(20.))
                            .w(px(20.))
                            .on_click(move |_, window, cx| on_clear(window, cx)),
                    )
                } else {
                    None
                }),
        )
        .child(
            Button::new("left-sidebar-new-thread")
                .ghost()
                .rounded(ButtonRounded::None)
                .tooltip("New thread")
                .icon(Icon::new(IconName::File).text_color(primary))
                .h(px(ICON_BUTTON_SIZE))
                .w(px(ICON_BUTTON_SIZE))
                .flex_shrink_0()
                .border_1()
                .border_color(border)
                .bg(surface)
                .on_click(move |_, window, cx| on_new_thread(window, cx)),
        )
}
