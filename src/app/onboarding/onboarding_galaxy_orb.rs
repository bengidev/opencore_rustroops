//! Galaxy orb canvas — monochrome spiral centerpiece.
//!
//! Renders a pixel-particle spiral galaxy using grayscale stellar
//! populations. Inner nucleus glows bright; outer arms fade to muted
//! graphite. Hold-to-zoom dynamics are preserved.

use std::time::Instant;

use gpui::{Bounds, Pixels};

use crate::shared::theme::{BackgroundToken, ForegroundToken, OpenCoreTheme, ThemeRgba};

use super::onboarding_draw::{Painter, Point2, Rgba, Size2, blend, with_alpha};

use super::onboarding_dynamics::{MAX_ZOOM, SPEED_CLAMP};

const LOGICAL_SIZE: Size2 = Size2 {
    width: 520.0,
    height: 340.0,
};

const DISC_RADIUS: f32 = 148.0;
const DISC_TILT: f32 = 0.40;
const ARM_COUNT: usize = 2;
const ARM_PITCH: f32 = 0.42;
const ARM_WIDTH: f32 = 0.55;

const DISC_STAR_COUNT: usize = 360;
const ARM_SATELLITE_COUNT: usize = 96;
const HALO_STAR_COUNT: usize = 168;
const GLOBULAR_CLUSTER_COUNT: usize = 36;
const BULGE_BLOCK_COUNT: usize = 70;
const NUCLEUS_BLOCK_COUNT: usize = 30;
const STARFIELD_COUNT: usize = 90;
const JET_SEGMENTS: usize = 16;

const SNAP_GRID: f32 = 3.0;

/// Grayscale galaxy palette — inner core bright, outer disc muted.
struct GalaxyPalette {
    arm_young: Rgba,
    arm_old: Rgba,
    bulge: Rgba,
    nucleus: Rgba,
    hii_region: Rgba,
    dust: Rgba,
    halo: Rgba,
    jet: Rgba,
    starfield: Rgba,
    core: Rgba,
    rim: Rgba,
}

impl GalaxyPalette {
    fn from_theme(theme: &OpenCoreTheme) -> Self {
        let fg = |token: ForegroundToken| rgba_from_theme(theme.rgba_foreground(token));
        let surface = |token: BackgroundToken| rgba_from_theme(theme.rgba_surface(token));

        let primary = fg(ForegroundToken::Primary);
        let secondary = fg(ForegroundToken::Secondary);
        let muted = fg(ForegroundToken::Muted);
        let accent = fg(ForegroundToken::Accent);
        let tertiary = surface(BackgroundToken::Tertiary);

        Self {
            arm_young: primary,
            arm_old: secondary,
            bulge: accent,
            nucleus: primary,
            hii_region: blend(secondary, accent, 0.35),
            dust: tertiary,
            halo: muted,
            jet: accent,
            starfield: primary,
            core: primary,
            rim: muted,
        }
    }

    /// Radial tint: blends a base colour toward core (inner) or rim (outer).
    fn radial_tint(&self, base: Rgba, r: f32, strength: f32) -> Rgba {
        let r = r.clamp(0.0, 1.0);
        let rim_weight = ((r - 0.3) / 0.4).clamp(0.0, 1.0);
        let accent = blend(self.core, self.rim, rim_weight);
        blend(base, accent, strength)
    }
}

#[derive(Debug, Clone)]
struct DiscParticle {
    r: f32,
    theta0: f32,
    energy: f32,
    phase: f32,
    shimmer_phase: f32,
    pulse_phase: f32,
    base_colour: Rgba,
    has_highlight: bool,
}

#[derive(Debug, Clone)]
struct HaloParticle {
    seed: f32,
    radial: f32,
    jitter_x: f32,
    jitter_y: f32,
    rotation_rate: f32,
    angle_offset: f32,
    phase: f32,
}

#[derive(Debug, Clone)]
struct StarfieldParticle {
    seed: f32,
    x: f32,
    y: f32,
}

#[derive(Debug, Clone)]
struct GlobularParticle {
    seed: f32,
    angle_offset: f32,
    orbit_radius: f32,
    plane: f32,
    phase: f32,
    omega: f32,
}

#[derive(Debug, Clone)]
struct ArmSatelliteParticle {
    r: f32,
    arm_index: f32,
    arm_jitter: f32,
    omega: f32,
    phase: f32,
    base_colour: Rgba,
}

#[derive(Debug, Clone)]
struct BulgeParticle {
    nx: f32,
    ny: f32,
    density: f32,
    phase: f32,
    shimmer_phase: f32,
}

