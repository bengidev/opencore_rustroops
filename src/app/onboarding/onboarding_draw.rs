//! GPUI canvas drawing helpers ported from the iced onboarding renderer.

use gpui::{point, px, rgb, Bounds, Hsla, Window};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn with_alpha(self, alpha: f32) -> Self {
        Self {
            a: alpha.clamp(0.0, 1.0),
            ..self
        }
    }

    pub fn to_hsla(self) -> Hsla {
        let r = (self.r.clamp(0.0, 1.0) * 255.0) as u32;
        let g = (self.g.clamp(0.0, 1.0) * 255.0) as u32;
        let b = (self.b.clamp(0.0, 1.0) * 255.0) as u32;
        rgb((r << 16) | (g << 8) | b).alpha(self.a).into()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Point2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Size2 {
    pub width: f32,
    pub height: f32,
}

pub struct Painter<'a> {
    window: &'a mut Window,
}

impl<'a> Painter<'a> {
    pub fn new(window: &'a mut Window) -> Self {
        Self { window }
    }

    pub fn fill_rectangle(&mut self, origin: Point2, size: Size2, color: Rgba) {
        self.fill_hsla(origin, size, color.to_hsla());
    }

    pub fn fill_hsla(&mut self, origin: Point2, size: Size2, color: Hsla) {
        self.window.paint_quad(gpui::fill(
            Bounds::new(
                point(px(origin.x), px(origin.y)),
                gpui::size(px(size.width), px(size.height)),
            ),
            color,
        ));
    }

    pub fn stroke_line(&mut self, from: Point2, to: Point2, width: f32, color: Rgba) {
        use gpui::PathBuilder;
        let mut builder = PathBuilder::stroke(px(width));
        builder.move_to(point(px(from.x), px(from.y)));
        builder.line_to(point(px(to.x), px(to.y)));
        if let Ok(path) = builder.build() {
            self.window.paint_path(path, color.to_hsla());
        }
    }

    pub fn stroke_rect(&mut self, origin: Point2, size: Size2, width: f32, color: Rgba) {
        let x = origin.x;
        let y = origin.y;
        let w = size.width;
        let h = size.height;
        self.stroke_line(Point2 { x, y }, Point2 { x: x + w, y }, width, color);
        self.stroke_line(Point2 { x: x + w, y }, Point2 { x: x + w, y: y + h }, width, color);
        self.stroke_line(Point2 { x: x + w, y: y + h }, Point2 { x, y: y + h }, width, color);
        self.stroke_line(Point2 { x, y: y + h }, Point2 { x, y }, width, color);
    }

    pub fn fill_circle(&mut self, center: Point2, radius: f32, color: Rgba) {
        self.paint_circle(center, radius, color.to_hsla(), true, 0.0);
    }

    pub fn fill_circle_hsla(&mut self, center: Point2, radius: f32, color: Hsla) {
        self.paint_circle_hsla(center, radius, color, true, 0.0);
    }

    pub fn stroke_circle(&mut self, center: Point2, radius: f32, width: f32, color: Rgba) {
        self.paint_circle(center, radius, color.to_hsla(), false, width);
    }

    fn paint_circle(
        &mut self,
        center: Point2,
        radius: f32,
        color: Hsla,
        fill: bool,
        stroke_width: f32,
    ) {
        self.paint_circle_hsla(center, radius, color, fill, stroke_width);
    }

    fn paint_circle_hsla(
        &mut self,
        center: Point2,
        radius: f32,
        color: Hsla,
        fill: bool,
        stroke_width: f32,
    ) {
        use gpui::PathBuilder;

        let cx = px(center.x);
        let cy = px(center.y);
        let r = px(radius);
        let handle = r * 0.552_284_8;
        let mut builder = if fill {
            PathBuilder::fill()
        } else {
            PathBuilder::stroke(px(stroke_width))
        };

        builder.move_to(point(cx + r, cy));
        builder.cubic_bezier_to(
            point(cx + r, cy + handle),
            point(cx + handle, cy + r),
            point(cx, cy + r),
        );
        builder.cubic_bezier_to(
            point(cx - handle, cy + r),
            point(cx - r, cy + handle),
            point(cx - r, cy),
        );
        builder.cubic_bezier_to(
            point(cx - r, cy - handle),
            point(cx - handle, cy - r),
            point(cx, cy - r),
        );
        builder.cubic_bezier_to(
            point(cx + handle, cy - r),
            point(cx + r, cy - handle),
            point(cx + r, cy),
        );

        if let Ok(path) = builder.build() {
            self.window.paint_path(path, color);
        }
    }
}

pub fn with_alpha(color: Rgba, alpha: f32) -> Rgba {
    color.with_alpha(alpha)
}

pub fn blend(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    Rgba {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}
