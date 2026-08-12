# Particle Diorama Hero Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship an original onboarding hero — edge-only cube cage with a light mist of contained “breathing” particles, soft underglow, soft wall press, and subtle hover/focus bias — behind a swap flag, without deleting the ASCII galaxy.

**Architecture:** Headless `DioramaSim` + `cage` projection math are unit-tested without GPUI. A `particle_diorama::view` paints via GPUI `canvas` (`paint_quad` + `PathBuilder::stroke`). `OnboardingUiState` owns the sim and ticks it on the existing `request_animation_frame` loop. `onboarding_view` swaps ASCII vs diorama with `USE_PARTICLE_DIORAMA`.

**Tech Stack:** Rust 2024, GPUI (`canvas`, `PathBuilder`, `paint_quad`), existing `OpenCoreTheme` / `ForegroundToken` / `ThemeRgba`, `cargo test`.

## Global Constraints

- Do **not** copy either reference’s silhouette, palette, or particle look.
- Faces stay **invisible** (edge-only cage).
- Density = **light mist** — `PARTICLE_COUNT = 640`.
- Colors from **theme tokens** only (no hardcoded reference orange).
- Keep `ascii_galaxy.rs` in tree; swap via `USE_PARTICLE_DIORAMA`.
- No runtime shader path in v1.
- No network/IO in the hero.
- Prefer TDD: failing test → implement → pass → commit per task.

## File map

| File | Responsibility |
|------|----------------|
| `src/app/onboarding/particle_diorama/mod.rs` | Module exports + shared constants |
| `src/app/onboarding/particle_diorama/cage.rs` | Unit cube edges, 3D→2D projection, underglow ellipse, back/front edge split helper |
| `src/app/onboarding/particle_diorama/sim.rs` | Particle pool, curl advection, containment, soft press, hover easing |
| `src/app/onboarding/particle_diorama/view.rs` | GPUI `canvas` painter + theme color mapping |
| `src/app/onboarding/mod.rs` | `mod particle_diorama;` |
| `src/app/onboarding/onboarding_ui_state.rs` | Own `DioramaSim`, tick it, hover setter, snapshot for paint |
| `src/app/onboarding/onboarding_view.rs` | `USE_PARTICLE_DIORAMA` swap in `hero_block`; hover callbacks |
| `src/app/app_desktop.rs` | Wire `on_hero_hover` on `OnboardingCallbacks` |

ASCII modules stay unchanged except that the view may not call `last_frame()` when the flag is true.

---

### Task 1: Cage projection math

**Files:**
- Create: `src/app/onboarding/particle_diorama/mod.rs`
- Create: `src/app/onboarding/particle_diorama/cage.rs`
- Modify: `src/app/onboarding/mod.rs`

**Interfaces:**
- Consumes: nothing from later tasks
- Produces:
  - `pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }`
  - `pub struct Vec2 { pub x: f32, pub y: f32 }`
  - `pub struct Edge3 { pub a: Vec3, pub b: Vec3 }`
  - `pub struct ProjectedEdge { pub a: Vec2, pub b: Vec2, pub depth: f32 }`
  - `pub fn unit_cube_edges() -> [Edge3; 12]`
  - `pub fn project_point(p: Vec3, cam: &Camera) -> Vec2`
  - `pub struct Camera { pub yaw: f32, pub pitch: f32, pub scale: f32, pub origin: Vec2 }`
  - `pub fn default_camera(origin: Vec2, scale: f32) -> Camera`
  - `pub fn project_edges(cam: &Camera) -> Vec<ProjectedEdge>`
  - `pub fn split_edges_back_front(edges: &[ProjectedEdge]) -> (Vec<ProjectedEdge>, Vec<ProjectedEdge>)`
  - `pub fn underglow_ellipse(cam: &Camera) -> (Vec2, f32, f32)` — center + radii in screen space for the cube’s bottom face footprint

- [ ] **Step 1: Wire empty module**

Add to `src/app/onboarding/mod.rs` after `mod ascii_galaxy;`:

```rust
mod particle_diorama;
```

Create `src/app/onboarding/particle_diorama/mod.rs`:

```rust
//! Edge-only cube cage + contained particle mist (onboarding diorama hero).

mod cage;
mod sim;
mod view;

pub use cage::{
    Camera, Edge3, ProjectedEdge, Vec2, Vec3, default_camera, project_edges, project_point,
    split_edges_back_front, underglow_ellipse, unit_cube_edges,
};
pub use sim::{DioramaSim, PARTICLE_COUNT, Particle};
pub use view::diorama_canvas;
```

Create stub `sim.rs` and `view.rs` so the crate compiles after Task 1 only needs `cage` — for Task 1, temporarily comment `mod sim; mod view;` and their `pub use` lines, **or** add minimal stubs:

`sim.rs` stub:

```rust
pub const PARTICLE_COUNT: usize = 640;
pub struct Particle;
pub struct DioramaSim;
```

`view.rs` stub:

```rust
use gpui::IntoElement;
use crate::shared::theme::OpenCoreTheme;
use super::sim::DioramaSim;

pub fn diorama_canvas(_theme: OpenCoreTheme, _sim: &DioramaSim) -> impl IntoElement {
    gpui::div()
}
```