#[derive(Debug, Clone)]
struct NucleusParticle {
    nx: f32,
    ny: f32,
    density: f32,
    phase: f32,
    shimmer_phase: f32,
}

/// Baked galaxy particle placements — rejection sampling runs once at bake time.
#[derive(Debug, Clone)]
pub struct GalaxyParticleCache {
    theme: OpenCoreTheme,
    disc: Vec<DiscParticle>,
    halo: Vec<HaloParticle>,
    starfield: Vec<StarfieldParticle>,
    globular: Vec<GlobularParticle>,
    satellites: Vec<ArmSatelliteParticle>,
    bulge: Vec<BulgeParticle>,
    nucleus: Vec<NucleusParticle>,
}

impl GalaxyParticleCache {
    pub fn bake(theme: OpenCoreTheme) -> Self {
        let pal = GalaxyPalette::from_theme(&theme);
        Self {
            theme,
            disc: bake_disc(&pal),
            halo: bake_halo(),
            starfield: bake_starfield(),
            globular: bake_globular(),
            satellites: bake_arm_satellites(&pal),
            bulge: bake_bulge(),
            nucleus: bake_nucleus(),
        }
    }

    pub fn theme(&self) -> OpenCoreTheme {
        self.theme
    }
}

#[cfg(test)]
impl GalaxyParticleCache {
    fn disc_len(&self) -> usize {
        self.disc.len()
    }

    fn halo_len(&self) -> usize {
        self.halo.len()
    }

    fn fingerprint(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        match self.theme.mode {
            crate::shared::theme::ThemeMode::Dark => 0u8.hash(&mut hasher),
            crate::shared::theme::ThemeMode::Light => 1u8.hash(&mut hasher),
        }
        for particle in self.disc.iter().take(32) {
            particle.r.to_bits().hash(&mut hasher);
            particle.base_colour.r.to_bits().hash(&mut hasher);
            particle.base_colour.g.to_bits().hash(&mut hasher);
            particle.base_colour.b.to_bits().hash(&mut hasher);
        }
        for particle in self.halo.iter().take(16) {
            particle.seed.to_bits().hash(&mut hasher);
        }
        hasher.finish()
    }
}

fn bake_disc(pal: &GalaxyPalette) -> Vec<DiscParticle> {
    let t = 0.0;
    let mut disc = Vec::with_capacity(DISC_STAR_COUNT);
    let mut placed = 0usize;
    let mut attempt = 0usize;
    // Rejection sampling may need many tries to fill the disc; cap avoids infinite loops.
    let max_attempts = DISC_STAR_COUNT * 24;

    while placed < DISC_STAR_COUNT && attempt < max_attempts {
        let seed = 2_400.0 + attempt as f32;
        let nx = noise(seed, 3.0) * 2.0 - 1.0;
        let ny = noise(seed, 9.0) * 2.0 - 1.0;
        let r = (nx * nx + ny * ny).sqrt();

        if !(0.04..=1.0).contains(&r) {
            attempt += 1;
            continue;
        }

        let (density, arm_distance) = arm_density(nx, ny, t);
        if noise(seed, 15.0) > density {
            attempt += 1;
            continue;
        }

        let energy = (density * (1.0 - arm_distance * 0.4)).clamp(0.0, 1.0);

        let base_colour = if energy > 0.7 {
            let hii_blend = noise(seed, 97.0);
            if hii_blend > 0.85 {
                blend(pal.arm_young, pal.hii_region, (hii_blend - 0.85) * 6.67)
            } else {
                pal.arm_young
            }
        } else if energy > 0.3 {
            let interm = (energy - 0.3) / 0.4;
            blend(pal.arm_old, pal.arm_young, interm)
        } else if arm_distance < 0.2 {
            pal.dust
        } else {
            blend(pal.arm_old, pal.dust, 0.3)
        };

        let theta0 = ny.atan2(nx);
        disc.push(DiscParticle {
            r,
            theta0,
            energy,
            phase: noise(seed, 53.0) * std::f32::consts::TAU,
            shimmer_phase: noise(seed, 71.0) * std::f32::consts::TAU,
            pulse_phase: noise(seed, 89.0) * std::f32::consts::TAU,
            base_colour,
            has_highlight: energy > 0.55,
        });

        placed += 1;
        attempt += 1;
    }

    disc
}

