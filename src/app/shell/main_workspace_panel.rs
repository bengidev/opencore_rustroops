//! Center workspace raw concept — monospace chrome and rectangular controls.

use gpui::{
    App, AppContext, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, Styled, Subscription, Window, div,
    prelude::FluentBuilder, px, relative,
};
use gpui_component::{
    Icon, IconName, Sizable,
    button::{Button, ButtonRounded, ButtonVariants as _},
    dock::{Panel, PanelEvent},
    h_flex,
    input::{Input, InputEvent, InputState},
    menu::{DropdownMenu as _, PopupMenu, PopupMenuItem},
    v_flex,
};

use crate::shared::theme::{
    ActionToken, BackgroundToken, BorderToken, ForegroundToken, OpenCoreTheme, SpacingToken,
    TypeRole,
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
    composer: ComposerToolbarState,
    context_percent: u32,
    _input_subscription: Subscription,
}

const COMPOSER_MIN_ROWS: usize = 1;
const COMPOSER_MAX_ROWS: usize = 6;

const MODEL_OPTIONS: &[&str] = &["Claude Opus 4.5", "Claude Sonnet 4", "GPT-5"];
const PRIORITY_OPTIONS: &[&str] = &["High", "Normal"];
const MODE_OPTIONS: &[&str] = &["Build", "Plan", "Ask"];
const ACCESS_OPTIONS: &[&str] = &["Full access", "Read only", "Ask before edits"];
const DEFAULT_CONTEXT_PERCENT: u32 = 85;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ComposerToolbarState {
    model: String,
    priority: String,
    mode: String,
    access: String,
}

impl Default for ComposerToolbarState {
    fn default() -> Self {
        Self {
            model: MODEL_OPTIONS[0].to_string(),
            priority: PRIORITY_OPTIONS[0].to_string(),
            mode: MODE_OPTIONS[0].to_string(),
            access: ACCESS_OPTIONS[0].to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ComposerField {
    Model,
    Priority,
    Mode,
    Access,
}

struct ComposerDropdownSpec<'a> {
    id: &'static str,
    label: &'a str,
    panel: gpui::Entity<MainWorkspacePanel>,
    primary: gpui::Hsla,
    icon: Option<(IconName, gpui::Hsla)>,
    sans: SharedString,
    button_height: f32,
    button_px: f32,
    section: &'static str,
    options: &'static [&'static str],
    field: ComposerField,
}

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
            composer: ComposerToolbarState::default(),
            context_percent: DEFAULT_CONTEXT_PERCENT,
            _input_subscription,
        }
    }

    fn select_composer_option(
        &mut self,
        field: ComposerField,
        value: String,
        cx: &mut Context<Self>,
    ) {
        match field {
            ComposerField::Model => self.composer.model = value,
            ComposerField::Priority => self.composer.priority = value,
            ComposerField::Mode => self.composer.mode = value,
            ComposerField::Access => self.composer.access = value,
        }
        cx.notify();
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
            .child(composer_bar(self, cx, &self.input, &theme, mono, pad))
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
    panel: &MainWorkspacePanel,
    cx: &mut Context<MainWorkspacePanel>,
    input: &gpui::Entity<InputState>,
    theme: &OpenCoreTheme,
    mono: SharedString,
    pad: f32,
) -> impl IntoElement {
    const COMPOSER_MIN_HEIGHT: f32 = 72.;
    const COMPOSER_TEXT: f32 = 16.;
    const TOOLBAR_HEIGHT: f32 = 40.;
    const TOOLBAR_BUTTON_HEIGHT: f32 = 32.;
    const TOOLBAR_BUTTON_PX: f32 = 8.;
    const CONTROL_HEIGHT: f32 = 32.;
    const CONTEXT_WIDTH: f32 = 36.;

    let surface = theme.surface(BackgroundToken::Secondary);
    let border = theme.border_token(BorderToken::Default);
    let border_strong = theme.border_token(BorderToken::Strong);
    let primary = theme.foreground(ForegroundToken::Primary);
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let muted = theme.foreground(ForegroundToken::Muted);
    let action_bg = theme.action(ActionToken::Strong);
    let action_fg = theme.action(ActionToken::StrongText);
    let sans = sans_family();
    let panel_entity = cx.entity().clone();

    v_flex()
        .w_full()
        .gap(px(SpacingToken::S1.value()))
        .px(px(pad))
        .pb(px(pad))
        .pt(px(SpacingToken::S3.value()))
        .border_t_1()
        .border_color(border)
        .child(
            v_flex()
                .w_full()
                .border_1()
                .border_color(border)
                .bg(surface)
                .child(
                    div()
                        .w_full()
                        .min_h(px(COMPOSER_MIN_HEIGHT))
                        .px(px(SpacingToken::S3.value()))
                        .pt(px(SpacingToken::S3.value()))
                        .pb(px(SpacingToken::S1.value()))
                        .child(
                            Input::new(input)
                                .large()
                                .w_full()
                                .text_size(px(COMPOSER_TEXT))
                                .bordered(false)
                                .appearance(false)
                                .cleanable(false),
                        ),
                )
                .child(div().w_full().h(px(1.)).bg(border))
                .child(
                    h_flex()
                        .w_full()
                        .h(px(TOOLBAR_HEIGHT))
                        .px(px(SpacingToken::S3.value()))
                        .py(px(2.))
                        .items_center()
                        .gap(px(2.))
                        .child(composer_dropdown_button(ComposerDropdownSpec {
                            id: "workspace-composer-model",
                            label: &panel.composer.model,
                            panel: panel_entity.clone(),
                            primary,
                            icon: Some((IconName::Cpu, secondary)),
                            sans: sans.clone(),
                            button_height: TOOLBAR_BUTTON_HEIGHT,
                            button_px: TOOLBAR_BUTTON_PX,
                            section: "Model",
                            options: MODEL_OPTIONS,
                            field: ComposerField::Model,
                        }))
                        .child(composer_toolbar_divider(border))
                        .child(composer_dropdown_button(ComposerDropdownSpec {
                            id: "workspace-composer-priority",
                            label: &panel.composer.priority,
                            panel: panel_entity.clone(),
                            primary,
                            icon: None,
                            sans: sans.clone(),
                            button_height: TOOLBAR_BUTTON_HEIGHT,
                            button_px: TOOLBAR_BUTTON_PX,
                            section: "Priority",
                            options: PRIORITY_OPTIONS,
                            field: ComposerField::Priority,
                        }))
                        .child(composer_toolbar_divider(border))
                        .child(composer_dropdown_button(ComposerDropdownSpec {
                            id: "workspace-composer-build",
                            label: &panel.composer.mode,
                            panel: panel_entity.clone(),
                            primary,
                            icon: Some((IconName::Bot, secondary)),
                            sans: sans.clone(),
                            button_height: TOOLBAR_BUTTON_HEIGHT,
                            button_px: TOOLBAR_BUTTON_PX,
                            section: "Mode",
                            options: MODE_OPTIONS,
                            field: ComposerField::Mode,
                        }))
                        .child(composer_toolbar_divider(border))
                        .child(composer_dropdown_button(ComposerDropdownSpec {
                            id: "workspace-composer-access",
                            label: &panel.composer.access,
                            panel: panel_entity.clone(),
                            primary,
                            icon: Some((IconName::Eye, secondary)),
                            sans,
                            button_height: TOOLBAR_BUTTON_HEIGHT,
                            button_px: TOOLBAR_BUTTON_PX,
                            section: "Access",
                            options: ACCESS_OPTIONS,
                            field: ComposerField::Access,
                        }))
                        .child(div().flex_1().min_w(px(8.)))
                        .child(composer_context_badge(
                            primary,
                            surface,
                            border_strong,
                            mono.clone(),
                            CONTROL_HEIGHT,
                            CONTEXT_WIDTH,
                            panel.context_percent,
                        ))
                        .child(
                            div()
                                .id("workspace-composer-send")
                                .debug_selector(|| "workspace-composer-send".to_string())
                                .ml(px(SpacingToken::S1.value()))
                                .child(
                                    Button::new("workspace-send")
                                        .ghost()
                                        .rounded(ButtonRounded::None)
                                        .icon(Icon::new(IconName::ArrowUp).text_color(action_fg))
                                        .h(px(CONTROL_HEIGHT))
                                        .w(px(CONTROL_HEIGHT))
                                        .border_1()
                                        .border_color(border_strong)
                                        .bg(action_bg)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.submit_composer(window, cx);
                                        })),
                                ),
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

fn composer_toolbar_divider(color: gpui::Hsla) -> impl IntoElement {
    div()
        .mx(px(SpacingToken::S1.value()))
        .w(px(1.))
        .h(px(18.))
        .flex_shrink_0()
        .bg(color.alpha(0.75))
}

fn composer_toolbar_label(
    label: impl Into<SharedString>,
    primary: gpui::Hsla,
    sans: SharedString,
) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .mr(px(2.))
        .font_family(sans)
        .text_size(px(TypeRole::LabelMd.size()))
        .line_height(relative(TypeRole::LabelMd.line_height()))
        .text_color(primary)
        .child(label.into())
}

