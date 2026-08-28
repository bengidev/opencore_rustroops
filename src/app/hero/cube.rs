//! Wireframe cube hero — GPUI canvas port of iOS `OnboardingCubeView`.

use std::f32::consts::PI;
use std::time::Instant;

use gpui::{
    Bounds, Hsla, IntoElement, ParentElement, PathBuilder, Pixels, Point, Styled, Window, canvas,
    div, point, px,
};

const BASE_YAW: f32 = 0.6;
const BASE_PITCH: f32 = 0.52;
const HEADER_YAW: f32 = PI / 4.0;
const HEADER_PITCH: f32 = 0.615_479_7; // atan(1/sqrt(2))

const CONSTRUCTION_DURATION: f32 = 0.75;
const VERTEX_PHASE_END: f32 = 0.35;
const EDGE_PHASE_START: f32 = 0.2;
const MORPH_OVERLAP_START: f32 = 0.72;
const MORPH_SEGMENT_OVERLAP: f32 = 0.88;
const MORPH_SEGMENT_DURATION_MIN: f32 = 0.30;
const MORPH_SEGMENT_DURATION_MAX: f32 = 0.58;
const MORPH_SEGMENT_IMMEDIATE_DURATION_MIN: f32 = 0.28;
const MORPH_SEGMENT_IMMEDIATE_DURATION_MAX: f32 = 0.50;

const VERTICES: [(f32, f32, f32); 8] = [
    (-1.0, -1.0, -1.0),
    (1.0, -1.0, -1.0),
    (1.0, 1.0, -1.0),
    (-1.0, 1.0, -1.0),
    (-1.0, -1.0, 1.0),
    (1.0, -1.0, 1.0),
    (1.0, 1.0, 1.0),
    (-1.0, 1.0, 1.0),
];

const EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 0),
    (4, 5),
    (5, 6),
    (6, 7),
    (7, 4),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

/// Back-bottom-left hidden edges (dashed), matching iOS.
const DASHED_EDGES: [usize; 3] = [6, 7, 11];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CubePhase {
    #[default]
    Construction,
    Morph,
}

#[derive(Clone, Copy, Debug, Default)]
struct Orientation {
    yaw: f32,
    pitch: f32,
    roll: f32,
}

pub struct CubeHeroState {
    last_tick: Instant,
    construction: f32,
    construction_started: Option<Instant>,
    phase: CubePhase,
    morph_from: Orientation,
    morph_to: Orientation,
    morph_segment_start: Option<Instant>,
    morph_segment_duration: f32,
    morph_rng: MorphRng,
    recent_targets: [Option<Orientation>; RECENT_TARGET_HISTORY],
    recent_target_index: usize,
    rotation_from: Option<Orientation>,
    last_rotation_progress: f32,
    orientation: Orientation,
}

const RECENT_TARGET_HISTORY: usize = 4;
const MORPH_YAW_RANGE: (f32, f32) = (0.12, 1.38);
const MORPH_PITCH_RANGE: (f32, f32) = (0.10, 0.82);
const MORPH_ROLL_RANGE: (f32, f32) = (-0.34, 0.34);

