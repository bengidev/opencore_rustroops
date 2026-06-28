//! Animated scene backdrop — subtle dot grid.

use std::time::Instant;

use gpui::{Bounds, Hsla, Pixels};

use crate::shared::theme::{ForegroundToken, OpenCoreTheme};

use super::onboarding_draw::{Painter, Point2, Size2};

#[derive(Debug, Clone, Copy)]
pub struct SceneBackdrop {
    theme: OpenCoreTheme,
    started_at: Instant,
    now: Instant,
}

impl SceneBackdrop {
    pub fn new(theme: OpenCoreTheme, started_at: Instant, now: Instant) -> Self {
        Self {
            theme,
            started_at,
            now,
        }
    }

    fn elapsed(&self) -> f32 {
        self.now
            .saturating_duration_since(self.started_at)
            .as_secs_f32()
    }

    pub fn paint(&self, painter: &mut Painter<'_>, bounds: Bounds<Pixels>) {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        let origin_x: f32 = bounds.origin.x.into();
        let origin_y: f32 = bounds.origin.y.into();
        let dot_color = self.theme.foreground(ForegroundToken::Muted).alpha(0.12);
        draw_dot_grid(
            painter,
            Size2 { width, height },
            origin_x,
            origin_y,
            dot_color,
            self.elapsed(),
        );
    }
}

fn draw_dot_grid(
    painter: &mut Painter<'_>,
    size: Size2,
    origin_x: f32,
    origin_y: f32,
    color: Hsla,
    t: f32,
) {
    let spacing = 28.0;
    let drift = (t * 0.08).sin() * 2.0;
    let cols = (size.width / spacing).ceil() as i32 + 1;
    let rows = (size.height / spacing).ceil() as i32 + 1;

    for row in 0..rows {
        for col in 0..cols {
            let x = origin_x + col as f32 * spacing + drift;
            let y = origin_y + row as f32 * spacing - drift * 0.5;
            let edge = edge_fade(x - origin_x, y - origin_y, size);
            let alpha = color.a * edge;
            if alpha < 0.01 {
                continue;
            }
            let dot = 1.2;
            painter.fill_hsla(
                Point2 {
                    x: x - dot * 0.5,
                    y: y - dot * 0.5,
                },
                Size2 {
                    width: dot,
                    height: dot,
                },
                color.alpha(alpha),
            );
        }
    }
}

fn edge_fade(x: f32, y: f32, size: Size2) -> f32 {
    let nx = (x / size.width - 0.5).abs() * 2.0;
    let ny = (y / size.height - 0.5).abs() * 2.0;
    let edge = nx.max(ny);
    (1.0 - (edge - 0.55).max(0.0) * 2.2).clamp(0.0, 1.0)
}
