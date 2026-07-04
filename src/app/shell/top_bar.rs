//! Workspace top bar — sidebar toggle, context label, mode tabs, global actions.

use gpui::{
    App, ClickEvent, FontWeight, IntoElement, ParentElement, SharedString, Styled, Window, div, px,
};
use gpui_component::IconName;
use gpui_component::Sizable;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::h_flex;

use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, TypeRole,
};

use super::workspace_mode::WorkspaceMode;

#[allow(clippy::too_many_arguments)]
pub fn shell_top_bar_row(
    theme: OpenCoreTheme,
    context_label: SharedString,
    active_mode: WorkspaceMode,
    on_toggle_sidebar: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_editor: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_chat: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_terminal: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_open_instructions: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_open_credentials: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let border = theme.border_token(BorderToken::Default);
    let background = theme.surface(BackgroundToken::Secondary);
    let primary = theme.foreground(ForegroundToken::Primary);
    let inset = px(theme.spacing.sm as f32);

    h_flex()
        .flex_shrink_0()
        .w_full()
        .h(px(44.))
        .items_center()
        .gap_2()
        .px(inset)
        .border_b_1()
        .border_color(border)
        .bg(background)
        .child(
            Button::new("shell-sidebar-toggle")
                .icon(IconName::PanelLeft)
                .ghost()
                .small()
                .tooltip("Toggle sidebar")
                .on_click(on_toggle_sidebar),
        )
        .child(
            div()
                .min_w(px(0.))
                .flex_shrink(1.)
                .overflow_hidden()
                .text_size(px(TypeRole::LabelMd.size()))
                .font_weight(FontWeight::MEDIUM)
                .text_color(primary)
                .child(context_label),
        )
        .child(div().flex_grow(1.))
        .child(mode_tabs(
            theme,
            active_mode,
            on_select_editor,
            on_select_chat,
            on_select_terminal,
        ))
        .child(div().flex_grow(1.))
        .child(
            h_flex()
                .gap_1()
                .items_center()
                .child(
                    Button::new("shell-open-instructions")
                        .icon(IconName::BookOpen)
                        .ghost()
                        .small()
                        .tooltip("Custom instructions")
                        .on_click(on_open_instructions),
                )
                .child(
                    Button::new("shell-open-credentials")
                        .icon(IconName::Settings)
                        .ghost()
                        .small()
                        .tooltip("OpenRouter credentials")
                        .on_click(on_open_credentials),
                ),
        )
}

fn mode_tabs(
    theme: OpenCoreTheme,
    active: WorkspaceMode,
    on_editor: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_chat: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_terminal: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    h_flex()
        .gap_1()
        .items_center()
        .child(mode_tab_button(
            theme,
            "shell-mode-editor",
            WorkspaceMode::Editor,
            active,
            on_editor,
        ))
        .child(mode_tab_button(
            theme,
            "shell-mode-chat",
            WorkspaceMode::Chat,
            active,
            on_chat,
        ))
        .child(mode_tab_button(
            theme,
            "shell-mode-terminal",
            WorkspaceMode::Terminal,
            active,
            on_terminal,
        ))
}

fn mode_tab_button(
    theme: OpenCoreTheme,
    id: &'static str,
    mode: WorkspaceMode,
    active: WorkspaceMode,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Button {
    let is_active = mode == active;
    let foreground = if is_active {
        theme.foreground(ForegroundToken::Primary)
    } else {
        theme.foreground(ForegroundToken::Muted)
    };
    let mut button = Button::new(id)
        .label(mode.label())
        .ghost()
        .small()
        .text_color(foreground)
        .on_click(on_click);
    if is_active {
        button = button.bg(theme.surface(BackgroundToken::Tertiary));
    }
    button
}
