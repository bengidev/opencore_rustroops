//! Flat workspace layout — shell chrome and chat as sibling views under the app root.
//!
//! Uses absolute positioning so the chat entity receives stable bounds from GPUI
//! (flex-nested entities were producing corrupted scroll layout).

use gpui::{
    App, ClickEvent, Entity, IntoElement, ParentElement, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use gpui_component::IconName;
use gpui_component::Sizable;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::h_flex;

use crate::shared::theme::{BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme};

use super::ShellChatAction;
use super::shell_helpers::context_label_from_shell_context;
use super::shell_placeholders::render_mode_placeholder;
use super::shell_state::{ShellCommand, WorkspaceMode};
use super::shell_view::ShellView;

const TOP_BAR_HEIGHT: f32 = 35.0;
const SIDEBAR_STUB_WIDTH: f32 = 235.0;

/// Renders the workspace shell with chat mounted as a sibling entity, not nested.
pub fn render_workspace_shell(
    shell: Entity<ShellView>,
    _window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let theme = shell.read(cx).theme();
    let state = shell.read(cx).state().clone();
    let chat = shell.read(cx).chat_handle().chat_view();
    let context = shell.read(cx).chat_handle().context(cx);
    let context_label = context_label_from_shell_context(&context);

    let background = theme.surface(BackgroundToken::Primary);
    let border = theme.border_token(BorderToken::Default);
    let foreground = theme.foreground(ForegroundToken::Primary);
    let muted = theme.foreground(ForegroundToken::Muted);
    let sidebar_bg = theme.surface(BackgroundToken::Secondary);

    let sidebar_left = if state.sidebar_collapsed {
        px(0.)
    } else {
        px(SIDEBAR_STUB_WIDTH)
    };

    div()
        .relative()
        .size_full()
        .overflow_hidden()
        .bg(background)
        .child(render_center_panel(
            chat,
            state.active_mode,
            theme,
            sidebar_left,
        ))
        .when(!state.sidebar_collapsed, |layer| {
            layer.child(
                div()
                    .absolute()
                    .top(px(TOP_BAR_HEIGHT))
                    .left_0()
                    .bottom_0()
                    .w(px(SIDEBAR_STUB_WIDTH))
                    .overflow_hidden()
                    .border_r_1()
                    .border_color(border)
                    .bg(sidebar_bg),
            )
        })
        .child(render_top_bar(
            shell.clone(),
            theme,
            context_label,
            state.active_mode,
            state.sidebar_collapsed,
            border,
            foreground,
            muted,
        ))
}

fn render_top_bar(
    shell: Entity<ShellView>,
    theme: OpenCoreTheme,
    context_label: &str,
    active_mode: WorkspaceMode,
    sidebar_collapsed: bool,
    border: gpui::Hsla,
    foreground: gpui::Hsla,
    muted: gpui::Hsla,
) -> impl IntoElement {
    let top_bg = theme.surface(BackgroundToken::Secondary);

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .h(px(TOP_BAR_HEIGHT))
        .child(
            h_flex()
                .size_full()
                .px(px(8.))
                .items_center()
                .justify_between()
                .gap(px(8.))
                .border_b_1()
                .border_color(border)
                .bg(top_bg)
                .child(
                    h_flex()
                        .flex_1()
                        .min_w_0()
                        .items_center()
                        .gap(px(6.))
                        .child(
                            Button::new("shell-sidebar-toggle")
                                .icon(IconName::PanelLeft)
                                .ghost()
                                .small()
                                .tooltip(if sidebar_collapsed {
                                    "Show sidebar"
                                } else {
                                    "Hide sidebar"
                                })
                                .on_click({
                                    let shell = shell.clone();
                                    move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                                        let _ = shell.update(cx, |shell, cx| {
                                            shell.apply_command(
                                                ShellCommand::ToggleSidebar,
                                                window,
                                                cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .text_color(foreground)
                                .text_size(px(theme.label.size_px as f32))
                                .child(context_label.to_string()),
                        ),
                )
                .child(render_mode_tabs(
                    shell.clone(),
                    active_mode,
                    muted,
                    foreground,
                ))
                .child(
                    h_flex()
                        .flex_1()
                        .min_w_0()
                        .justify_end()
                        .items_center()
                        .gap(px(4.))
                        .child(
                            Button::new("shell-open-instructions")
                                .icon(IconName::BookOpen)
                                .ghost()
                                .small()
                                .tooltip("Custom instructions")
                                .on_click({
                                    let shell = shell.clone();
                                    move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                                        let _ = shell.update(cx, |shell, cx| {
                                            shell.dispatch_chat_action(
                                                ShellChatAction::OpenCustomInstructions,
                                                window,
                                                cx,
                                            );
                                        });
                                    }
                                }),
                        )
                        .child(
                            Button::new("shell-open-credential-settings")
                                .icon(IconName::Settings)
                                .ghost()
                                .small()
                                .tooltip("OpenRouter credentials")
                                .on_click({
                                    let shell = shell.clone();
                                    move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                                        let _ = shell.update(cx, |shell, cx| {
                                            shell.dispatch_chat_action(
                                                ShellChatAction::OpenCredentialSettings,
                                                window,
                                                cx,
                                            );
                                        });
                                    }
                                }),
                        ),
                ),
        )
}

fn render_mode_tabs(
    shell: Entity<ShellView>,
    active_mode: WorkspaceMode,
    muted: gpui::Hsla,
    foreground: gpui::Hsla,
) -> impl IntoElement {
    let mut tabs = h_flex().items_center().gap(px(2.));
    for mode in WorkspaceMode::ALL {
        let is_active = mode == active_mode;
        let label = mode.label();
        let id = format!("shell-mode-{}", label.to_lowercase());
        let shell = shell.clone();
        let mut button = Button::new(id).label(label).small().on_click(
            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                let _ = shell.update(cx, |shell, cx| {
                    shell.apply_command(ShellCommand::SelectMode(mode), window, cx);
                });
            },
        );
        button = if is_active {
            button.ghost().text_color(foreground)
        } else {
            button.ghost().text_color(muted)
        };
        tabs = tabs.child(button);
    }
    tabs
}

fn render_center_panel(
    chat: Entity<crate::chat::ChatView>,
    active_mode: WorkspaceMode,
    theme: OpenCoreTheme,
    sidebar_left: gpui::Pixels,
) -> impl IntoElement {
    div()
        .absolute()
        .top(px(TOP_BAR_HEIGHT))
        .left(sidebar_left)
        .right_0()
        .bottom_0()
        .overflow_hidden()
        .child(chat)
        .when(active_mode != WorkspaceMode::Chat, |layer| {
            layer.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .overflow_hidden()
                    .child(render_mode_placeholder(active_mode, theme)),
            )
        })
}