- [ ] **Step 2: Write the failing cage tests**

Append to `cage.rs` (file may only contain tests + unimplemented items first):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_cube_has_twelve_edges() {
        assert_eq!(unit_cube_edges().len(), 12);
    }

    #[test]
    fn projected_edge_endpoints_are_finite() {
        let cam = default_camera(Vec2 { x: 160.0, y: 180.0 }, 110.0);
        for edge in project_edges(&cam) {
            assert!(edge.a.x.is_finite() && edge.a.y.is_finite());
            assert!(edge.b.x.is_finite() && edge.b.y.is_finite());
            assert!(edge.depth.is_finite());
        }
    }

    #[test]
    fn underglow_ellipse_is_finite_and_positive() {
        let cam = default_camera(Vec2 { x: 160.0, y: 180.0 }, 110.0);
        let (c, rx, ry) = underglow_ellipse(&cam);
        assert!(c.x.is_finite() && c.y.is_finite());
        assert!(rx > 1.0 && ry > 1.0);
    }

    #[test]
    fn back_front_split_partitions_all_edges() {
        let cam = default_camera(Vec2 { x: 100.0, y: 100.0 }, 80.0);
        let edges = project_edges(&cam);
        let (back, front) = split_edges_back_front(&edges);
        assert_eq!(back.len() + front.len(), edges.len());
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --lib particle_diorama::cage::tests -- --nocapture`

Expected: compile failure or FAIL (missing items / `todo!`).

- [ ] **Step 4: Implement `cage.rs`**

```rust
//! Unit-cube wireframe projection for the diorama cage.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge3 {
    pub a: Vec3,
    pub b: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectedEdge {
    pub a: Vec2,
    pub b: Vec2,
    /// Average camera-space depth (larger = farther). Used for back/front split.
    pub depth: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub scale: f32,
    pub origin: Vec2,
}

pub fn default_camera(origin: Vec2, scale: f32) -> Camera {
    Camera {
        yaw: 0.55,
        pitch: 0.40,
        scale,
        origin,
    }
}

pub fn unit_cube_edges() -> [Edge3; 12] {
    const H: f32 = 0.5;
    let v = |
        x: f32,
        y: f32,
        z: f32,
    | Vec3 { x, y, z };
    let e = |a: Vec3, b: Vec3| Edge3 { a, b };
    [
        // bottom y=-H
        e(v(-H, -H, -H), v(H, -H, -H)),
        e(v(H, -H, -H), v(H, -H, H)),
        e(v(H, -H, H), v(-H, -H, H)),
        e(v(-H, -H, H), v(-H, -H, -H)),
        // top y=H
        e(v(-H, H, -H), v(H, H, -H)),
        e(v(H, H, -H), v(H, H, H)),
        e(v(H, H, H), v(-H, H, H)),
        e(v(-H, H, H), v(-H, H, -H)),
        // verticals
        e(v(-H, -H, -H), v(-H, H, -H)),
        e(v(H, -H, -H), v(H, H, -H)),
        e(v(H, -H, H), v(H, H, H)),
        e(v(-H, -H, H), v(-H, H, H)),
    ]
}

fn rotate(p: Vec3, cam: &Camera) -> Vec3 {
    let cy = cam.yaw.cos();
    let sy = cam.yaw.sin();
    let cp = cam.pitch.cos();
    let sp = cam.pitch.sin();
    let x1 = p.x * cy + p.z * sy;
    let z1 = -p.x * sy + p.z * cy;
    let y2 = p.y * cp - z1 * sp;
    let z2 = p.y * sp + z1 * cp;
    Vec3 {
        x: x1,
        y: y2,
        z: z2,
    }
}

pub fn project_point(p: Vec3, cam: &Camera) -> Vec2 {
    let r = rotate(p, cam);
    Vec2 {
        x: cam.origin.x + r.x * cam.scale,
        y: cam.origin.y - r.y * cam.scale,
    }
}

fn depth_of(p: Vec3, cam: &Camera) -> f32 {
    rotate(p, cam).z
}

pub fn project_edges(cam: &Camera) -> Vec<ProjectedEdge> {
    unit_cube_edges()
        .into_iter()
        .map(|e| {
            let da = depth_of(e.a, cam);
            let db = depth_of(e.b, cam);
            ProjectedEdge {
                a: project_point(e.a, cam),
                b: project_point(e.b, cam),
                depth: 0.5 * (da + db),
            }
        })
        .collect()
}

pub fn split_edges_back_front(edges: &[ProjectedEdge]) -> (Vec<ProjectedEdge>, Vec<ProjectedEdge>) {
    let mut depths: Vec<f32> = edges.iter().map(|e| e.depth).collect();
    depths.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = depths.get(depths.len() / 2).copied().unwrap_or(0.0);
    let mut back = Vec::new();
    let mut front = Vec::new();
    for e in edges {
        if e.depth >= mid {
            back.push(*e);
        } else {
            front.push(*e);
        }
    }
    (back, front)
}

pub fn underglow_ellipse(cam: &Camera) -> (Vec2, f32, f32) {
    const H: f32 = 0.5;
    let corners = [
        Vec3 {
            x: -H,
            y: -H,
            z: -H,
        },
        Vec3 {
            x: H,
            y: -H,
            z: -H,
        },
        Vec3 {
            x: H,
            y: -H,
            z: H,
        },
        Vec3 {
            x: -H,
            y: -H,
            z: H,
        },
    ];
    let pts: Vec<Vec2> = corners.iter().map(|p| project_point(*p, cam)).collect();
    let cx = pts.iter().map(|p| p.x).sum::<f32>() / 4.0;
    let cy = pts.iter().map(|p| p.y).sum::<f32>() / 4.0 + cam.scale * 0.06;
    let rx = pts
        .iter()
        .map(|p| (p.x - cx).abs())
        .fold(0.0_f32, f32::max)
        .max(8.0)
        * 1.15;
    let ry = (rx * 0.28).max(4.0);
    (Vec2 { x: cx, y: cy }, rx, ry)
}
```

Keep the `#[cfg(test)]` module from Step 2 at the bottom of `cage.rs`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib particle_diorama::cage::tests -- --nocapture`

Expected: PASS (4 tests).

- [ ] **Step 6: Commit**

```bash
git add src/app/onboarding/mod.rs src/app/onboarding/particle_diorama/
git commit -m "$(cat <<'EOF'
feat(onboarding): add diorama cage projection math

Unit-cube edge projection, back/front split, and underglow footprint helpers with tests.
EOF
)"
```

---

### Task 2: Simulation — spawn, containment, soft press

**Files:**
- Modify: `src/app/onboarding/particle_diorama/sim.rs`
- Modify: `src/app/onboarding/particle_diorama/mod.rs` (ensure `pub use` matches)

**Interfaces:**
- Consumes: `cage::Vec3`
- Produces:
  - `pub const PARTICLE_COUNT: usize = 640;`
  - `pub const HALF: f32 = 0.5;`
  - `pub struct Particle { pub pos: Vec3, pub vel: Vec3 }`
  - `pub struct DioramaSim { /* private fields */ }`
  - `impl DioramaSim { pub fn new(seed: u32) -> Self; pub fn tick(&mut self, dt: f32); pub fn particles(&self) -> &[Particle]; pub fn set_hover_target(&mut self, hovered: bool); pub fn hover_amount(&self) -> f32; pub fn soft_press_weight(pos: Vec3) -> f32; pub fn wall_band_fraction(&self, band: f32) -> f32; }`

- [ ] **Step 1: Write the failing sim tests**

Replace stub `sim.rs` with tests + `todo!` skeletons first:

```rust
use super::cage::Vec3;

pub const PARTICLE_COUNT: usize = 640;
pub const HALF: f32 = 0.5;
pub const DEFAULT_SEED: u32 = 0xD10R_AMA1; // use 0xD10R_AMA1 invalid — use 0xD1_0A_4A_A1

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
}

pub struct DioramaSim {
    particles: Vec<Particle>,
    time: f32,
    hover_amount: f32,
    hover_target: f32,
    seed: u32,
}

impl DioramaSim {
    pub fn new(seed: u32) -> Self {
        todo!()
    }
    pub fn tick(&mut self, dt: f32) {
        todo!()
    }
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }
    pub fn set_hover_target(&mut self, hovered: bool) {
        self.hover_target = if hovered { 1.0 } else { 0.0 };
    }
    pub fn hover_amount(&self) -> f32 {
        self.hover_amount
    }
    pub fn soft_press_weight(pos: Vec3) -> f32 {
        todo!()
    }
    pub fn wall_band_fraction(&self, band: f32) -> f32 {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_spawns_fixed_count_inside_cube() {
        let sim = DioramaSim::new(0xD10A_4AA1);
        assert_eq!(sim.particles().len(), PARTICLE_COUNT);
        for p in sim.particles() {
            assert!(p.pos.x.abs() <= HALF + 1e-4);
            assert!(p.pos.y.abs() <= HALF + 1e-4);
            assert!(p.pos.z.abs() <= HALF + 1e-4);
        }
    }

    #[test]
    fn containment_holds_after_many_steps() {
        let mut sim = DioramaSim::new(0xD10A_4AA1);
        for _ in 0..240 {
            sim.tick(1.0 / 60.0);
        }
        for p in sim.particles() {
            assert!(p.pos.x.abs() <= HALF + 1e-3);
            assert!(p.pos.y.abs() <= HALF + 1e-3);
            assert!(p.pos.z.abs() <= HALF + 1e-3);
        }
    }

    #[test]
    fn soft_press_weight_higher_near_faces() {
        let center = Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let near = Vec3 {
            x: 0.46,
            y: 0.0,
            z: 0.0,
        };
        assert!(DioramaSim::soft_press_weight(near) > DioramaSim::soft_press_weight(center));
    }

    #[test]
    fn wall_band_occupancy_rises_vs_uniform_expectation() {
        let mut sim = DioramaSim::new(0xD10A_4AA1);
        for _ in 0..180 {
            sim.tick(1.0 / 60.0);
        }
        let frac = sim.wall_band_fraction(0.12);
        assert!(
            frac > 0.22,
            "expected soft-press to enrich wall band, got {frac}"
        );
    }
}
```

Fix the seed constant comment — use `pub const DEFAULT_SEED: u32 = 0xD10A_4AA1;`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib particle_diorama::sim::tests -- --nocapture`

Expected: FAIL / panic on `todo!`.

- [ ] **Step 3: Implement simulation core**

Full `sim.rs` implementation (replace stubs):

```rust
//! Contained breathing mist inside the unit cube.

use super::cage::Vec3;

pub const PARTICLE_COUNT: usize = 640;
pub const HALF: f32 = 0.5;
pub const DEFAULT_SEED: u32 = 0xD10A_4AA1;
const PRESS_BAND: f32 = 0.14;
const HOVER_EASE: f32 = 8.0;

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
}

