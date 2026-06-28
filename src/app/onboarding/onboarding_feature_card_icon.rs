//! Wireframe feature icons for onboarding highlight cards.

use std::time::Instant;

use gpui::{Bounds, Pixels};

use crate::shared::theme::OpenCoreTheme;

use super::onboarding_draw::{Painter, Point2, Rgba, Size2, blend, with_alpha};
use super::onboarding_feature_card_dynamics::accent_pulse;

const DESIGN_SIZE: f32 = 120.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureKind {
    Chat,
    Terminal,
    TextEditor,
    Rust,
}

#[derive(Clone)]
pub struct FeatureCardIcon {
    kind: FeatureKind,
    glow: f32,
    now: Instant,
    started_at: Instant,
    theme: OpenCoreTheme,
}

impl FeatureCardIcon {
    pub fn new(
        kind: FeatureKind,
        glow: f32,
        now: Instant,
        started_at: Instant,
        theme: OpenCoreTheme,
    ) -> Self {
        Self {
            kind,
            glow: glow.clamp(0.0, 1.0),
            now,
            started_at,
            theme,
        }
    }

    pub fn paint(&self, painter: &mut Painter<'_>, bounds: Bounds<Pixels>) {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        let accent = theme_accent_rgba(self.theme);
        let muted = theme_muted_rgba(self.theme);
        let pulse = accent_pulse(self.now, self.started_at, 1.35);
        let emphasis = self.glow.clamp(0.0, 1.0);
        let stroke_color = blend(muted, accent, emphasis);
        let node = blend(with_alpha(muted, 0.85), accent, emphasis);

        let fit = (width.min(height) / DESIGN_SIZE) * 0.84;
        let content = DESIGN_SIZE * fit;
        let mapper = Mapper {
            offset: Point2 {
                x: (width - content) * 0.5,
                y: (height - content) * 0.5,
            },
            fit,
        };
        let line = (1.05 * fit).max(0.7);
        let node_size = (3.0 * fit).max(1.4);

        if emphasis > 0.02 {
            let radius = fit * (34.0 + pulse * 4.0 * emphasis);
            let alpha = 0.04 + emphasis * (0.06 + pulse * 0.04);
            draw_glow(
                painter,
                mapper.map(Point2 { x: 60.0, y: 58.0 }),
                radius,
                accent,
                alpha,
            );
        }

        match self.kind {
            FeatureKind::Chat => draw_chat(painter, &mapper, line, stroke_color, node, node_size),
            FeatureKind::Terminal => draw_terminal(
                painter,
                &mapper,
                line,
                stroke_color,
                node,
                node_size,
                fit,
            ),
            FeatureKind::TextEditor => {
                draw_text_editor(painter, &mapper, line, stroke_color, node, node_size)
            }
            FeatureKind::Rust => draw_rust(painter, &mapper, line, stroke_color, node, node_size),
        }
    }
}

struct Mapper {
    offset: Point2,
    fit: f32,
}

impl Mapper {
    fn map(&self, point: Point2) -> Point2 {
        Point2 {
            x: self.offset.x + point.x * self.fit,
            y: self.offset.y + point.y * self.fit,
        }
    }
}

fn draw_glow(painter: &mut Painter<'_>, center: Point2, radius: f32, accent: Rgba, peak_alpha: f32) {
    for (scale, alpha_scale) in [(1.0, 0.55), (0.72, 0.75), (0.46, 1.0)] {
        let size = radius * scale * 2.0;
        painter.fill_rectangle(
            Point2 {
                x: center.x - size * 0.5,
                y: center.y - size * 0.5,
            },
            Size2 { width: size, height: size },
            with_alpha(accent, peak_alpha * alpha_scale),
        );
    }
}

fn draw_chat(
    painter: &mut Painter<'_>,
    mapper: &Mapper,
    line: f32,
    stroke: Rgba,
    node: Rgba,
    node_size: f32,
) {
    stroke_rect(painter, mapper, Point2 { x: 28.0, y: 28.0 }, 44.0, 36.0, line, stroke);
    stroke_rect(painter, mapper, Point2 { x: 58.0, y: 46.0 }, 44.0, 36.0, line, stroke);
    for point in [
        Point2 { x: 28.0, y: 28.0 },
        Point2 { x: 72.0, y: 28.0 },
        Point2 { x: 72.0, y: 64.0 },
        Point2 { x: 28.0, y: 64.0 },
        Point2 { x: 58.0, y: 46.0 },
        Point2 { x: 102.0, y: 46.0 },
        Point2 { x: 102.0, y: 82.0 },
        Point2 { x: 58.0, y: 82.0 },
    ] {
        fill_node(painter, mapper.map(point), node_size, node);
    }
    stroke_line(
        painter,
        mapper.map(Point2 { x: 72.0, y: 46.0 }),
        mapper.map(Point2 { x: 58.0, y: 46.0 }),
        line,
        stroke,
    );
}

