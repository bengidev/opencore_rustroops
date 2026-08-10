//! Morphing 3D wireframe spiral rendered headlessly via `ascii_renderer`.

use ascii_renderer::prelude::*;

pub const COLS: usize = 72;
pub const ROWS: usize = 22;
pub const DEFAULT_SEED: u32 = 0x4E_07_41_46;
pub const TARGET_DT: f32 = 1.0 / 22.0;

const ARM_COUNT: usize = 3;
const POINTS_PER_ARM: usize = 48;

pub struct GalaxyAscii {
    renderer: Renderer,
    buffer: CharBuffer,
    rest: Vec<Vector3>,
    time: f32,
    last_frame: String,
    #[allow(dead_code)]
    seed: u32,
}

impl GalaxyAscii {
    pub fn new(seed: u32) -> Self {
        let (mesh, rest) = build_spiral_mesh();
        let mut galaxy = Self {
            renderer: Renderer {
                meshs: vec![mesh],
                camera: Camera {
                    position: vec3!(0.0, 0.0, -7.0),
                    rotation: vec3!(0.0, 0.0, 0.0),
                    fov: vec2!(0.95, 0.95 * (ROWS as f32 / COLS as f32)),
                },
            },
            buffer: CharBuffer::new(COLS, ROWS),
            rest,
            time: 0.0,
            last_frame: String::new(),
            seed,
        };
        galaxy.tick(0.0);
        galaxy
    }

    pub fn tick(&mut self, dt: f32) -> String {
        self.time += dt;

        let mesh = &mut self.renderer.meshs[0];
        let verts = mesh.get_verticies_mut();
        for (idx, &p) in self.rest.iter().enumerate() {
            let a = idx / POINTS_PER_ARM;
            let i = idx % POINTS_PER_ARM;
            let t = i as f32 / (POINTS_PER_ARM - 1) as f32;
            let wave = (self.time * 1.7 + a as f32 * 1.1 + t * 4.0).sin();
            let fold = (self.time * 2.3 + a as f32 * 0.7 + t * 2.5).cos();
            let radial = 1.0 + wave * 0.22 + fold * 0.12;
            let y_off = fold * 0.18 * t;
            verts.insert(
                idx,
                vec3!(p.x * radial, p.y + y_off, p.z * radial),
            );
        }

        mesh.rotation.y += dt * 0.55;
        mesh.rotation.x += dt * 0.22;

        self.buffer.fill(' ');
        self.renderer.draw(&mut self.buffer);
        self.last_frame = buffer_to_frame(&self.buffer);
        self.last_frame.clone()
    }

    pub fn frame(&self) -> &str {
        &self.last_frame
    }

    pub fn vertex_count(&self) -> usize {
        self.rest.len()
    }

    pub fn edge_count(&self) -> usize {
        self.renderer.meshs[0].get_edges().len()
    }
}

fn build_spiral_mesh() -> (Mesh, Vec<Vector3>) {
    let mut mesh = Mesh::default();
    mesh.char = '*';
    let mut rest = Vec::with_capacity(ARM_COUNT * POINTS_PER_ARM);

    for a in 0..ARM_COUNT {
        for i in 0..POINTS_PER_ARM {
            let t = i as f32 / (POINTS_PER_ARM - 1) as f32;
            let radius = 0.35 + t * 2.4;
            let theta = t * 4.5 + a as f32 * (std::f32::consts::TAU / ARM_COUNT as f32);
            let x = radius * theta.cos();
            let y = (t - 0.5) * 0.35;
            let z = radius * theta.sin();
            let pos = vec3!(x, y, z);
            let idx = a * POINTS_PER_ARM + i;
            mesh.insert_vertex(idx, pos);
            rest.push(pos);
        }
    }

    for a in 0..ARM_COUNT {
        for i in 0..(POINTS_PER_ARM - 1) {
            let from = a * POINTS_PER_ARM + i;
            let to = a * POINTS_PER_ARM + i + 1;
            mesh.add_edge((from, to));
        }
    }

    let core0 = 0;
    let core1 = POINTS_PER_ARM;
    let core2 = 2 * POINTS_PER_ARM;
    mesh.add_edge((core0, core1));
    mesh.add_edge((core1, core2));
    mesh.add_edge((core2, core0));

    (mesh, rest)
}

fn buffer_to_frame(buf: &CharBuffer) -> String {
    buf.data
        .iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{COLS, DEFAULT_SEED, GalaxyAscii, ROWS};

    #[test]
    fn spiral_mesh_has_expected_topology() {
        let g = GalaxyAscii::new(DEFAULT_SEED);
        assert_eq!(g.vertex_count(), 3 * 48);
        assert_eq!(g.edge_count(), 3 * 47 + 3); // arm segments + core triangle
    }

    #[test]
    fn tick_frame_has_fixed_dimensions() {
        let mut g = GalaxyAscii::new(DEFAULT_SEED);
        let frame = g.tick(1.0 / 22.0);
        assert_eq!(frame.lines().count(), ROWS);
        // Raw row length == COLS proves we did not use CharBuffer Display
        // (Display inserts a space between every glyph, roughly doubling width).
        assert!(frame.lines().all(|l| l.chars().count() == COLS));
    }

    #[test]
    fn tick_is_deterministic_for_same_dts() {
        let mut a = GalaxyAscii::new(DEFAULT_SEED);
        let mut b = GalaxyAscii::new(DEFAULT_SEED);
        let fa = a.tick(0.05);
        let fb = b.tick(0.05);
        assert_eq!(fa, fb);
        let fa2 = a.tick(0.05);
        let fb2 = b.tick(0.05);
        assert_eq!(fa2, fb2);
        assert_ne!(fa, fa2); // morph/rotation advanced
    }
}
