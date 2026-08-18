use std::{rc::Rc, time::Instant};

use gpui::{App, Context, IntoElement, ParentElement, Render, Styled, Window, div, px};
use gpui_component::button::{Button, ButtonVariants as _};

use crate::shared::theme::{BackgroundToken, ForegroundToken, OpenCoreTheme};

use super::{DimTween, ShellChrome, TITLEBAR_HEIGHT, TabModel, eval_tween, tween_finished};

/// Callback used by the shell to persist chrome changes at the application root.
pub type ShellSaveFn = Rc<dyn Fn(ShellChrome, &mut App)>;

/// Static holy-grail shell layout and the live state used by later interactions.
#[allow(dead_code)]
pub struct Shell {
    chrome: ShellChrome,
    tab_model: TabModel,
    left_tween: Option<DimTween>,
    right_tween: Option<DimTween>,
    bottom_tween: Option<DimTween>,
    save: ShellSaveFn,
    theme: OpenCoreTheme,
}

impl Shell {
    pub fn new(chrome: ShellChrome, save: ShellSaveFn, _cx: &mut Context<Self>) -> Self {
        let tab_model = TabModel::from_chrome(&chrome);
        Self {
            chrome,
            tab_model,
            left_tween: None,
            right_tween: None,
            bottom_tween: None,
            save,
            theme: OpenCoreTheme::resolve(crate::shared::theme::ThemeMode::Dark),
        }
    }

    pub fn set_theme(&mut self, theme: OpenCoreTheme) {
        self.theme = theme;
    }

    pub fn left_target(&self) -> f32 {
        Self::left_target_for(&self.chrome)
    }

    pub fn right_target(&self) -> f32 {
        Self::right_target_for(&self.chrome)
    }

    pub fn bottom_target(&self) -> f32 {
        Self::bottom_target_for(&self.chrome)
    }

    pub fn left_target_for(chrome: &ShellChrome) -> f32 {
        if chrome.left_open {
            chrome.left_width
        } else {
            0.0
        }
    }

    pub fn right_target_for(chrome: &ShellChrome) -> f32 {
        if chrome.right_open {
            chrome.right_width
        } else {
            0.0
        }
    }

    pub fn bottom_target_for(chrome: &ShellChrome) -> f32 {
        if chrome.bottom_open {
            chrome.bottom_height
        } else {
            0.0
        }
    }

    pub fn toggle_left(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        let reduced = reduced_motion(cx);
        let from = eval_tween(self.left_tween.as_ref(), self.left_target(), now, reduced);
        self.left_tween = Some(toggle_panel(
            &mut self.chrome.left_open,
            from,
            self.chrome.left_width,
            now,
        ));
        self.schedule_save(cx);
        cx.notify();
    }

    pub fn toggle_right(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        let reduced = reduced_motion(cx);
        let from = eval_tween(self.right_tween.as_ref(), self.right_target(), now, reduced);
        self.right_tween = Some(toggle_panel(
            &mut self.chrome.right_open,
            from,
            self.chrome.right_width,
            now,
        ));
        self.schedule_save(cx);
        cx.notify();
    }

    pub fn toggle_bottom(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        let reduced = reduced_motion(cx);
        let from = eval_tween(
            self.bottom_tween.as_ref(),
            self.bottom_target(),
            now,
            reduced,
        );
        self.bottom_tween = Some(toggle_panel(
            &mut self.chrome.bottom_open,
            from,
            self.chrome.bottom_height,
            now,
        ));
        self.schedule_save(cx);
        cx.notify();
    }

    fn schedule_save(&self, cx: &mut Context<Self>) {
        (self.save)(self.chrome.clone(), cx);
    }