impl CubeHeroState {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            last_tick: now,
            construction: 0.0,
            construction_started: Some(now),
            phase: CubePhase::Construction,
            morph_from: base_orientation(),
            morph_to: base_orientation(),
            morph_segment_start: None,
            morph_segment_duration: MORPH_SEGMENT_DURATION_MAX,
            morph_rng: MorphRng::new(),
            recent_targets: [None; RECENT_TARGET_HISTORY],
            recent_target_index: 0,
            rotation_from: None,
            last_rotation_progress: 0.0,
            orientation: base_orientation(),
        }
    }

    pub fn docked() -> Self {
        Self {
            last_tick: Instant::now(),
            construction: 1.0,
            construction_started: None,
            phase: CubePhase::Morph,
            morph_from: header_orientation(),
            morph_to: header_orientation(),
            morph_segment_start: None,
            morph_segment_duration: MORPH_SEGMENT_DURATION_MAX,
            morph_rng: MorphRng::new(),
            recent_targets: [None; RECENT_TARGET_HISTORY],
            recent_target_index: 0,
            rotation_from: None,
            last_rotation_progress: 1.0,
            orientation: header_orientation(),
        }
    }

    pub fn tick(&mut self, now: Instant, rotation_progress: f32) {
        let _ = now
            .saturating_duration_since(self.last_tick)
            .as_secs_f32()
            .clamp(0.0, 0.1);
        self.last_tick = now;

        if self.last_rotation_progress <= 0.0 && rotation_progress > 0.0 {
            self.rotation_from = Some(self.free_orientation(now));
            self.phase = CubePhase::Morph;
        }
        self.last_rotation_progress = rotation_progress;

        if let Some(start) = self.construction_started {
            self.construction =
                ((now.saturating_duration_since(start).as_secs_f32()) / CONSTRUCTION_DURATION).min(1.0);
        }

        if rotation_progress <= 0.0 {
            if self.construction >= MORPH_OVERLAP_START && self.phase == CubePhase::Construction {
                self.phase = CubePhase::Morph;
                self.begin_morph_segment(now, true);
            } else if self.phase == CubePhase::Morph {
                self.advance_morph_if_needed(now);
            }
        }

        self.orientation = self.compute_orientation(now, rotation_progress);
    }

    pub fn construction(&self) -> f32 {
        self.construction
    }

    fn begin_morph_segment(&mut self, now: Instant, immediate: bool) {
        let current = self.free_orientation(now);
        self.morph_from = current;
        self.morph_to = self.pick_morph_target(current, immediate);
        self.remember_target(self.morph_to);
        self.morph_segment_duration = if immediate {
            self.morph_rng.range(
                MORPH_SEGMENT_IMMEDIATE_DURATION_MIN,
                MORPH_SEGMENT_IMMEDIATE_DURATION_MAX,
            )
        } else {
            self.morph_rng.range(MORPH_SEGMENT_DURATION_MIN, MORPH_SEGMENT_DURATION_MAX)
        };
        self.morph_segment_start = Some(now);
    }

    fn pick_morph_target(&mut self, from: Orientation, immediate: bool) -> Orientation {
        let minimum_delta = if immediate { 0.16 } else { 0.07 };
        let history_gap = if immediate { 0.14 } else { 0.10 };
        let big_swing = self.morph_rng.chance(0.22);
        let attempts = if big_swing { 20 } else { 14 };

        for _ in 0..attempts {
            let candidate = self.morph_rng.orientation();
            let delta = orientation_distance(candidate, from);
            if big_swing && delta < 0.28 {
                continue;
            }
            if !big_swing && delta < minimum_delta {
                continue;
            }
            if self.too_close_to_recent(candidate, history_gap) {
                continue;
            }
            return candidate;
        }

        let sign = if self.morph_rng.chance(0.5) { 1.0 } else { -1.0 };
        Orientation {
            yaw: (from.yaw + sign * 0.34).clamp(MORPH_YAW_RANGE.0, MORPH_YAW_RANGE.1),
            pitch: (from.pitch - sign * 0.18).clamp(MORPH_PITCH_RANGE.0, MORPH_PITCH_RANGE.1),
            roll: (from.roll + sign * 0.22).clamp(MORPH_ROLL_RANGE.0, MORPH_ROLL_RANGE.1),
        }
    }

    fn remember_target(&mut self, target: Orientation) {
        self.recent_targets[self.recent_target_index] = Some(target);
        self.recent_target_index = (self.recent_target_index + 1) % RECENT_TARGET_HISTORY;
    }

    fn too_close_to_recent(&self, candidate: Orientation, minimum_gap: f32) -> bool {
        self.recent_targets.iter().any(|recent| {
            recent.is_some_and(|stored| orientation_distance(candidate, stored) < minimum_gap)
        })
    }

    fn advance_morph_if_needed(&mut self, now: Instant) {
        let Some(start) = self.morph_segment_start else {
            self.begin_morph_segment(now, false);
            return;
        };
        let elapsed = now.saturating_duration_since(start).as_secs_f32();
        if elapsed >= self.morph_segment_duration * MORPH_SEGMENT_OVERLAP {
            self.begin_morph_segment(now, false);
        }
    }

    fn free_orientation(&self, now: Instant) -> Orientation {
        if self.phase != CubePhase::Morph {
            return base_orientation();
        }
        let Some(start) = self.morph_segment_start else {
            return base_orientation();
        };
        let t = ease_in_out(
            (now.saturating_duration_since(start).as_secs_f32() / self.morph_segment_duration).clamp(0.0, 1.0),
        );
        Orientation {
            yaw: lerp(self.morph_from.yaw, self.morph_to.yaw, t),
            pitch: lerp(self.morph_from.pitch, self.morph_to.pitch, t),
            roll: lerp(self.morph_from.roll, self.morph_to.roll, t),
        }
    }

    fn compute_orientation(&self, now: Instant, rotation_progress: f32) -> Orientation {
        if rotation_progress > 0.0 {
            let from = self.rotation_from.unwrap_or_else(|| self.free_orientation(now));
            let t = rotation_progress.clamp(0.0, 1.0);
            return Orientation {
                yaw: lerp(from.yaw, HEADER_YAW, t),
                pitch: lerp(from.pitch, HEADER_PITCH, t),
                roll: lerp(from.roll, 0.0, t),
            };
        }
        self.free_orientation(now)
    }
}

