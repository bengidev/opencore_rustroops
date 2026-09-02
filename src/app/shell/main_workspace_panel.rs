//! Center workspace raw concept — monospace chrome and rectangular controls.

use gpui::{
    App, AppContext, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, Styled, Subscription, Window, div, px,
    relative,
};
use gpui_component::{
    IconName, Sizable,
    button::{Button, ButtonRounded, ButtonVariants as _},
    dock::{Panel, PanelEvent},
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
};

use crate::shared::theme::{
    BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, SpacingToken, TypeRole,
};

use super::workspace_theme::WorkspaceTheme;

const PANEL_TITLE: &str = "WORKSPACE";
const EMPTY_HEADLINE: &str = "Start a new atom";
const EMPTY_BODY: &str =
    "Ask questions, run commands, and edit files without leaving your machine.";
const COMPOSER_PLACEHOLDER: &str = "Ask anything…";

pub struct MainWorkspacePanel {
    focus_handle: FocusHandle,
    theme: WorkspaceTheme,
    input: gpui::Entity<InputState>,
    _input_subscription: Subscription,
}

const COMPOSER_MIN_ROWS: usize = 1;
const COMPOSER_MAX_ROWS: usize = 6;

impl MainWorkspacePanel {
    pub fn new(window: &mut Window, theme: WorkspaceTheme, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .auto_grow(COMPOSER_MIN_ROWS, COMPOSER_MAX_ROWS)
                .submit_on_enter(true)
                .placeholder(COMPOSER_PLACEHOLDER)
        });
        let _input_subscription = cx.subscribe_in(&input, window, |this, _, event, window, cx| {
            if let InputEvent::PressEnter { shift: false, .. } = event {
                this.submit_composer(window, cx);
            }
        });
        Self {
            focus_handle: cx.focus_handle(),
            theme,
            input,
            _input_subscription,
        }
    }

    fn submit_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.input.read(cx).value().trim().to_string();
        if text.is_empty() {
            return;
        }

        self.input.update(cx, |input, cx| {
            input.set_value("", window, cx);
        });
        cx.notify();
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            .child(composer_bar(cx, &self.input, &theme, mono, pad))
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
    let actions = ["New atom", "Run command", "Open file"];
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
    cx: &mut Context<MainWorkspacePanel>,
    input: &gpui::Entity<InputState>,
    theme: &OpenCoreTheme,
    mono: SharedString,
    pad: f32,
) -> impl IntoElement {
    const COMPOSER_MIN_HEIGHT: f32 = 56.;
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
                .items_end()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .min_h(px(COMPOSER_MIN_HEIGHT))
                        .child(
                            Input::new(input)
                                .large()
                                .w_full()
                                .text_size(px(COMPOSER_TEXT))
                                .bordered(true)
                                .appearance(true)
                                .cleanable(false),
                        ),
                )
                .child(
                    div()
                        .id("workspace-composer-send")
                        .debug_selector(|| "workspace-composer-send".to_string())
                        .child(
                            Button::new("workspace-send")
                                .ghost()
                                .rounded(ButtonRounded::None)
                                .icon(IconName::ArrowUp)
                                .h(px(COMPOSER_MIN_HEIGHT))
                                .w(px(COMPOSER_MIN_HEIGHT))
                                .text_color(primary)
                                .border_1()
                                .border_color(border_strong)
                                .bg(surface)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.submit_composer(window, cx);
                                })),
                        ),
                ),
        )
        .child(
            div()
                .font_family(mono)
                .text_size(px(TypeRole::MonoSm.size()))
                .text_color(muted)
                .child("Enter to send atom · Shift+Enter for newline · ⌘K for commands"),
        )
}

fn mono_family() -> SharedString {
    SharedString::from("Space Mono")
}

