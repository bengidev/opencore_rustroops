use std::rc::Rc;

use gpui::{App, Context, IntoElement, ParentElement, Render, Styled, Window, div, px};

use crate::shared::theme::{BackgroundToken, ForegroundToken, OpenCoreTheme};

use super::{DimTween, ShellChrome, TITLEBAR_HEIGHT, TabModel};

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
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let background = self.theme.surface(BackgroundToken::Primary);
        let panel_background = self.theme.surface(BackgroundToken::Secondary);
        let titlebar_background = self.theme.surface(BackgroundToken::Tertiary);
        let label = self.theme.foreground(ForegroundToken::Muted);

        let left = stub_region("LEFT", panel_background, label)
            .w(px(self.left_target()))
            .h_full()
            .flex_shrink_0();
        let right = stub_region("RIGHT", panel_background, label)
            .w(px(self.right_target()))
            .h_full()
            .flex_shrink_0();
        let main = stub_region("MAIN", background, label).flex_1().w_full();
        let bottom = stub_region("BOTTOM", panel_background, label)
            .w_full()
            .h(px(self.bottom_target()))
            .flex_shrink_0();

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
                    .bg(titlebar_background),
            )
    }
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
    use super::Shell;
    use crate::app::shell::ShellChrome;

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
}