impl Default for CubeHeroState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn cube_hero_canvas(
    state: &CubeHeroState,
    ink: Hsla,
    rotation_progress: f32,
) -> impl IntoElement {
    let construction = state.construction();
    let orientation = state.orientation;
    div().size_full().child(
        canvas(
            move |bounds, _window, _cx| (bounds, construction, orientation, ink, rotation_progress),
            move |_bounds, (bounds, construction, orientation, ink, rotation_progress), window, _cx| {
                paint_wireframe_cube(
                    window,
                    bounds,
                    construction,
                    orientation,
                    ink,
                    rotation_progress,
                );
            },
        )
        .size_full(),
    )
}

fn paint_wireframe_cube(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    construction: f32,
    orientation: Orientation,
    ink: Hsla,
    rotation_progress: f32,
) {
    let center = bounds.center();
    let side = bounds.size.width.min(bounds.size.height).as_f32();
    let compact = side <= 28.0;
    let docked = side <= 18.0;
    let half = side * if docked { 0.42 } else if compact { 0.38 } else { 0.34 };
    let projected = project_all(center, half, orientation);
    let construction_active = construction < 1.0 && rotation_progress <= 0.0;
    let stroke_width = if docked { 1.0 } else if compact { 1.25 } else { 2.0 };
    let vertex_radius = if docked { 1.0 } else if compact { 1.5 } else { 3.5 };

    for (index, &(start, end)) in EDGES.iter().enumerate() {
        let stroke_end = if construction_active {
            edge_stroke_end(index, construction)
        } else {
            1.0
        };
        if stroke_end <= 0.0 {
            continue;
        }
        paint_edge(
            window,
            projected[start],
            projected[end],
            stroke_end,
            ink,
            DASHED_EDGES.contains(&index),
            stroke_width,
            compact,
            docked,
        );
    }

    for (index, vertex) in projected.into_iter().enumerate() {
        if construction_active {
            let opacity = vertex_opacity(index, construction);
            if opacity <= 0.0 {
                continue;
            }
            let scale = 0.85 + 0.15 * opacity;
            paint_vertex_dot(window, vertex, ink.opacity(opacity), scale * vertex_radius);
        } else {
            paint_vertex_dot(window, vertex, ink, vertex_radius);
        }
    }
}