pub struct DioramaSim {
    particles: Vec<Particle>,
    time: f32,
    hover_amount: f32,
    hover_target: f32,
    seed: u32,
}

impl DioramaSim {
    pub fn new(seed: u32) -> Self {
        let mut particles = Vec::with_capacity(PARTICLE_COUNT);
        for i in 0..PARTICLE_COUNT {
            let u = hash_unit(seed ^ (i as u32).wrapping_mul(0x9E37_79B9));
            let v = hash_unit(seed.wrapping_add(i as u32).wrapping_mul(0x85EB_CA6B));
            let w = hash_unit(seed ^ (!(i as u32)).wrapping_mul(0xC2B2_AE35));
            let pos = Vec3 {
                x: (u * 2.0 - 1.0) * (HALF * 0.92),
                y: (v * 2.0 - 1.0) * (HALF * 0.92),
                z: (w * 2.0 - 1.0) * (HALF * 0.92),
            };
            particles.push(Particle {
                pos,
                vel: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            });
        }
        Self {
            particles,
            time: 0.0,
            hover_amount: 0.0,
            hover_target: 0.0,
            seed,
        }
    }

    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    pub fn set_hover_target(&mut self, hovered: bool) {
        self.hover_target = if hovered { 1.0 } else { 0.0 };
    }

