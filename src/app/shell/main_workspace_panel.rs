//! Center workspace raw concept — monospace chrome and rectangular controls.

use gpui::{
    App, AppContext, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, Styled, Window, div, px, relative,
};
use gpui_component::{
    IconName, Sizable,
    button::{Button, ButtonRounded, ButtonVariants as _},
    dock::{Panel, PanelEvent},
    h_flex,
    input::{Input, InputState},
    v_flex,
};

use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, SpacingToken, TypeRole,
};

use super::workspace_theme::WorkspaceTheme;

const PANEL_TITLE: &str = "WORKSPACE";
const EMPTY_HEADLINE: &str = "Start a new session";
const EMPTY_BODY: &str =
    "Ask questions, run commands, and edit files without leaving your machine.";
const COMPOSER_PLACEHOLDER: &str = "Ask anything…";

pub struct MainWorkspacePanel {
    focus_handle: FocusHandle,
    theme: WorkspaceTheme,
    input: gpui::Entity<InputState>,
}

impl MainWorkspacePanel {
    pub fn new(window: &mut Window, theme: WorkspaceTheme, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(COMPOSER_PLACEHOLDER));
        Self {
            focus_handle: cx.focus_handle(),
            theme,
            input,
        }
    }
}

impl EventEmitter<PanelEvent> for MainWorkspacePanel {}

impl Focusable for MainWorkspacePanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for MainWorkspacePanel {
    fn panel_name(&self) -> &'static str {
        "main-stub"
    }

    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .font_family(mono_family())
            .text_size(px(TypeRole::MonoSm.size()))
            .child(PANEL_TITLE)
    }

    fn tab_name(&self, _: &App) -> Option<SharedString> {
        Some(PANEL_TITLE.into())
    }

    fn closable(&self, _: &App) -> bool {
        false
    }

    fn zoomable(&self, _: &App) -> Option<gpui_component::dock::PanelControl> {
        None
    }
}

impl Render for MainWorkspacePanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.get();
        let page = theme.surface(BackgroundToken::Primary);
        let surface = theme.surface(BackgroundToken::Secondary);
        let tertiary = theme.surface(BackgroundToken::Tertiary);
        let border = theme.border_token(BorderToken::Default);
        let border_strong = theme.border_token(BorderToken::Strong);
        let primary = theme.foreground(ForegroundToken::Primary);
        let secondary = theme.foreground(ForegroundToken::Secondary);
        let mono = mono_family();
        let sans = sans_family();
        let pad = SpacingToken::S4.value();

        div()
            .id("main-workspace-panel")
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(page)
            .track_focus(&self.focus_handle)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .px(px(pad))
                    .gap(px(SpacingToken::S3.value()))
                    .child(empty_state_card(
                        surface,
                        border_strong,
                        primary,
                        secondary,
                        mono.clone(),
                        sans,
                    ))
                    .child(quick_actions_row(tertiary, border, primary)),
            )
            .child(composer_bar(&self.input, &theme, mono, pad))
    }
}

fn empty_state_card(
    surface: gpui::Hsla,
    border: gpui::Hsla,
    primary: gpui::Hsla,
    secondary: gpui::Hsla,
    mono: SharedString,
    sans: SharedString,
) -> impl IntoElement {
    v_flex()
        .w_full()
        .max_w(px(420.))
        .items_center()
        .gap(px(SpacingToken::S3.value()))
        .px(px(SpacingToken::S4.value()))
        .py(px(28.))
        .border_1()
        .border_color(border)
        .bg(surface)
        .child(
            div()
                .w(px(40.))
                .h(px(40.))
                .flex()
                .items_center()
                .justify_center()
                .border_1()
                .border_color(border)
                .font_family(mono.clone())
                .text_size(px(20.))
                .text_color(primary)
                .child("+"),
        )
        .child(
            div()
                .font_family(mono.clone())
                .text_size(px(TypeRole::LabelMd.size()))
                .text_color(primary)
                .child(EMPTY_HEADLINE),
        )
        .child(
            div()
                .text_center()
                .max_w(px(320.))
                .text_size(px(TypeRole::LabelMd.size()))
                .line_height(relative(TypeRole::LabelMd.line_height()))
                .font_family(sans)
                .text_color(secondary)
                .child(EMPTY_BODY),
        )
}

fn quick_actions_row(
    surface: gpui::Hsla,
    border: gpui::Hsla,
    primary: gpui::Hsla,
) -> impl IntoElement {
    let actions = ["New session", "Run command", "Open file"];
    h_flex()
        .gap(px(SpacingToken::S1.value()))
        .flex_wrap()
        .justify_center()
        .children(actions.into_iter().enumerate().map(|(index, label)| {
            Button::new(format!("workspace-action-{index}"))
                .ghost()
                .rounded(ButtonRounded::None)
                .label(label)
                .h(px(32.))
                .text_size(px(TypeRole::MonoSm.size()))
                .text_color(primary)
                .border_1()
                .border_color(border)
                .bg(surface)
        }))
}

fn composer_bar(
    input: &gpui::Entity<InputState>,
    theme: &OpenCoreTheme,
    mono: SharedString,
    pad: f32,
) -> impl IntoElement {
    const COMPOSER_HEIGHT: f32 = 56.;
    const COMPOSER_TEXT: f32 = 16.;

    let surface = theme.surface(BackgroundToken::Secondary);
    let border = theme.border_token(BorderToken::Default);
    let border_strong = theme.border_token(BorderToken::Strong);
    let primary = theme.foreground(ForegroundToken::Primary);
    let muted = theme.foreground(ForegroundToken::Muted);

    v_flex()
        .w_full()
        .gap(px(SpacingToken::S1.value()))
        .px(px(pad))
        .pb(px(pad))
        .pt(px(SpacingToken::S3.value()))
        .border_t_1()
        .border_color(border)
        .child(
            h_flex()
                .w_full()
                .gap(px(SpacingToken::S1.value()))
                .items_center()
                .child(
                    div().flex_1().min_w_0().child(
                        Input::new(input)
                            .large()
                            .w_full()
                            .h(px(COMPOSER_HEIGHT))
                            .text_size(px(COMPOSER_TEXT))
                            .bordered(true)
                            .appearance(true)
                            .cleanable(false),
                    ),
                )
                .child(
                    Button::new("workspace-send")
                        .ghost()
                        .rounded(ButtonRounded::None)
                        .icon(IconName::ArrowUp)
                        .h(px(COMPOSER_HEIGHT))
                        .w(px(COMPOSER_HEIGHT))
                        .text_color(primary)
                        .border_1()
                        .border_color(border_strong)
                        .bg(surface),
                ),
        )
        .child(
            div()
                .font_family(mono)
                .text_size(px(TypeRole::MonoSm.size()))
                .text_color(muted)
                .child("Enter to send · ⌘K for commands"),
        )
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}

fn sans_family() -> SharedString {
    SharedString::from("Space Grotesk")
}
