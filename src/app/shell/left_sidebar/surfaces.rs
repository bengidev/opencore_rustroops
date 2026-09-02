//! Row surface colors for sidebar atom rows.

use gpui::Hsla;

use crate::shared::theme::{
    BackgroundToken, ForegroundToken, OpenCoreTheme, SUCCESS_GREEN, WARNING_AMBER,
};

use super::demo_data::AtomStatus;

pub fn row_hover_bg(theme: &OpenCoreTheme) -> Hsla {
    theme.surface(BackgroundToken::Secondary).alpha(0.55)
}

pub fn row_active_bg(theme: &OpenCoreTheme) -> Hsla {
    theme.surface(BackgroundToken::Tertiary)
}

pub fn row_selected_bg(theme: &OpenCoreTheme) -> Hsla {
    theme.surface(BackgroundToken::Secondary)
}

pub fn drop_line_color(theme: &OpenCoreTheme) -> Hsla {
    theme.foreground(ForegroundToken::Primary)
}

pub fn draft_bg(_theme: &OpenCoreTheme) -> Hsla {
    let color: Hsla = gpui::rgb(WARNING_AMBER).into();
    color.alpha(0.06)
}

pub fn draft_bg_hover(_theme: &OpenCoreTheme) -> Hsla {
    let color: Hsla = gpui::rgb(WARNING_AMBER).into();
    color.alpha(0.12)
}

pub fn status_color(status: AtomStatus, theme: &OpenCoreTheme, dimmed: bool) -> Hsla {
    let hsla = match status {
        AtomStatus::Working | AtomStatus::Monitoring => gpui::rgb(0x0E_A5_E9).into(),
        AtomStatus::Approval | AtomStatus::Woke => gpui::rgb(WARNING_AMBER).into(),
        AtomStatus::Input => gpui::rgb(0x63_66_F1).into(),
        AtomStatus::Failed => theme.foreground(ForegroundToken::Accent),
        AtomStatus::Ready => theme.foreground(ForegroundToken::Muted),
    };
    if dimmed && matches!(status, AtomStatus::Working | AtomStatus::Monitoring) {
        hsla.alpha(0.75)
    } else {
        hsla
    }
}

pub fn pr_open_color() -> Hsla {
    gpui::rgb(SUCCESS_GREEN).into()
}

pub fn project_favicon_color(hue: u32) -> Hsla {
    gpui::rgb(hue).into()
}