    pub fn hover_amount(&self) -> f32 {
        self.hover_amount
    }

    pub fn soft_press_weight(pos: Vec3) -> f32 {
        let d = min_face_distance(pos);
        (1.0 - (d / PRESS_BAND).clamp(0.0, 1.0)).powf(1.5)
    }

    pub fn wall_band_fraction(&self, band: f32) -> f32 {
        let n = self.particles.len().max(1) as f32;
        let inside = self
            .particles
            .iter()
            .filter(|p| min_face_distance(p.pos) <= band)
            .count() as f32;
        inside / n
    }

    pub fn tick(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 0.1);
        self.time += dt;
        let t = self.time;
        // Ease hover
        let k = 1.0 - (-HOVER_EASE * dt).exp();
        self.hover_amount += (self.hover_target - self.hover_amount) * k;

        for (i, p) in self.particles.iter_mut().enumerate() {
            let curl = curl_noise(p.pos, t, self.seed ^ i as u32);
            let center = Vec3 {
                x: -p.pos.x,
                y: -p.pos.y,
                z: -p.pos.z,
            };
            let wall = soft_press_force(p.pos);
            let hover = hover_bias(p.pos, self.hover_amount);

            let accel_x = curl.x * 0.55 + center.x * 0.35 + wall.x * 1.1 + hover.x * 0.45;
            let accel_y = curl.y * 0.55 + center.y * 0.35 + wall.y * 1.1 + hover.y * 0.45;
            let accel_z = curl.z * 0.55 + center.z * 0.35 + wall.z * 1.1 + hover.z * 0.45;

            p.vel.x = (p.vel.x + accel_x * dt) * 0.92;
            p.vel.y = (p.vel.y + accel_y * dt) * 0.92;
            p.vel.z = (p.vel.z + accel_z * dt) * 0.92;

            p.pos.x += p.vel.x * dt;
            p.pos.y += p.vel.y * dt;
            p.pos.z += p.vel.z * dt;
            clamp_inside(&mut p.pos, &mut p.vel);
        }
    }
}

fn min_face_distance(p: Vec3) -> f32 {
    (HALF - p.x.abs())
        .min(HALF - p.y.abs())
        .min(HALF - p.z.abs())
        .max(0.0)
}

fn soft_press_force(p: Vec3) -> Vec3 {
    // Attract toward a thin shell just inside each nearby face (density press).
    let target = PRESS_BAND * 0.55;
    let mut f = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    for (coord, out) in [
        (p.x, &mut f.x),
        (p.y, &mut f.y),
        (p.z, &mut f.z),
    ] {
        let dist = HALF - coord.abs();
        if dist < PRESS_BAND {
            let sign = if coord >= 0.0 { 1.0 } else { -1.0 };
            let shell = (HALF - target) * sign;
            *out += (shell - coord) * (1.0 - dist / PRESS_BAND);
        }
    }
    f
}

fn hover_bias(p: Vec3, amount: f32) -> Vec3 {
    if amount <= 0.001 {
        return Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
    }
    // Subtle lift + mild +X bias when hovered.
    Vec3 {
        x: amount * 0.15,
        y: amount * 0.12 * (1.0 - p.y.abs()),
        z: amount * -0.05,
    }
}

