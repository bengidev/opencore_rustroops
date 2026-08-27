//! Project scope filter row with dropdown and new-project affordance.

use gpui::{
    App, InteractiveElement, IntoElement, ParentElement, SharedString, Styled, Window, div, px,
    relative,
};
use gpui_component::{
    Icon, IconName,
    button::{Button, ButtonRounded, ButtonVariants as _},
    h_flex,
    menu::{DropdownMenu as _, PopupMenuItem},
};

use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, SpacingToken, TypeRole,
};

use super::super::demo_data::{ALL_PROJECTS_LABEL, DEMO_PROJECTS};
use super::super::tokens::ICON_BUTTON_SIZE;

pub fn sidebar_project_scope_row(
    scoped_label: &str,
    theme: &OpenCoreTheme,
    on_scope: impl Fn(Option<String>, &mut Window, &mut App) + Clone + 'static,
    on_new_project: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let surface = theme.surface(BackgroundToken::Secondary);
    let border = theme.border_token(BorderToken::Default);
    let primary = theme.foreground(ForegroundToken::Primary);
    let muted = theme.foreground(ForegroundToken::Muted);

    h_flex()
        .id("left-sidebar-project-scope")
        .w_full()
        .min_w_0()
        .gap(px(SpacingToken::S1.value()))
        .items_center()
        .overflow_hidden()
        .child({
            let on_scope = on_scope.clone();
            Button::new("left-sidebar-scope-trigger")
                .ghost()
                .rounded(ButtonRounded::None)
                .h(px(ICON_BUTTON_SIZE))
                .flex_1()
                .min_w_0()
                .border_1()
                .border_color(border)
                .bg(surface)
                .icon(Icon::new(IconName::Folder).text_color(muted))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .font_family(mono_family())
                        .text_size(px(TypeRole::LabelMd.size()))
                        .line_height(relative(TypeRole::LabelMd.line_height()))
                        .text_color(primary)
                        .child(SharedString::from(scoped_label)),
                )
                .dropdown_menu({
                    let on_scope = on_scope.clone();
                    move |menu, _window, _cx| {
                        let mut menu = menu.label("Projects");
                        let on_all = on_scope.clone();
                        menu = menu.item(PopupMenuItem::new(ALL_PROJECTS_LABEL).on_click(
                            move |_, window, cx| {
                                on_all(None, window, cx);
                            },
                        ));
                        for project in DEMO_PROJECTS {
                            let key = project.key.to_string();
                            let label = project.display_name;
                            let on_scope = on_scope.clone();
                            menu = menu.item(
                                PopupMenuItem::new(label)
                                    .icon(Icon::new(IconName::Folder).text_color(muted))
                                    .on_click(move |_, window, cx| {
                                        on_scope(Some(key.clone()), window, cx);
                                    }),
                            );
                        }
                        menu
                    }
                })
        })
        .child(
            Button::new("left-sidebar-new-project")
                .ghost()
                .rounded(ButtonRounded::None)
                .tooltip("New project")
                .icon(Icon::new(IconName::FolderOpen).text_color(primary))
                .h(px(ICON_BUTTON_SIZE))
                .w(px(ICON_BUTTON_SIZE))
                .flex_shrink_0()
                .border_1()
                .border_color(border)
                .bg(surface)
                .on_click(move |_, window, cx| on_new_project(window, cx)),
        )
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}