fn paint_edge(
    window: &mut Window,
    start: Point<Pixels>,
    end: Point<Pixels>,
    stroke_end: f32,
    ink: Hsla,
    dashed: bool,
    stroke_width: f32,
    compact: bool,
    docked: bool,
) {
    let trimmed_end = lerp_point(start, end, stroke_end.min(1.0));
    let mut builder = PathBuilder::stroke(px(stroke_width));
    if dashed {
        let dash = if docked {
            px(2.0)
        } else if compact {
            px(2.5)
        } else {
            px(4.0)
        };
        let gap = if docked {
            px(1.5)
        } else if compact {
            px(2.0)
        } else {
            px(3.0)
        };
        builder = builder.dash_array(&[dash, gap]);
    }
    builder.move_to(start);
    builder.line_to(trimmed_end);
    if let Ok(path) = builder.build() {
        window.paint_path(path, ink);
    }
}

fn project_all(center: Point<Pixels>, half: f32, orientation: Orientation) -> [Point<Pixels>; 8] {
    let mut out = [point(px(0.0), px(0.0)); 8];
    for (index, vertex) in VERTICES.iter().enumerate() {
        out[index] = project_vertex(*vertex, center, half, orientation);
    }
    out
}

fn project_vertex(
    (x, y, z): (f32, f32, f32),
    center: Point<Pixels>,
    half: f32,
    Orientation { yaw, pitch, roll }: Orientation,
) -> Point<Pixels> {
    let x1 = x * yaw.cos() + z * yaw.sin();
    let z1 = -x * yaw.sin() + z * yaw.cos();
    let y2 = y * pitch.cos() - z1 * pitch.sin();
    let x3 = x1 * roll.cos() - y2 * roll.sin();
    let y3 = x1 * roll.sin() + y2 * roll.cos();
    point(center.x + px(x3 * half), center.y + px(y3 * half))
}

fn vertex_opacity(index: usize, construction: f32) -> f32 {
    let vertex_progress = (construction / VERTEX_PHASE_END).min(1.0);
    let delay = index as f32 * 0.025;
    let normalized_delay = delay / VERTEX_PHASE_END;
    let linear = ((vertex_progress - normalized_delay) / (1.0 - normalized_delay)).clamp(0.0, 1.0);
    ease_out(linear)
}

fn edge_stroke_end(index: usize, construction: f32) -> f32 {
    let edge_span = (1.0 - EDGE_PHASE_START).max(0.001);
    let edge_progress = ((construction - EDGE_PHASE_START) / edge_span).clamp(0.0, 1.0);
    let linear = if DASHED_EDGES.contains(&index) {
        ((edge_progress - 0.35) / 0.65).clamp(0.0, 1.0)
    } else {
        let delay = index as f32 * 0.05;
        ((edge_progress - delay) / (1.0 - delay)).clamp(0.0, 1.0)
    };
    ease_out(linear)
}

fn paint_vertex_dot(window: &mut Window, center: Point<Pixels>, ink: Hsla, radius: f32) {
    let cx = center.x.as_f32();
    let cy = center.y.as_f32();
    let mut builder = PathBuilder::fill();
    builder.move_to(point(px(cx), px(cy - radius)));
    builder.arc_to(
        point(px(radius), px(radius)),
        px(0.0),
        false,
        true,
        point(px(cx + radius), px(cy)),
    );
    builder.arc_to(
        point(px(radius), px(radius)),
        px(0.0),
        false,
        true,
        point(px(cx), px(cy + radius)),
    );
    builder.arc_to(
        point(px(radius), px(radius)),
        px(0.0),
        false,
        true,
        point(px(cx - radius), px(cy)),
    );
    builder.arc_to(
        point(px(radius), px(radius)),
        px(0.0),
        false,
        true,
        point(px(cx), px(cy - radius)),
    );
    builder.close();
    if let Ok(path) = builder.build() {
        window.paint_path(path, ink);
    }
}