fn clamp_inside(pos: &mut Vec3, vel: &mut Vec3) {
    let limit = HALF - 0.002;
    for (c, v) in [
        (&mut pos.x, &mut vel.x),
        (&mut pos.y, &mut vel.y),
        (&mut pos.z, &mut vel.z),
    ] {
        if *c > limit {
            *c = limit;
            *v *= -0.15;
        } else if *c < -limit {
            *c = -limit;
            *v *= -0.15;
        }
    }
}

fn hash_unit(mut x: u32) -> f32 {
    x = x.wrapping_mul(0x9E37_79B9);
    x ^= x >> 16;
    x = x.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 13;
    (x as f32) / (u32::MAX as f32)
}

fn value_noise(p: Vec3, seed: u32) -> f32 {
    let ix = p.x.floor() as i32;
    let iy = p.y.floor() as i32;
    let iz = p.z.floor() as i32;
    let fx = p.x.fract().abs();
    let fy = p.y.fract().abs();
    let fz = p.z.fract().abs();
    let corner = |x: i32, y: i32, z: i32| {
        let h = seed
            ^ (x as u32).wrapping_mul(374761393)
            ^ (y as u32).wrapping_mul(668265263)
            ^ (z as u32).wrapping_mul(2147483647);
        hash_unit(h) * 2.0 - 1.0
    };
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let sz = fz * fz * (3.0 - 2.0 * fz);
    let n000 = corner(ix, iy, iz);
    let n100 = corner(ix + 1, iy, iz);
    let n010 = corner(ix, iy + 1, iz);
    let n110 = corner(ix + 1, iy + 1, iz);
    let n001 = corner(ix, iy, iz + 1);
    let n101 = corner(ix + 1, iy, iz + 1);
    let n011 = corner(ix, iy + 1, iz + 1);
    let n111 = corner(ix + 1, iy + 1, iz + 1);
    let x00 = n000 + (n100 - n000) * sx;
    let x10 = n010 + (n110 - n010) * sx;
    let x01 = n001 + (n101 - n001) * sx;
    let x11 = n011 + (n111 - n011) * sx;
    let y0 = x00 + (x10 - x00) * sy;
    let y1 = x01 + (x11 - x01) * sy;
    y0 + (y1 - y0) * sz
}

fn curl_noise(p: Vec3, time: f32, seed: u32) -> Vec3 {
    let q = Vec3 {
        x: p.x * 2.2 + time * 0.15,
        y: p.y * 2.2 + time * 0.11,
        z: p.z * 2.2 + time * 0.09,
    };
    let e = 0.05;
    let dx = Vec3 {
        x: e,
        y: 0.0,
        z: 0.0,
    };
    let dy = Vec3 {
        x: 0.0,
        y: e,
        z: 0.0,
    };
    let dz = Vec3 {
        x: 0.0,
        y: 0.0,
        z: e,
    };
    let n = |pp: Vec3, s: u32| value_noise(pp, s);
    let cx = n(Vec3 {
        x: q.x,
        y: q.y + e,
        z: q.z,
    }, seed) - n(Vec3 {
        x: q.x,
        y: q.y - e,
        z: q.z,
    }, seed);
    // Proper curl via finite differences on three noise potentials:
    let px = |pp| n(pp, seed);
    let py = |pp| n(pp, seed ^ 0xA5A5_5A5A);
    let pz = |pp| n(pp, seed ^ 0x3C6E_F372);
    let _ = (dx, dy, dz, cx);
    let curl_x = (pz(Vec3 {
        x: q.x,
        y: q.y + e,
        z: q.z,
    }) - pz(Vec3 {
        x: q.x,
        y: q.y - e,
        z: q.z,
    })) - (py(Vec3 {
        x: q.x,
        y: q.y,
        z: q.z + e,
    }) - py(Vec3 {
        x: q.x,
        y: q.y,
        z: q.z - e,
    }));
    let curl_y = (px(Vec3 {
        x: q.x,
        y: q.y,
        z: q.z + e,
    }) - px(Vec3 {
        x: q.x,
        y: q.y,
        z: q.z - e,
    })) - (pz(Vec3 {
        x: q.x + e,
        y: q.y,
        z: q.z,
    }) - pz(Vec3 {
        x: q.x - e,
        y: q.y,
        z: q.z,
    }));
    let curl_z = (py(Vec3 {
        x: q.x + e,
        y: q.y,
        z: q.z,
    }) - py(Vec3 {
        x: q.x - e,
        y: q.y,
        z: q.z,
    })) - (px(Vec3 {
        x: q.x,
        y: q.y + e,
        z: q.z,
    }) - px(Vec3 {
        x: q.x,
        y: q.y - e,
        z: q.z,
    }));
    Vec3 {
        x: curl_x / (2.0 * e),
        y: curl_y / (2.0 * e),
        z: curl_z / (2.0 * e),
    }
}
```

**Implementer note:** Clean up the unused `axes` / early `cx` leftovers while implementing — the plan’s curl body above is authoritative; delete dead bindings so `cargo clippy` stays quiet. If `wall_band_fraction` assertion flakes, raise steps to 300 or lower threshold to `0.18`, but keep the test meaningful (must fail if soft-press force is removed).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib particle_diorama::sim::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/app/onboarding/particle_diorama/sim.rs src/app/onboarding/particle_diorama/mod.rs
git commit -m "$(cat <<'EOF'
feat(onboarding): add diorama particle simulation

Curl advection, soft wall press, containment, and hover easing with unit tests.
EOF
)"
```

