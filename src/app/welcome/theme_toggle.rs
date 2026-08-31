//! Theme toggle using gpui-component `Button` with Lucide icons (`IconName::Sun` / `IconName::Moon`).

use gpui::IntoElement;
use gpui_component::IconName;
use gpui_component::button::Button;
use gpui_component::Disableable;

use crate::app::gpui_callbacks::WindowAppHandler;
use crate::shared::theme::{OpenCoreTheme, ThemeMode};

pub fn theme_toggle_button(
    theme: OpenCoreTheme,
    on_press: WindowAppHandler,
    enabled: bool,
) -> impl IntoElement {
    let (icon, label) = match theme.mode {
        ThemeMode::Dark => (IconName::Sun, "Light"),
        ThemeMode::Light => (IconName::Moon, "Dark"),
    };
    Button::new("theme-toggle")
        .outline()
        .icon(icon)
        .label(label)
        .disabled(!enabled)
        .on_click(move |_, window, cx| on_press(window, cx))
}