fn draw_terminal(
    painter: &mut Painter<'_>,
    mapper: &Mapper,
    line: f32,
    stroke: Rgba,
    node: Rgba,
    node_size: f32,
    fit: f32,
) {
    stroke_rect(painter, mapper, Point2 { x: 24.0, y: 22.0 }, 72.0, 76.0, line, stroke);
    stroke_line(
        painter,
        mapper.map(Point2 { x: 24.0, y: 36.0 }),
        mapper.map(Point2 { x: 96.0, y: 36.0 }),
        line,
        stroke,
    );
    painter.fill_rectangle(
        mapper.map(Point2 { x: 48.0, y: 47.0 }),
        Size2 {
            width: 6.0 * fit,
            height: 9.0 * fit,
        },
        stroke,
    );
    for (y, width) in [(62.0, 48.0), (72.0, 36.0), (82.0, 54.0)] {
        stroke_line(
            painter,
            mapper.map(Point2 { x: 34.0, y }),
            mapper.map(Point2 { x: 34.0 + width, y }),
            line,
            with_alpha(stroke, 0.72),
        );
    }
    fill_node(
        painter,
        mapper.map(Point2 { x: 24.0, y: 22.0 }),
        node_size,
        node,
    );
}

fn draw_text_editor(
    painter: &mut Painter<'_>,
    mapper: &Mapper,
    line: f32,
    stroke: Rgba,
    node: Rgba,
    node_size: f32,
) {
    stroke_rect(painter, mapper, Point2 { x: 22.0, y: 20.0 }, 76.0, 80.0, line, stroke);
    stroke_line(
        painter,
        mapper.map(Point2 { x: 40.0, y: 20.0 }),
        mapper.map(Point2 { x: 40.0, y: 100.0 }),
        line,
        with_alpha(stroke, 0.55),
    );
    for (y, x0, width) in [
        (36.0, 48.0, 42.0),
        (46.0, 48.0, 32.0),
        (56.0, 48.0, 46.0),
        (66.0, 48.0, 28.0),
        (76.0, 48.0, 38.0),
        (86.0, 48.0, 30.0),
    ] {
        stroke_line(
            painter,
            mapper.map(Point2 { x: x0, y }),
            mapper.map(Point2 { x: x0 + width, y }),
            line,
            stroke,
        );
    }
    fill_node(
        painter,
        mapper.map(Point2 { x: 22.0, y: 20.0 }),
        node_size,
        node,
    );
}

fn draw_rust(
    painter: &mut Painter<'_>,
    mapper: &Mapper,
    line: f32,
    stroke: Rgba,
    node: Rgba,
    node_size: f32,
) {
    let center = mapper.map(Point2 { x: 60.0, y: 60.0 });
    let points = [
        Point2 { x: 60.0, y: 22.0 },
        Point2 { x: 92.0, y: 40.0 },
        Point2 { x: 92.0, y: 80.0 },
        Point2 { x: 60.0, y: 98.0 },
        Point2 { x: 28.0, y: 80.0 },
        Point2 { x: 28.0, y: 40.0 },
    ]
    .map(|point| mapper.map(point));

    for index in 0..6 {
        stroke_line(painter, points[index], points[(index + 1) % 6], line, stroke);
    }
    for point in points {
        fill_node(painter, point, node_size, node);
    }
    fill_node(painter, center, node_size * 1.1, node);
}

fn stroke_rect(
    painter: &mut Painter<'_>,
    mapper: &Mapper,
    origin: Point2,
    width: f32,
    height: f32,
    line: f32,
    stroke: Rgba,
) {
    let origin = mapper.map(origin);
    let width = width * mapper.fit;
    let height = height * mapper.fit;
    painter.stroke_rect(origin, Size2 { width, height }, line, stroke);
}

fn stroke_line(painter: &mut Painter<'_>, from: Point2, to: Point2, line: f32, stroke: Rgba) {
    painter.stroke_line(from, to, line, stroke);
}

fn fill_node(painter: &mut Painter<'_>, center: Point2, size: f32, color: Rgba) {
    painter.fill_rectangle(
        Point2 {
            x: center.x - size * 0.5,
            y: center.y - size * 0.5,
        },
        Size2 { width: size, height: size },
        color,
    );
}

fn theme_accent_rgba(theme: OpenCoreTheme) -> Rgba {
    match theme.mode {
        crate::shared::theme::ThemeMode::Light => Rgba::rgb(0.09, 0.09, 0.09),
        crate::shared::theme::ThemeMode::Dark => Rgba::rgb(0.90, 0.90, 0.90),
    }
}

fn theme_muted_rgba(theme: OpenCoreTheme) -> Rgba {
    match theme.mode {
        crate::shared::theme::ThemeMode::Light => Rgba::rgb(0.64, 0.64, 0.64),
        crate::shared::theme::ThemeMode::Dark => Rgba::rgb(0.45, 0.45, 0.45),
    }
}
