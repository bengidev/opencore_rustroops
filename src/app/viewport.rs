//! Live window viewport dimensions for layout that tracks user resize.

use gpui::Window;

/// Client-area size in pixels, refreshed each render from [`Window::viewport_size`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowViewport {
    pub width: f32,
    pub height: f32,
}

impl WindowViewport {
    pub fn from_window(window: &Window) -> Self {
        let size = window.viewport_size();
        Self {
            width: size.width.as_f32(),
            height: size.height.as_f32(),
        }
    }
}

impl Default for WindowViewport {
    fn default() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_viewport_is_zero() {
        let viewport = WindowViewport::default();
        assert_eq!(viewport.width, 0.0);
        assert_eq!(viewport.height, 0.0);
    }
}