fn bake_halo() -> Vec<HaloParticle> {
    let mut halo = Vec::with_capacity(HALO_STAR_COUNT);
    for i in 0..HALO_STAR_COUNT {
        let seed = 2_900.0 + i as f32;
        halo.push(HaloParticle {
            seed,
            radial: 0.92 + noise(seed, 3.0).powf(0.65) * 0.55,
            jitter_x: (noise(seed, 29.0) - 0.5) * 12.0,
            jitter_y: (noise(seed, 37.0) - 0.5) * 9.0,
            rotation_rate: 0.04 + noise(seed, 19.0) * 0.03,
            angle_offset: noise(seed, 11.0) * std::f32::consts::TAU,
            phase: noise(seed, 47.0) * std::f32::consts::TAU,
        });
    }
    halo
}

fn bake_starfield() -> Vec<StarfieldParticle> {
    let mut starfield = Vec::with_capacity(STARFIELD_COUNT);
    for i in 0..STARFIELD_COUNT {
        let seed = 13_700.0 + i as f32;
        let x = noise(seed, 3.0) * LOGICAL_SIZE.width;
        let y = noise(seed, 7.0) * LOGICAL_SIZE.height;

        let dx = x - LOGICAL_SIZE.width * 0.5;
        let dy = (y - LOGICAL_SIZE.height * 0.5) / DISC_TILT;
        if (dx * dx + dy * dy).sqrt() < DISC_RADIUS * 0.65 {
            continue;
        }

        starfield.push(StarfieldParticle { seed, x, y });
    }
    starfield
}

fn bake_globular() -> Vec<GlobularParticle> {
    let mut globular = Vec::with_capacity(GLOBULAR_CLUSTER_COUNT);
    for i in 0..GLOBULAR_CLUSTER_COUNT {
        let seed = 11_200.0 + i as f32;
        globular.push(GlobularParticle {
            seed,
            angle_offset: noise(seed, 13.0) * std::f32::consts::TAU,
            orbit_radius: DISC_RADIUS * (0.35 + noise(seed, 17.0) * 0.55),
            plane: 0.35 + noise(seed, 31.0) * 0.55,
            phase: noise(seed, 53.0) * std::f32::consts::TAU,
            omega: 0.10 + noise(seed, 23.0) * 0.16,
        });
    }
    globular
}

fn bake_arm_satellites(pal: &GalaxyPalette) -> Vec<ArmSatelliteParticle> {
    let warm = pal.arm_young;
    let cool = pal.hii_region;
    let mut satellites = Vec::with_capacity(ARM_SATELLITE_COUNT);
    let mut placed = 0usize;
    let mut attempt = 0usize;
    let max_attempts = ARM_SATELLITE_COUNT * 18;

    while placed < ARM_SATELLITE_COUNT && attempt < max_attempts {
        let seed = 5_500.0 + attempt as f32;
        let r = 0.18 + noise(seed, 3.0).powf(0.8) * 0.78;
        let arm_index = (noise(seed, 7.0) * ARM_COUNT as f32).floor();
        let arm_jitter = (noise(seed, 11.0) - 0.5) * ARM_WIDTH * 0.6;
        let omega = 0.18 / (0.40 + r);

        let base_colour = if noise(seed, 83.0) > 0.75 { cool } else { warm };
        satellites.push(ArmSatelliteParticle {
            r,
            arm_index,
            arm_jitter,
            omega,
            phase: noise(seed, 29.0) * std::f32::consts::TAU,
            base_colour,
        });

        placed += 1;
        attempt += 1;
    }

    satellites
}

fn bake_bulge() -> Vec<BulgeParticle> {
    let mut bulge = Vec::with_capacity(BULGE_BLOCK_COUNT);
    let mut placed = 0usize;
    let mut attempt = 0usize;
    let max_attempts = BULGE_BLOCK_COUNT * 16;

    while placed < BULGE_BLOCK_COUNT && attempt < max_attempts {
        let seed = 6_200.0 + attempt as f32;
        let nx = noise(seed, 3.0) * 2.0 - 1.0;
        let ny = noise(seed, 9.0) * 2.0 - 1.0;
        let density = gaussian2d(nx, ny, 0.30, 0.24);

        if noise(seed, 15.0) > density {
            attempt += 1;
            continue;
        }

        bulge.push(BulgeParticle {
            nx,
            ny,
            density,
            phase: noise(seed, 53.0) * std::f32::consts::TAU,
            shimmer_phase: noise(seed, 71.0) * std::f32::consts::TAU,
        });

        placed += 1;
        attempt += 1;
    }

    bulge
}