---

### Task 3: Canvas view (cage + mist + underglow)

**Files:**
- Modify: `src/app/onboarding/particle_diorama/view.rs`
- Modify: `src/app/onboarding/particle_diorama/mod.rs` (export `diorama_canvas`)

**Interfaces:**
- Consumes: `DioramaSim`, `project_edges`, `split_edges_back_front`, `underglow_ellipse`, `project_point`, `OpenCoreTheme`, `ForegroundToken`, `ThemeRgba`
- Produces: `pub fn diorama_canvas(theme: OpenCoreTheme, sim: &DioramaSim) -> impl IntoElement`

- [ ] **Step 1: Write a pure helper test for particle screen mapping**

Add to `view.rs` (or a small `fn particle_draw_size` in `view.rs` tested):

```rust
pub(crate) fn particle_alpha(depth: f32, soft: f32, hover: f32) -> f32 {
    let depth_t = (1.0 - depth * 0.35).clamp(0.25, 1.0);
    (0.18 + soft * 0.45 + hover * 0.08) * depth_t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_alpha_increases_with_soft_press() {
        let a0 = particle_alpha(0.0, 0.0, 0.0);
        let a1 = particle_alpha(0.0, 1.0, 0.0);
        assert!(a1 > a0);
    }
}
```

- [ ] **Step 2: Run test — expect fail until helper exists**

Run: `cargo test --lib particle_diorama::view::tests -- --nocapture`

- [ ] **Step 3: Implement `diorama_canvas`**

```rust
//! GPUI canvas painter for the particle diorama.

use gpui::{
    Bounds, IntoElement, PathBuilder, Pixels, Point, size, fill, point, px, canvas, black,
};
use crate::shared::theme::{ForegroundToken, OpenCoreTheme, ThemeRgba};

use super::cage::{
    Camera, Vec2, Vec3, default_camera, project_edges, project_point, split_edges_back_front,
    underglow_ellipse,
};
use super::sim::{DioramaSim, HALF};

pub(crate) fn particle_alpha(depth: f32, soft: f32, hover: f32) -> f32 {
    let depth_t = (1.0 - depth * 0.35).clamp(0.25, 1.0);
    (0.18 + soft * 0.45 + hover * 0.08) * depth_t
}

pub fn diorama_canvas(theme: OpenCoreTheme, sim: &DioramaSim) -> impl IntoElement {
    let cage_hsla = theme.foreground(ForegroundToken::Primary);
    let glow_hsla = theme.foreground(ForegroundToken::Accent);
    let mist_hsla = theme.foreground(ForegroundToken::Secondary);
    let cage = ThemeRgba::from_hsla(cage_hsla);
    let glow = ThemeRgba::from_hsla(glow_hsla);
    let mist = ThemeRgba::from_hsla(mist_hsla);

    // Snapshot particle data for the 'static paint closure.
    let particles: Vec<(Vec3, f32)> = sim
        .particles()
        .iter()
        .map(|p| (p.pos, DioramaSim::soft_press_weight(p.pos)))
        .collect();
    let hover = sim.hover_amount();

    canvas(
        move |_bounds, _window, _cx| {},
        move |bounds: Bounds<Pixels>, _meta, window, _cx| {
            let origin = Vec2 {
                x: bounds.origin.x.0 + bounds.size.width.0 * 0.5,
                y: bounds.origin.y.0 + bounds.size.height.0 * 0.52,
            };
            let scale = bounds.size.height.0.min(bounds.size.width.0) * 0.34;
            let cam = default_camera(origin, scale);
            let edges = project_edges(&cam);
            let (back, front) = split_edges_back_front(&edges);

            // Underglow: stacked soft quads approximating an ellipse pool.
            let (gc, rx, ry) = underglow_ellipse(&cam);
            for i in 0..5 {
                let t = i as f32 / 4.0;
                let alpha = (0.16 * (1.0 - t)).max(0.02);
                let w = rx * (0.55 + t * 0.9);
                let h = ry * (0.55 + t * 0.9);
                let color = gpui::Rgba {
                    r: glow.r,
                    g: glow.g,
                    b: glow.b,
                    a: glow.a * alpha,
                };
                window.paint_quad(fill(
                    Bounds {
                        origin: point(px(gc.x - w), px(gc.y - h)),
                        size: size(px(w * 2.0), px(h * 2.0)),
                    },
                    color,
                ).corner_radii(px(h.max(w))));
            }

            let stroke_edges = |window: &mut gpui::Window, edges: &[super::cage::ProjectedEdge], alpha: f32| {
                for e in edges {
                    let mut builder = PathBuilder::stroke(px(1.25));
                    let _ = builder.move_to(point(px(e.a.x), px(e.a.y)));
                    let _ = builder.line_to(point(px(e.b.x), px(e.b.y)));
                    if let Ok(path) = builder.build() {
                        let color = gpui::Rgba {
                            r: cage.r,
                            g: cage.g,
                            b: cage.b,
                            a: cage.a * alpha,
                        };
                        window.paint_path(path, color);
                    }
                }
            };

            stroke_edges(window, &back, 0.35);

            // Particles between back and front edges.
            for (pos, soft) in &particles {
                let r = super::cage::rotate_for_depth(*pos, &cam); // see note below
                let depth = r; // normalized later
                let screen = project_point(*pos, &cam);
                let depth_n = ((depth + HALF) / (2.0 * HALF)).clamp(0.0, 1.0);
                let a = particle_alpha(depth_n, *soft, hover);
                let sz = 1.2 + soft * 1.4 + (1.0 - depth_n) * 0.6;
                let color = gpui::Rgba {
                    r: mist.r,
                    g: mist.g,
                    b: mist.b,
                    a: mist.a * a,
                };
                window.paint_quad(fill(
                    Bounds {
                        origin: point(px(screen.x - sz * 0.5), px(screen.y - sz * 0.5)),
                        size: size(px(sz), px(sz)),
                    },
                    color,
                ));
            }

            stroke_edges(window, &front, 0.85);
            let _ = black(); // silence if unused — remove if not needed
            let _ = Point::<Pixels>::new(px(0.), px(0.));
        },
    )
    .size_full()
}
```