fn sans_family() -> SharedString {
    SharedString::from("Space Grotesk")
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use gpui::{Entity, Modifiers, TestAppContext, VisualContext, VisualTestContext};
    use gpui_component::Root;

    use super::super::workspace_theme::WorkspaceTheme;
    use super::*;

    fn init_composer_test(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
    }

    macro_rules! mount_composer_panel {
        ($cx:ident, $panel:ident) => {
            let panel_cell = Rc::new(RefCell::new(None));
            let panel_cell_capture = panel_cell.clone();
            let (_, $cx) = $cx.add_window_view(|window, cx| {
                let panel =
                    cx.new(|cx| MainWorkspacePanel::new(window, WorkspaceTheme::default(), cx));
                panel_cell_capture.borrow_mut().replace(panel.clone());
                Root::new(panel, window, cx)
            });
            let $panel = panel_cell.borrow().clone().expect("composer panel entity");
        };
    }

    fn set_composer_value(
        panel: &Entity<MainWorkspacePanel>,
        value: &str,
        cx: &mut VisualTestContext,
    ) {
        cx.update_window_entity(panel, |panel, window, cx| {
            panel.input.update(cx, |input, cx| {
                input.set_value(value, window, cx);
            });
        });
    }

    fn focus_composer_at_end(panel: &Entity<MainWorkspacePanel>, cx: &mut VisualTestContext) {
        cx.update_window_entity(panel, |panel, window, cx| {
            panel.input.update(cx, |input, cx| {
                let value = input.value();
                let line_count = value.lines().count().max(1);
                let last_line = value.lines().last().unwrap_or("");
                input.set_cursor_position(
                    gpui_component::input::Position::new(
                        (line_count - 1) as u32,
                        last_line.chars().count() as u32,
                    ),
                    window,
                    cx,
                );
            });
        });
    }

    fn composer_value(panel: &Entity<MainWorkspacePanel>, cx: &mut VisualTestContext) -> String {
        cx.read_entity(panel, |panel, cx| panel.input.read(cx).value().to_string())
    }

    #[gpui::test]
    fn composer_enter_submits_non_empty_text(cx: &mut TestAppContext) {
        init_composer_test(cx);
        mount_composer_panel!(cx, panel);

        set_composer_value(&panel, "hello", cx);
        focus_composer_at_end(&panel, cx);
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();

        assert_eq!(composer_value(&panel, cx), "");
    }

    #[gpui::test]
    fn composer_submit_ignores_whitespace_only(cx: &mut TestAppContext) {
        init_composer_test(cx);
        mount_composer_panel!(cx, panel);

        set_composer_value(&panel, "   ", cx);
        cx.update_window_entity(&panel, |panel, window, cx| {
            panel.submit_composer(window, cx);
        });

        assert_eq!(composer_value(&panel, cx), "   ");
    }

    #[gpui::test]
    fn composer_send_button_submits_non_empty_text(cx: &mut TestAppContext) {
        init_composer_test(cx);
        mount_composer_panel!(cx, panel);

        set_composer_value(&panel, "hello", cx);
        cx.run_until_parked();

        let button_bounds = cx
            .debug_bounds("workspace-composer-send")
            .expect("workspace send button should be visible");
        cx.simulate_click(button_bounds.center(), Modifiers::none());
        cx.run_until_parked();

        assert_eq!(composer_value(&panel, cx), "");
    }

    #[gpui::test]
    fn composer_shift_enter_inserts_newline(cx: &mut TestAppContext) {
        init_composer_test(cx);
        mount_composer_panel!(cx, panel);

        set_composer_value(&panel, "line one", cx);
        focus_composer_at_end(&panel, cx);
        cx.simulate_keystrokes("shift-enter");
        cx.run_until_parked();

        assert_eq!(composer_value(&panel, cx), "line one\n");
    }

    #[test]
    fn composer_row_limits_match_spec() {
        assert_eq!(COMPOSER_MIN_ROWS, 1);
        assert_eq!(COMPOSER_MAX_ROWS, 6);
    }
}