fn bake_nucleus() -> Vec<NucleusParticle> {
    let mut nucleus = Vec::with_capacity(NUCLEUS_BLOCK_COUNT);
    let mut placed = 0usize;
    let mut attempt = 0usize;
    let max_attempts = NUCLEUS_BLOCK_COUNT * 16;

    while placed < NUCLEUS_BLOCK_COUNT && attempt < max_attempts {
        let seed = 7_700.0 + attempt as f32;
        let nx = noise(seed, 3.0) * 2.0 - 1.0;
        let ny = noise(seed, 9.0) * 2.0 - 1.0;
        let density = gaussian2d(nx, ny, 0.16, 0.14);

        if noise(seed, 15.0) < density {
            nucleus.push(NucleusParticle {
                nx,
                ny,
                density,
                phase: noise(seed, 53.0) * std::f32::consts::TAU,
                shimmer_phase: noise(seed, 71.0) * std::f32::consts::TAU,
            });
            placed += 1;
        }
        attempt += 1;
    }

    nucleus
}

#[derive(Debug, Clone, Copy)]
pub struct GalaxyOrb {
    started_at: Instant,
    now: Instant,
    speed_multiplier: f32,
    zoom: f32,
}

impl GalaxyOrb {
    pub fn with_dynamics(
        started_at: Instant,
        now: Instant,
        speed_multiplier: f32,
        zoom: f32,
    ) -> Self {
        Self {
            started_at,
            now,
            speed_multiplier: speed_multiplier.clamp(0.0, SPEED_CLAMP),
            zoom: zoom.clamp(1.0, MAX_ZOOM),
        }
    }

    fn elapsed_seconds(&self) -> f32 {
        self.now
            .saturating_duration_since(self.started_at)
            .as_secs_f32()
    }

    pub fn paint(
        &self,
        cache: &GalaxyParticleCache,
        painter: &mut Painter<'_>,
        bounds: Bounds<Pixels>,
    ) {
        let t = self.elapsed_seconds() * self.speed_multiplier;
        let pal = GalaxyPalette::from_theme(&cache.theme);
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        let origin_x: f32 = bounds.origin.x.into();
        let origin_y: f32 = bounds.origin.y.into();
        let fit_scale = (width / LOGICAL_SIZE.width).min(height / LOGICAL_SIZE.height);
        let scale = fit_scale * self.zoom;
        let translate = Point2 {
            x: origin_x + (width - LOGICAL_SIZE.width * scale) * 0.5,
            y: origin_y + (height - LOGICAL_SIZE.height * scale) * 0.5,
        };
        let project = |p: Point2| Point2 {
            x: translate.x + p.x * scale,
            y: translate.y + p.y * scale,
        };
        paint_starfield(painter, &pal, cache, t, scale, &project);
        paint_galactic_halo(painter, &pal, cache, t, scale, &project);
        paint_jet(painter, &pal, t, scale, &project);
        paint_globular_clusters(painter, &pal, cache, t, scale, &project);
        paint_disc(painter, &pal, cache, t, scale, &project);
        paint_arm_satellites(painter, &pal, cache, t, scale, &project);
        paint_bulge(painter, &pal, cache, t, scale, &project);
        paint_nucleus(painter, &pal, cache, t, scale, &project);
        paint_scanline(painter, &pal, t, scale, &project);
    }
}

fn paint_starfield(
    painter: &mut Painter<'_>,
    pal: &GalaxyPalette,
    cache: &GalaxyParticleCache,
    t: f32,
    scale: f32,
    project: &impl Fn(Point2) -> Point2,
) {
    for particle in &cache.starfield {
        let seed = particle.seed;
        let phase = noise(seed, 19.0) * std::f32::consts::TAU;
        let twinkle = ((t * 1.2 + phase).sin() * 0.5 + 0.5).powf(1.4);
        let alpha = (0.05 + twinkle * 0.40).clamp(0.0, 1.0);
        let size = scale.max(1.0) * (0.9 + noise(seed, 31.0) * 1.1);
        let color = pal.starfield;

        let p = project(Point2 {
            x: snap(particle.x),
            y: snap(particle.y),
        });
        painter.fill_rectangle(
            Point2 {
                x: p.x - size * 0.5,
                y: p.y - size * 0.5,
            },
            Size2 {
                width: size,
                height: size,
            },
            with_alpha(color, alpha),
        );
    }
}