fn base_orientation() -> Orientation {
    Orientation {
        yaw: BASE_YAW,
        pitch: BASE_PITCH,
        roll: 0.0,
    }
}

fn orientation_distance(a: Orientation, b: Orientation) -> f32 {
    (a.yaw - b.yaw).abs() + (a.pitch - b.pitch).abs() + (a.roll - b.roll).abs()
}

struct MorphRng {
    state: u32,
}

impl MorphRng {
    fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.subsec_nanos())
            .unwrap_or(0x_a5a5_5a5a);
        Self {
            state: seed.max(1),
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        self.state
    }

    fn unit(&mut self) -> f32 {
        (self.next_u32() as f32) / u32::MAX as f32
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.unit()
    }

    fn chance(&mut self, probability: f32) -> bool {
        self.unit() < probability.clamp(0.0, 1.0)
    }

    fn orientation(&mut self) -> Orientation {
        Orientation {
            yaw: self.range(MORPH_YAW_RANGE.0, MORPH_YAW_RANGE.1),
            pitch: self.range(MORPH_PITCH_RANGE.0, MORPH_PITCH_RANGE.1),
            roll: self.range(MORPH_ROLL_RANGE.0, MORPH_ROLL_RANGE.1),
        }
    }
}

fn header_orientation() -> Orientation {
    Orientation {
        yaw: HEADER_YAW,
        pitch: HEADER_PITCH,
        roll: 0.0,
    }
}

fn ease_out(t: f32) -> f32 {
    let clamped = t.clamp(0.0, 1.0);
    1.0 - (1.0 - clamped).powi(3)
}

fn ease_in_out(t: f32) -> f32 {
    let clamped = t.clamp(0.0, 1.0);
    if clamped < 0.5 {
        4.0 * clamped * clamped * clamped
    } else {
        1.0 - (-2.0 * clamped + 2.0).powi(3) / 2.0
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_point(start: Point<Pixels>, end: Point<Pixels>, t: f32) -> Point<Pixels> {
    point(
        start.x + (end.x - start.x) * t,
        start.y + (end.y - start.y) * t,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_reaches_one() {
        let mut state = CubeHeroState::new();
        let start = Instant::now();
        for step in 0..80 {
            state.tick(start + std::time::Duration::from_millis(step * 16), 0.0);
        }
        assert!((state.construction() - 1.0).abs() < 0.05);
    }

    #[test]
    fn morph_starts_before_construction_finishes() {
        let mut state = CubeHeroState::new();
        let start = Instant::now();
        let mut saw_morph = false;
        for step in 0..80 {
            state.tick(start + std::time::Duration::from_millis(step * 16), 0.0);
            if state.phase == CubePhase::Morph {
                saw_morph = true;
                assert!(state.construction() < 1.0);
                break;
            }
        }
        assert!(saw_morph);
    }

    #[test]
    fn header_orientation_at_full_rotation() {
        let mut state = CubeHeroState::new();
        let now = Instant::now();
        state.tick(now, 1.0);
        assert!((state.orientation.yaw - HEADER_YAW).abs() < 1e-3);
        assert!((state.orientation.pitch - HEADER_PITCH).abs() < 1e-3);
    }

    #[test]
    fn morph_targets_stay_varied_across_segments() {
        let mut state = CubeHeroState::new();
        let start = Instant::now();
        for step in 0..240 {
            state.tick(start + std::time::Duration::from_millis(step * 16), 0.0);
        }

        let mut unique = Vec::new();
        for recent in state.recent_targets {
            if let Some(target) = recent {
                let rounded = (
                    (target.yaw * 10.0).round() as i32,
                    (target.pitch * 10.0).round() as i32,
                    (target.roll * 10.0).round() as i32,
                );
                if !unique.contains(&rounded) {
                    unique.push(rounded);
                }
            }
        }
        assert!(unique.len() >= 3, "expected varied morph targets, got {unique:?}");
    }
}