**Critical implementer fix before commit:** `rotate_for_depth` is not public in Task 1. Either:

1. Add `pub fn camera_depth(p: Vec3, cam: &Camera) -> f32` to `cage.rs` (same as Task 1’s private `depth_of`), with a tiny unit test that it is finite, **or**
2. Approximate depth as `pos.z` after a duplicated rotate in `view.rs`.

Prefer option 1 — add `camera_depth` to `cage.rs` in this task’s commit if missing.

Also delete unused `black()` / dummy `Point` lines. Match exact GPUI `PathBuilder` / `Rgba` / `fill` APIs at the pinned Zed rev (adjust import paths if the compiler says so — do not invent alternate stacks).

- [ ] **Step 4: `cargo test --lib particle_diorama` and `cargo check`**

Expected: tests PASS; project compiles.

- [ ] **Step 5: Commit**

```bash
git add src/app/onboarding/particle_diorama/
git commit -m "$(cat <<'EOF'
feat(onboarding): paint particle diorama on GPUI canvas

Edge-only cage with back/front split, theme-token underglow, and light mist points.
EOF
)"
```

---

### Task 4: Wire into onboarding UI state + hover callback

**Files:**
- Modify: `src/app/onboarding/onboarding_ui_state.rs`
- Modify: `src/app/onboarding/onboarding_view.rs`
- Modify: `src/app/app_desktop.rs`

**Interfaces:**
- Consumes: `DioramaSim`, `diorama_canvas`, existing RAF tick loop
- Produces:
  - `OnboardingUiState::diorama(&self) -> &DioramaSim`
  - `OnboardingUiState::set_hero_hovered(&mut self, hovered: bool)`
  - `OnboardingCallbacks { on_enter, on_toggle_theme, on_hero_hover: Rc<dyn Fn(bool, &mut Window, &mut App)> }`
  - `const USE_PARTICLE_DIORAMA: bool = true;`

- [ ] **Step 1: Extend `OnboardingUiState`**

```rust
use super::ascii_galaxy::{DEFAULT_SEED, GalaxyAscii};
use super::particle_diorama::sim::{DEFAULT_SEED as DIORAMA_SEED, DioramaSim};

pub struct OnboardingUiState {
    galaxy: GalaxyAscii,
    diorama: DioramaSim,
    last_tick: Instant,
    focus_claimed: bool,
}

impl OnboardingUiState {
    pub fn new() -> Self {
        let mut galaxy = GalaxyAscii::new(DEFAULT_SEED);
        let _ = galaxy.tick(0.0);
        let diorama = DioramaSim::new(DIORAMA_SEED);
        Self {
            galaxy,
            diorama,
            last_tick: Instant::now(),
            focus_claimed: false,
        }
    }

    pub fn tick(&mut self, now: Instant) {
        let dt = now
            .saturating_duration_since(self.last_tick)
            .as_secs_f32()
            .clamp(0.0, 0.1);
        self.last_tick = now;
        let _ = self.galaxy.tick(dt);
        self.diorama.tick(dt);
    }

    pub fn last_frame(&self) -> &str {
        self.galaxy.last_frame()
    }

    pub fn diorama(&self) -> &DioramaSim {
        &self.diorama
    }

    pub fn set_hero_hovered(&mut self, hovered: bool) {
        self.diorama.set_hover_target(hovered);
    }

    // keep ensure_initial_focus unchanged
}
```