    fn settle_tweens(&mut self, now: Instant, reduced: bool) {
        if reduced {
            self.left_tween = None;
            self.right_tween = None;
            self.bottom_tween = None;
            return;
        }

        if self
            .left_tween
            .as_ref()
            .is_some_and(|tween| tween_finished(tween, now))
        {
            self.left_tween = None;
        }
        if self
            .right_tween
            .as_ref()
            .is_some_and(|tween| tween_finished(tween, now))
        {
            self.right_tween = None;
        }
        if self
            .bottom_tween
            .as_ref()
            .is_some_and(|tween| tween_finished(tween, now))
        {
            self.bottom_tween = None;
        }
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        let reduced = reduced_motion(cx);
        self.settle_tweens(now, reduced);

        let left_width = eval_tween(self.left_tween.as_ref(), self.left_target(), now, reduced);
        let right_width = eval_tween(self.right_tween.as_ref(), self.right_target(), now, reduced);
        let bottom_height = eval_tween(
            self.bottom_tween.as_ref(),
            self.bottom_target(),
            now,
            reduced,
        );
        if self.left_tween.is_some() || self.right_tween.is_some() || self.bottom_tween.is_some() {
            window.request_animation_frame();
        }

        let background = self.theme.surface(BackgroundToken::Primary);
        let panel_background = self.theme.surface(BackgroundToken::Secondary);
        let titlebar_background = self.theme.surface(BackgroundToken::Tertiary);
        let label = self.theme.foreground(ForegroundToken::Muted);

        let left = div()
            .w(px(left_width))
            .h_full()
            .overflow_hidden()
            .flex_shrink_0()
            .child(
                stub_region("LEFT", panel_background, label)
                    .w(px(self.chrome.left_width))
                    .h_full(),
            );
        let right = div()
            .w(px(right_width))
            .h_full()
            .overflow_hidden()
            .flex_shrink_0()
            .child(
                stub_region("RIGHT", panel_background, label)
                    .w(px(self.chrome.right_width))
                    .h_full(),
            );
        let main = stub_region("MAIN", background, label).flex_1().w_full();
        let bottom = div()
            .w_full()
            .h(px(bottom_height))
            .overflow_hidden()
            .flex_shrink_0()
            .child(
                stub_region("BOTTOM", panel_background, label)
                    .w_full()
                    .h(px(self.chrome.bottom_height)),
            );

        let on_left_toggle = cx.listener(|shell, _, _, cx| shell.toggle_left(cx));
        let on_right_toggle = cx.listener(|shell, _, _, cx| shell.toggle_right(cx));
        let on_bottom_toggle = cx.listener(|shell, _, _, cx| shell.toggle_bottom(cx));

        div()
            .relative()
            .size_full()
            .bg(background)
            .child(
                div()
                    .size_full()
                    .flex()
                    .pt(px(TITLEBAR_HEIGHT))
                    .child(left)
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .flex()
                            .flex_col()
                            .child(main)
                            .child(bottom),
                    )
                    .child(right),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(TITLEBAR_HEIGHT))
                    .bg(titlebar_background)
                    .flex()
                    .items_center()
                    .child(
                        Button::new("shell-left-toggle")
                            .ghost()
                            .compact()
                            .label("Left")
                            .on_click(on_left_toggle),
                    )
                    .child(
                        Button::new("shell-right-toggle")
                            .ghost()
                            .compact()
                            .label("Right")
                            .on_click(on_right_toggle),
                    )
                    .child(
                        Button::new("shell-bottom-toggle")
                            .ghost()
                            .compact()
                            .label("Bottom")
                            .on_click(on_bottom_toggle),
                    ),
            )
    }
}

fn toggle_panel(open: &mut bool, from: f32, open_size: f32, started: Instant) -> DimTween {
    *open = !*open;
    DimTween {
        from,
        to: if *open { open_size } else { 0.0 },
        started,
    }
}

/// GPUI does not expose a reduced-motion setting on `Window` or `App` in this revision.
fn reduced_motion(_cx: &App) -> bool {
    false
}

fn stub_region(label: &'static str, background: gpui::Hsla, foreground: gpui::Hsla) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .bg(background)
        .text_color(foreground)
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::{Shell, toggle_panel};
    use crate::app::shell::ShellChrome;
    use std::time::Instant;

    #[test]
    fn left_target_zero_when_closed() {
        let mut chrome = ShellChrome::default();
        chrome.left_open = false;

        assert_eq!(Shell::left_target_for(&chrome), 0.0);
    }

    #[test]
    fn left_target_uses_width_when_open() {
        let mut chrome = ShellChrome::default();
        chrome.left_width = 312.0;

        assert_eq!(Shell::left_target_for(&chrome), 312.0);
    }

    #[test]
    fn right_and_bottom_targets_follow_open_flags() {
        let mut chrome = ShellChrome::default();
        chrome.right_open = true;
        chrome.bottom_open = true;

        assert_eq!(Shell::right_target_for(&chrome), chrome.right_width);
        assert_eq!(Shell::bottom_target_for(&chrome), chrome.bottom_height);

        chrome.right_open = false;
        chrome.bottom_open = false;
        assert_eq!(Shell::right_target_for(&chrome), 0.0);
        assert_eq!(Shell::bottom_target_for(&chrome), 0.0);
    }

    #[test]
    fn toggle_panel_flips_open_flag_and_sets_tween_endpoints() {
        let now = Instant::now();
        let mut open = true;

        let tween = toggle_panel(&mut open, 256.0, 256.0, now);

        assert!(!open);
        assert_eq!(tween.from, 256.0);
        assert_eq!(tween.to, 0.0);
        assert_eq!(tween.started, now);

        let tween = toggle_panel(&mut open, 48.0, 256.0, now);

        assert!(open);
        assert_eq!(tween.from, 48.0);
        assert_eq!(tween.to, 256.0);
    }
}
