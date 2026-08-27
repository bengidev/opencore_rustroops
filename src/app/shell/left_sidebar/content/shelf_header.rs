//! Collapsible shelf headers for snoozed and settled thread tails.

use gpui::{
    App, InteractiveElement, IntoElement, MouseButton, ParentElement, Radians, SharedString,
    Styled, Window, div, px, relative,
};
use gpui_component::{Icon, IconName, Sizable, h_flex};

use crate::shared::theme::{BorderToken, ForegroundToken, OpenCoreTheme, SpacingToken, TypeRole};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShelfTone {
    Snoozed,
    Settled,
}

pub fn sidebar_shelf_header(
    label: impl Into<SharedString>,
    expanded: bool,
    tone: ShelfTone,
    theme: &OpenCoreTheme,
    on_toggle: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    let (label_color, line_color) = shelf_colors(tone, theme);
    let chevron_rotation = if expanded {
        Radians(std::f32::consts::PI)
    } else {
        Radians(0.)
    };

    h_flex()
        .id(format!(
            "left-sidebar-shelf-{}",
            label.split(' ').next().unwrap_or("shelf")
        ))
        .w_full()
        .min_w_0()
        .mt(px(SpacingToken::S3.value()))
        .mb(px(4.))
        .px(px(10.))
        .items_center()
        .gap(px(8.))
        .overflow_hidden()
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, move |_, window, cx| on_toggle(window, cx))
        .child(
            div()
                .flex_shrink_0()
                .font_family(mono_family())
                .text_size(px(TypeRole::LabelMd.size()))
                .line_height(relative(TypeRole::LabelMd.line_height()))
                .text_color(label_color)
                .child(label),
        )
        .child(div().flex_1().min_w_0().h(px(1.)).bg(line_color))
        .child(
            Icon::new(IconName::ChevronDown)
                .text_color(label_color)
                .small()
                .flex_shrink_0()
                .transform(gpui::Transformation::rotate(chevron_rotation)),
        )
}

fn shelf_colors(tone: ShelfTone, theme: &OpenCoreTheme) -> (gpui::Hsla, gpui::Hsla) {
    match tone {
        ShelfTone::Snoozed => {
            let blue = gpui::rgb(0x3B82F6).into();
            (blue, blue.alpha(0.2))
        }
        ShelfTone::Settled => {
            let muted = theme.foreground(ForegroundToken::Muted);
            let border = theme.border_token(BorderToken::Default);
            (muted.alpha(0.55), border.alpha(0.6))
        }
    }
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}