Also drive a small focus bias: in `tick`, after computing dt, if you pass focus later — for v1, hover alone is enough; optional: `set_hover_target(hovered || focused)` from the view via the same callback when focus is on the onboarding root. Spec asks subtle hover/**focus**: in `app_desktop` render, before paint:

```rust
let focused = self.focus_handle.is_focused(window);
if let Some(ui) = self.onboarding_ui.as_mut() {
    // Focus contributes a soft target without fighting hover: OR in setter from view,
    // or call `ui.diorama.set_hover_target(ui_hero_hovered || focused)` if you store hero_hovered bool separately.
}
```

Simplest clean approach: store `hero_hovered: bool` on `OnboardingUiState`, and in `tick`:

```rust
let focus_boost = false; // set from parameter — see below
self.diorama
    .set_hover_target(self.hero_hovered || focus_boost);
self.diorama.tick(dt);
```

Change `tick` to `tick(&mut self, now: Instant, onboarding_focused: bool)` and pass `self.focus_handle.is_focused(window)` from `app_desktop`.

- [ ] **Step 2: Add layout constant test update**

In `onboarding_view.rs` tests, add:

```rust
#[test]
fn particle_diorama_flag_is_bool() {
    let _ = USE_PARTICLE_DIORAMA;
}
```

- [ ] **Step 3: Swap hero + hover handlers**

```rust
const USE_PARTICLE_DIORAMA: bool = true;

pub struct OnboardingCallbacks {
    pub on_enter: WindowAppHandler,
    pub on_toggle_theme: WindowAppHandler,
    pub on_hero_hover: Rc<dyn Fn(bool, &mut Window, &mut App)>,
}
```

In `hero_block`, when flag is true:

```rust
let on_hover = callbacks.on_hero_hover.clone();
let on_leave = callbacks.on_hero_hover.clone();
let hero_visual = div()
    .relative()
    .w_full()
    .h(px(ASCII_HERO_HEIGHT))
    .flex()
    .items_center()
    .justify_center()
    .on_mouse_enter(move |_, window, cx| on_hover(true, window, cx))
    .on_mouse_leave(move |_, window, cx| on_leave(false, window, cx))
    .child(diorama_canvas(theme, ui.diorama()));
```

When flag is false, keep existing ASCII path (including `hero_glow`).

Pass `callbacks` into `hero_block` (update signature).

- [ ] **Step 4: Wire `OnboardingCallbacks::from_app`**

```rust
let on_hero_hover = {
    let view = view.clone();
    Rc::new(move |hovered: bool, window: &mut Window, cx: &mut App| {
        view.update(cx, |app, cx| {
            if let Some(ui) = app.onboarding_ui.as_mut() {
                ui.set_hero_hovered(hovered);
            }
            cx.notify();
            window.request_animation_frame();
        });
    }) as Rc<dyn Fn(bool, &mut Window, &mut App)>
};
```

Update `ui.tick(Instant::now(), self.focus_handle.is_focused(window));`.

- [ ] **Step 5: Run tests**

Run:

```bash
cargo test --lib particle_diorama -- --nocapture
cargo test --lib onboarding -- --nocapture
cargo check
```

Expected: PASS / compile OK.

- [ ] **Step 6: Commit**

```bash
git add src/app/onboarding/onboarding_ui_state.rs src/app/onboarding/onboarding_view.rs src/app/app_desktop.rs src/app/onboarding/particle_diorama/
git commit -m "$(cat <<'EOF'
feat(onboarding): wire particle diorama hero behind swap flag

Tick sim on RAF, theme-painted canvas in hero_block, subtle hover/focus bias; ASCII galaxy retained.
EOF
)"
```

---

### Task 5: Manual acceptance + docs touch-up

**Files:**
- Modify: `docs/design/2026-08-12-particle-diorama-hero-design.md` (status → Implemented in tree / ready for QA)
- Optional: set `USE_PARTICLE_DIORAMA` default based on QA (keep `true` if accepted)

- [ ] **Step 1: Run full test suite**

```bash
cargo test --lib
```

Expected: all PASS.

- [ ] **Step 2: Manual checklist (run the app)**

```bash
cargo run
```

Verify:

- [ ] Edge-only cage; no filled faces
- [ ] Mist breathes; no directed jet; no particles escape
- [ ] Soft thicken near walls
- [ ] Soft underglow under footprint
- [ ] Hover/focus: small bias; leave eases back
- [ ] Feels light on idle
- [ ] Theme toggle still works; colors track tokens
- [ ] `USE_PARTICLE_DIORAMA = false` restores ASCII hero

- [ ] **Step 3: Update design doc status line**

Set status to: `Implemented (v1); QA checklist in plan Task 5`.

- [ ] **Step 4: Commit**

```bash
git add docs/design/2026-08-12-particle-diorama-hero-design.md
git commit -m "$(cat <<'EOF'
docs: mark particle diorama design implemented for QA

EOF
)"
```

---

## Spec coverage self-check

| Spec item | Task |
|-----------|------|
| Edge-only cage | 1, 3 |
| Breathing mist / curl | 2 |
| Soft press | 2, 3 |
| Underglow | 1, 3 |
| Hover/focus subtle bias | 2, 4 |
| Light mist (640) | 2 |
| Theme tokens | 3, 4 |
| Keep ASCII + swap flag | 4 |
| Containment / soft press / projection tests | 1, 2 |
| Approach 1 + optional back/front | 3 (`split_edges_back_front`) |
| No shaders / no copy | Global constraints |

## Placeholder scan

None intentionally left. If GPUI paint API names differ at the pinned rev, fix compile errors in Task 3 without changing architecture.
