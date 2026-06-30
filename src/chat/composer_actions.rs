//! Callback surface for composer toolbar controls without importing [`ChatView`].

use gpui::{ClickEvent, Context, Window};

use crate::api::SpeedMode;

/// Host entity that handles composer footer interactions.
pub trait ComposerActions: Sized + 'static {
    fn set_speed_mode(&mut self, mode: SpeedMode, cx: &mut Context<Self>);
    fn set_reasoning_effort(&mut self, effort: &str, cx: &mut Context<Self>);
    fn on_send_clicked(&mut self, event: &ClickEvent, window: &mut Window, cx: &mut Context<Self>);
    fn on_stop_clicked(&mut self, event: &ClickEvent, window: &mut Window, cx: &mut Context<Self>);
}