fn paint_galactic_halo(
    painter: &mut Painter<'_>,
    pal: &GalaxyPalette,
    cache: &GalaxyParticleCache,
    t: f32,
    scale: f32,
    project: &impl Fn(Point2) -> Point2,
) {
    let cool = pal.halo;
    let center = Point2 {
        x: LOGICAL_SIZE.width * 0.5,
        y: LOGICAL_SIZE.height * 0.5,
    };

    for particle in &cache.halo {
        let seed = particle.seed;
        let angle = particle.angle_offset + t * particle.rotation_rate;

        let p = Point2 {
            x: center.x + angle.cos() * DISC_RADIUS * particle.radial + particle.jitter_x,
            y: center.y
                + angle.sin() * DISC_RADIUS * particle.radial * DISC_TILT
                + particle.jitter_y,
        };

        let twinkle = ((t * 0.7 + particle.phase).sin() * 0.5 + 0.5) * 0.18;
        let alpha = 0.06 + twinkle;
        let size = (1.0 + noise(seed, 53.0) * 1.6) * scale;
        let color = pal.radial_tint(cool, particle.radial, 0.7);

        let projected = project(p);
        painter.fill_rectangle(
            Point2 {
                x: projected.x - size * 0.5,
                y: projected.y - size * 0.5,
            },
            Size2 {
                width: size,
                height: size,
            },
            with_alpha(color, alpha),
        );
    }
}

fn paint_jet(
    painter: &mut Painter<'_>,
    pal: &GalaxyPalette,
    t: f32,
    scale: f32,
    project: &impl Fn(Point2) -> Point2,
) {
    let warm = pal.nucleus;
    let cool = pal.jet;
    let center = Point2 {
        x: LOGICAL_SIZE.width * 0.5,
        y: LOGICAL_SIZE.height * 0.5,
    };

    let swell = ((t * 0.35).sin() * 0.5 + 0.5).powf(1.4);
    let intensity = 0.35 + swell * 0.55;

    let inner = 12.0;
    let outer = 96.0;

    for direction in [-1.0, 1.0] {
        for s in 0..JET_SEGMENTS {
            let f = s as f32 / (JET_SEGMENTS - 1) as f32;
            let r = inner + (outer - inner) * f;

            let wobble = (t * 0.6 + f * 6.0 + direction).sin() * (1.0 + f * 3.5);
            let raw_x = center.x + wobble;
            let raw_y = center.y + direction * r;

            let p = project(Point2 {
                x: snap(raw_x),
                y: snap(raw_y),
            });

            let falloff = (1.0 - f).powf(1.1);
            let alpha = (intensity * falloff * 0.55).clamp(0.0, 1.0);
            let width = (3.0 - f * 1.6).max(1.0) * scale;
            let height = (2.0 + falloff * 1.8) * scale;

            // Use blue galaxy tint for jets (no sun accent)
            let base_colour = blend(warm, cool, f * 0.85);
            let colour = pal.radial_tint(base_colour, 0.8, 0.7);

            painter.fill_rectangle(
                Point2 {
                    x: p.x - width * 0.5,
                    y: p.y - height * 0.5,
                },
                Size2 { width, height },
                with_alpha(colour, alpha),
            );

            if f < 0.55 {
                let side_size = scale.max(1.0);
                for off in [-1.0, 1.0] {
                    painter.fill_rectangle(
                        Point2 {
                            x: p.x + off * width * 0.65 - side_size * 0.5,
                            y: p.y - side_size * 0.5,
                        },
                        Size2 {
                            width: side_size,
                            height: side_size,
                        },
                        with_alpha(colour, alpha * 0.5),
                    );
                }
            }
        }
    }
}

fn paint_globular_clusters(
    painter: &mut Painter<'_>,
    pal: &GalaxyPalette,
    cache: &GalaxyParticleCache,
    t: f32,
    scale: f32,
    project: &impl Fn(Point2) -> Point2,
) {
    let center = Point2 {
        x: LOGICAL_SIZE.width * 0.5,
        y: LOGICAL_SIZE.height * 0.5,
    };

    for particle in &cache.globular {
        let seed = particle.seed;
        let angle = particle.angle_offset + t * particle.omega + particle.phase;

        let p = Point2 {
            x: center.x + angle.cos() * particle.orbit_radius,
            y: center.y + angle.sin() * particle.orbit_radius * particle.plane,
        };

        let twinkle = ((t * 1.3 + particle.phase).sin() * 0.5 + 0.5) * 0.42;
        let size = (1.6 + noise(seed, 23.0) * 1.8) * scale;

        let r_norm = particle.orbit_radius / DISC_RADIUS;
        let colour = pal.radial_tint(pal.arm_young, r_norm, 0.8);

        let projected = project(p);
        let glow_size = size * 1.8;
        painter.fill_rectangle(
            Point2 {
                x: projected.x - glow_size * 0.5,
                y: projected.y - glow_size * 0.5,
            },
            Size2 {
                width: glow_size,
                height: glow_size,
            },
            with_alpha(colour, 0.06 + twinkle * 0.10),
        );
        painter.fill_rectangle(
            Point2 {
                x: projected.x - size * 0.5,
                y: projected.y - size * 0.5,
            },
            Size2 {
                width: size,
                height: size,
            },
            with_alpha(colour, 0.18 + twinkle * 0.28),
        );
    }
}