fn composer_section_menu(
    menu: PopupMenu,
    section: impl Into<SharedString>,
    options: &[&'static str],
    selected: &str,
    field: ComposerField,
    panel: gpui::Entity<MainWorkspacePanel>,
) -> PopupMenu {
    let mut menu = menu.label(section).separator();
    for option in options {
        let option = (*option).to_string();
        let panel = panel.clone();
        let checked = option == selected;
        menu = menu.item(
            PopupMenuItem::new(option.clone())
                .checked(checked)
                .on_click(move |_, _, cx| {
                    panel.update(cx, |panel, cx| {
                        panel.select_composer_option(field, option.clone(), cx);
                    });
                }),
        );
    }
    menu
}

fn composer_dropdown_button(spec: ComposerDropdownSpec<'_>) -> impl IntoElement {
    let selected = spec.label.to_string();
    let panel_for_menu = spec.panel.clone();
    Button::new(spec.id)
        .ghost()
        .compact()
        .rounded(ButtonRounded::None)
        .h(px(spec.button_height))
        .px(px(spec.button_px))
        .gap(px(6.))
        .text_color(spec.primary)
        .when_some(spec.icon, |this, (icon, color)| {
            this.icon(Icon::new(icon).text_color(color).small())
        })
        .child(composer_toolbar_label(spec.label, spec.primary, spec.sans))
        .dropdown_caret(true)
        .dropdown_menu(move |menu, _, _| {
            composer_section_menu(
                menu,
                spec.section,
                spec.options,
                &selected,
                spec.field,
                panel_for_menu.clone(),
            )
        })
}

fn composer_context_badge(
    primary: gpui::Hsla,
    surface: gpui::Hsla,
    border: gpui::Hsla,
    mono: SharedString,
    height: f32,
    width: f32,
    percent: u32,
) -> impl IntoElement {
    div()
        .id("workspace-composer-context")
        .w(px(width))
        .h(px(height))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(border)
        .bg(surface)
        .child(
            div()
                .font_family(mono)
                .text_size(px(TypeRole::MonoSm.size()))
                .text_color(primary)
                .child(format!("{percent}")),
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
