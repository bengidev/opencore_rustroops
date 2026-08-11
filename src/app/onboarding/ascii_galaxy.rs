//! Procedural ASCII galaxy glow field.
//!
//! Renders a fixed `COLS` by `ROWS` ASCII frame headlessly. Intensity is
//! derived from concentric rings, cross waves, and deterministic per-cell
//! seeds, then mapped through an ASCII ramp. Motion comes purely from a
//! time accumulation fed by the app's animation frame scheduler; there are
//! no timers, worker threads, or browser-style APIs.

pub const COLS: usize = 74;
pub const ROWS: usize = 44;
pub const DEFAULT_SEED: u32 = 0x4E_07_41_46;

const ASCII_RAMP: &[u8] = b" .'`^,:;Il!i><~+_-?][}{1)(|/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$";
const GLYPH_THRESHOLD: f32 = 0.22;

pub struct GalaxyAscii {
    seeds: Vec<f32>,
    time: f32,
    last_frame: String,
}

impl GalaxyAscii {
    pub fn new(seed: u32) -> Self {
        let seeds = build_cell_seeds(seed);
        let mut galaxy = Self {
            seeds,
            time: 0.0,
            last_frame: String::new(),
        };
        galaxy.tick(0.0);
        galaxy
    }

    pub fn tick(&mut self, dt: f32) -> String {
        self.time += dt;
        self.last_frame = render_frame(&self.seeds, self.time, COLS, ROWS);
        self.last_frame.clone()
    }

    pub fn last_frame(&self) -> &str {
        &self.last_frame
    }
}

/// Builds exactly `COLS * ROWS` deterministic unit-hash values in [0, 1).
fn build_cell_seeds(seed: u32) -> Vec<f32> {
    (0..(COLS * ROWS))
        .map(|idx| hash_unit(seed ^ idx as u32))
        .collect()
}

/// Small deterministic hash mapping an integer into the unit interval [0, 1).
fn hash_unit(mut x: u32) -> f32 {
    x = x.wrapping_mul(0x9E37_79B9);
    x ^= x >> 16;
    x = x.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 13;
    (x as f32) / (u32::MAX as f32)
}

/// Renders a frame by computing an intensity at every cell and mapping it
/// through the ASCII ramp, keeping cells below `GLYPH_THRESHOLD` as spaces so
/// the output preserves negative space.
fn render_frame(seeds: &[f32], time: f32, cols: usize, rows: usize) -> String {
    let mut out = String::with_capacity(cols * (rows + 1));
    let rows_div = rows.saturating_sub(1).max(1) as f32;
    let cols_div = cols.saturating_sub(1).max(1) as f32;

    for row in 0..rows {
        let ny = row as f32 / rows_div;
        let cy = ny * 2.0 - 1.0;
        for col in 0..cols {
            let nx = col as f32 / cols_div;
            let cx = nx * 2.0 - 1.0;
            let idx = row * cols + col;
            let r = (cx * cx + cy * cy).sqrt();
            // Concentric rings radiating from the center, fading outward.
            let ring = (((r * 6.5) - time * 0.8).sin() * 0.5 + 0.5) * (1.0 - r * 0.30).max(0.0);
            // Cross waves adding organic motion over time.
            let wave = (cx * 5.0 + time * 0.5).sin() * (cy * 4.0 - time * 0.4).cos() * 0.5 + 0.5;
            // Deterministic per-cell sparkle from the seeded hash field.
            let sparkle = seeds[idx];

            let intensity = (ring * 0.45 + wave * 0.35 + sparkle * 0.20).clamp(0.0, 1.0);
            out.push(glyph_for_intensity(intensity));
        }
        out.push('\n');
    }
    out
}

fn glyph_for_intensity(intensity: f32) -> char {
    if intensity < GLYPH_THRESHOLD {
        return ' ';
    }
    let idx = ((intensity - GLYPH_THRESHOLD) / (1.0 - GLYPH_THRESHOLD)) * (ASCII_RAMP.len() as f32);
    let idx = idx.clamp(0.0, (ASCII_RAMP.len() - 1) as f32) as usize;
    ASCII_RAMP[idx] as char
}

#[cfg(test)]
mod tests {
    use super::{COLS, DEFAULT_SEED, GalaxyAscii, ROWS};

    #[test]
    fn tick_frame_has_fixed_dimensions() {
        let mut g = GalaxyAscii::new(DEFAULT_SEED);
        let frame = g.tick(1.0 / 22.0);
        assert_eq!(frame.lines().count(), ROWS);
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
        assert_ne!(fa, fa2); // time accumulation advances the field
    }

    #[test]
    fn frame_contains_thresholded_glow_field() {
        let mut g = GalaxyAscii::new(DEFAULT_SEED);
        let frame = g.tick(1.0 / 24.0);
        let visible = frame.chars().filter(|&c| c != ' ' && c != '\n').count();
        let blank = frame.chars().filter(|&c| c == ' ').count();

        assert!(visible > 120, "expected visible glyphs in glow field");
        assert!(blank > 120, "expected thresholded negative space");
    }

    #[test]
    fn different_seeds_change_static_texture() {
        let mut a = GalaxyAscii::new(DEFAULT_SEED);
        let mut b = GalaxyAscii::new(DEFAULT_SEED ^ 0xA5A5_5A5A);

        assert_ne!(a.tick(0.0), b.tick(0.0));
    }

    #[test]
    fn ascii_hero_generator_dimensions() {
        assert_eq!(COLS, 74);
        assert_eq!(ROWS, 44);
    }
}