fn paint_disc(
    painter: &mut Painter<'_>,
    pal: &GalaxyPalette,
    cache: &GalaxyParticleCache,
    t: f32,
    scale: f32,
    project: &impl Fn(Point2) -> Point2,
) {
    let center = Point2 {
        x: LOGICAL_SIZE.width * 0.5,
        y: LOGICAL_SIZE.height * 0.5,
    };

    for particle in &cache.disc {
        let omega = 0.18 / (0.40 + particle.r);
        let theta = particle.theta0 - t * omega;
        let nx = particle.r * theta.cos();
        let ny = particle.r * theta.sin();

        let raw_x = center.x + nx * DISC_RADIUS;
        let raw_y = center.y + ny * DISC_RADIUS * DISC_TILT;

        let drift_x = (t * 0.5 + particle.phase).sin() * 0.9;
        let drift_y = (t * 0.4 + particle.phase * 1.3).cos() * 0.6;

        let block_x = snap(raw_x + drift_x);
        let block_y = snap(raw_y + drift_y);

        let colour = pal.radial_tint(particle.base_colour, particle.r, 0.45);

        let shimmer = ((t * 1.4 + particle.shimmer_phase).sin() * 0.5 + 0.5)
            * (0.18 + particle.energy * 0.18);
        let base_alpha = 0.22 + particle.energy * 0.65;
        let alpha = (base_alpha * (0.78 + shimmer)).clamp(0.05, 1.0);

        let pulse = (t * 1.0 + particle.pulse_phase).sin() * 0.5 + 0.5;
        let block_size = (2.6 + particle.energy * 4.2 + pulse * 0.7).clamp(2.0, 8.0);

        let projected = project(Point2 {
            x: block_x,
            y: block_y,
        });
        let size = block_size * scale;
        painter.fill_rectangle(
            Point2 {
                x: projected.x - size * 0.5,
                y: projected.y - size * 0.5,
            },
            Size2 {
                width: size,
                height: size,
            },
            with_alpha(colour, alpha),
        );

        if particle.has_highlight {
            let hi_size = (size * 0.32).max(scale * 1.2);
            let hot = blend(
                colour,
                Rgba {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
                0.55,
            );
            painter.fill_rectangle(
                Point2 {
                    x: projected.x - size * 0.5 + hi_size * 0.4,
                    y: projected.y - size * 0.5 + hi_size * 0.4,
                },
                Size2 {
                    width: hi_size,
                    height: hi_size,
                },
                with_alpha(hot, (alpha * 0.85).clamp(0.0, 1.0)),
            );
        }
    }
}

fn paint_arm_satellites(
    painter: &mut Painter<'_>,
    pal: &GalaxyPalette,
    cache: &GalaxyParticleCache,
    t: f32,
    scale: f32,
    project: &impl Fn(Point2) -> Point2,
) {
    let center = Point2 {
        x: LOGICAL_SIZE.width * 0.5,
        y: LOGICAL_SIZE.height * 0.5,
    };

    for particle in &cache.satellites {
        let theta_arm = ARM_PITCH * (1.0 + particle.r * 6.0).ln()
            + particle.arm_index * std::f32::consts::TAU / ARM_COUNT as f32
            + particle.arm_jitter
            - t * particle.omega;

        let nx = particle.r * theta_arm.cos();
        let ny = particle.r * theta_arm.sin();

        let raw_x = center.x + nx * DISC_RADIUS;
        let raw_y = center.y + ny * DISC_RADIUS * DISC_TILT;

        let twinkle = ((t * 1.3 + particle.phase).sin() * 0.5 + 0.5) * 0.55;
        let colour = pal.radial_tint(particle.base_colour, particle.r, 0.6);
        let alpha = (0.30 + twinkle * 0.45).clamp(0.0, 1.0);
        let size = (2.2 + (1.0 - particle.r) * 2.0) * scale;

        let projected = project(Point2 {
            x: snap(raw_x),
            y: snap(raw_y),
        });
        let glow_size = size * 1.7;
        painter.fill_rectangle(
            Point2 {
                x: projected.x - glow_size * 0.5,
                y: projected.y - glow_size * 0.5,
            },
            Size2 {
                width: glow_size,
                height: glow_size,
            },
            with_alpha(colour, alpha * 0.30),
        );
        painter.fill_rectangle(
            Point2 {
                x: projected.x - size * 0.5,
                y: projected.y - size * 0.5,
            },
            Size2 {
                width: size,
                height: size,
            },
            with_alpha(colour, alpha),
        );
    }
}

fn paint_bulge(
    painter: &mut Painter<'_>,
    pal: &GalaxyPalette,
    cache: &GalaxyParticleCache,
    t: f32,
    scale: f32,
    project: &impl Fn(Point2) -> Point2,
) {
    let warm = pal.bulge;
    let center = Point2 {
        x: LOGICAL_SIZE.width * 0.5,
        y: LOGICAL_SIZE.height * 0.5,
    };

    let breath = (t * 0.6).sin() * 0.5 + 0.5;
    let breath_alpha = 0.55 + breath * 0.30;

    for particle in &cache.bulge {
        let r = (particle.nx * particle.nx + particle.ny * particle.ny).sqrt();
        let drift_x = (t * 0.7 + particle.phase).sin() * 0.7;
        let drift_y = (t * 0.5 + particle.phase * 1.3).cos() * 0.5;

        let raw_x = center.x + particle.nx * 38.0 + drift_x;
        let raw_y = center.y + particle.ny * 32.0 + drift_y;

        let block_x = snap(raw_x);
        let block_y = snap(raw_y);

        let r_norm = r / 0.38;
        let tinted = pal.radial_tint(warm, r_norm, 0.7);
        let hot = blend(tinted, Rgba::WHITE, (1.0 - r * 1.3).clamp(0.0, 0.6));

        let shimmer = (t * 1.6 + particle.shimmer_phase).sin() * 0.5 + 0.5;
        let alpha = (breath_alpha * particle.density * (0.7 + shimmer * 0.3)).clamp(0.06, 0.95);
        let size = (2.4 + particle.density * 3.4) * scale;

        let projected = project(Point2 {
            x: block_x,
            y: block_y,
        });
        painter.fill_rectangle(
            Point2 {
                x: projected.x - size * 0.5,
                y: projected.y - size * 0.5,
            },
            Size2 {
                width: size,
                height: size,
            },
            with_alpha(hot, alpha),
        );
    }
}

fn paint_nucleus(
    painter: &mut Painter<'_>,
    pal: &GalaxyPalette,
    cache: &GalaxyParticleCache,
    t: f32,
    scale: f32,
    project: &impl Fn(Point2) -> Point2,
) {
    let warm = pal.nucleus;
    let tinted = pal.radial_tint(warm, 0.1, 0.85);
    let hot = blend(
        tinted,
        Rgba {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        },
        0.65,
    );
    let center = Point2 {
        x: LOGICAL_SIZE.width * 0.5,
        y: LOGICAL_SIZE.height * 0.5,
    };

    let breath = (t * 0.85).sin() * 0.5 + 0.5;
    let breath_alpha = 0.65 + breath * 0.30;

    for particle in &cache.nucleus {
        let drift_x = (t * 0.9 + particle.phase).sin() * 0.6;
        let drift_y = (t * 0.7 + particle.phase * 1.3).cos() * 0.5;

        let raw_x = center.x + particle.nx * 18.0 + drift_x;
        let raw_y = center.y + particle.ny * 14.0 + drift_y;

        let shimmer = (t * 2.4 + particle.shimmer_phase).sin() * 0.5 + 0.5;
        let alpha = (breath_alpha * (0.7 + shimmer * 0.3)).clamp(0.2, 1.0);
        let size = (2.4 + particle.density * 3.0) * scale;

        let projected = project(Point2 {
            x: snap(raw_x),
            y: snap(raw_y),
        });
        painter.fill_rectangle(
            Point2 {
                x: projected.x - size * 0.5,
                y: projected.y - size * 0.5,
            },
            Size2 {
                width: size,
                height: size,
            },
            with_alpha(hot, alpha),
        );
    }
}

fn paint_scanline(
    painter: &mut Painter<'_>,
    pal: &GalaxyPalette,
    t: f32,
    scale: f32,
    project: &impl Fn(Point2) -> Point2,
) {
    let accent = pal.core;

    let cycle = 8.0;
    let phase = ((t / cycle) - (t / cycle).floor()).clamp(0.0, 1.0);
    let band_y = phase * LOGICAL_SIZE.height;

    let cols = 60;
    for c in 0..cols {
        let fx = c as f32 / (cols - 1) as f32;
        let x = fx * LOGICAL_SIZE.width;
        let jitter = (noise(c as f32, 3.0) - 0.5) * 1.2;
        let p = project(Point2 {
            x: snap(x),
            y: snap(band_y + jitter),
        });
        let edge = (1.0 - (fx - 0.5).abs() * 1.8).clamp(0.0, 1.0);
        let alpha = 0.06 * edge;
        let size = scale * 1.6;

        painter.fill_rectangle(
            Point2 {
                x: p.x - size * 0.5,
                y: p.y - size * 0.5,
            },
            Size2 {
                width: size,
                height: size,
            },
            with_alpha(accent, alpha),
        );
    }
}

fn arm_density(x: f32, y: f32, t: f32) -> (f32, f32) {
    let r = (x * x + y * y).sqrt();
    if r < 1e-3 {
        return (0.0, 0.0);
    }

    let theta = y.atan2(x);
    let omega = 0.18 / (0.40 + r);
    let theta_r = theta + t * omega;

    let arm_phase = theta_r - ARM_PITCH * (1.0 + r * 6.0).ln();
    let arm_n = ARM_COUNT as f32;
    let wrapped = wrap_pi(arm_phase * arm_n) / arm_n;
    let arm_distance = (wrapped.abs() / (std::f32::consts::PI / arm_n)).clamp(0.0, 1.0);

    let arm_strength = (-(wrapped / ARM_WIDTH).powi(2) * 4.0).exp();

    let envelope = (-((r - 0.55) / 0.32).powi(2)).exp() * 0.85 + (1.0 - r).max(0.0).powi(2) * 0.25;

    let density = (arm_strength * envelope).clamp(0.0, 1.0);
    (density, arm_distance)
}

fn wrap_pi(angle: f32) -> f32 {
    let two_pi = std::f32::consts::TAU;
    let mut a = angle % two_pi;
    if a > std::f32::consts::PI {
        a -= two_pi;
    } else if a <= -std::f32::consts::PI {
        a += two_pi;
    }
    a
}

fn gaussian2d(x: f32, y: f32, sigma_x: f32, sigma_y: f32) -> f32 {
    (-0.5 * ((x / sigma_x).powi(2) + (y / sigma_y).powi(2))).exp()
}

fn noise(value: f32, seed: f32) -> f32 {
    let mixed = (value * 12.9898 + seed * 78.233).sin() * 43_758.547;
    mixed - mixed.floor()
}

fn snap(value: f32) -> f32 {
    (value / SNAP_GRID).round() * SNAP_GRID
}

fn rgba_from_theme(color: ThemeRgba) -> Rgba {
    Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::theme::ThemeMode;

    #[test]
    fn particle_cache_bake_is_deterministic_for_theme() {
        let theme = OpenCoreTheme::resolve(ThemeMode::Dark);
        let a = GalaxyParticleCache::bake(theme);
        let b = GalaxyParticleCache::bake(theme);
        assert_eq!(a.disc_len(), b.disc_len());
        assert_eq!(a.halo_len(), b.halo_len());
        assert_eq!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn particle_cache_disc_count_matches_target() {
        let theme = OpenCoreTheme::resolve(ThemeMode::Dark);
        let cache = GalaxyParticleCache::bake(theme);
        assert_eq!(cache.disc_len(), DISC_STAR_COUNT);
    }

    #[test]
    fn particle_cache_rebuilds_when_theme_changes() {
        let dark = GalaxyParticleCache::bake(OpenCoreTheme::resolve(ThemeMode::Dark));
        let light = GalaxyParticleCache::bake(OpenCoreTheme::resolve(ThemeMode::Light));
        assert_ne!(dark.theme().mode, light.theme().mode);
        assert_ne!(dark.fingerprint(), light.fingerprint());
    }

    #[test]
    fn noise_in_unit_range() {
        for i in 0..256 {
            let n = noise(i as f32, (i as f32) * 1.7);
            assert!((0.0..=1.0).contains(&n));
        }
    }

    #[test]
    fn noise_deterministic() {
        let a = noise(7.0, 13.0);
        let b = noise(7.0, 13.0);
        assert_eq!(a, b);
    }
}
