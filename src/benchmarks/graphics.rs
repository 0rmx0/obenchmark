use std::hint::black_box;
use std::time::{Duration, Instant};

use anyhow::Result;
use image::GenericImageView;

use crate::engines::benchmark::Benchmark;
use crate::model::result::SampleResult;

/// Durée de la phase d'échauffement (non mesurée) exécutée avant chaque
/// série d'échantillons.
const WARMUP_DURATION: Duration = Duration::from_millis(500);

/// Durée d'un échantillon de mesure individuel.
const SAMPLE_DURATION: Duration = Duration::from_secs(2);

/// Nombre d'échantillons indépendants dont la médiane constitue le score
/// final d'un test.
const SAMPLE_COUNT: usize = 5;

const MIN_ELAPSED_SEC: f64 = 1e-6;

// Texture embarquée à la compilation : la trame "grain" déjà utilisée par la
// charte graphique R_M_X. Asset maison, pas de dépendance à du contenu tiers.
const GRAIN_TEXTURE_BYTES: &[u8] = include_bytes!("../../grain.png");

/// 1. Benchmark 2D — décodage + compositing alpha en rafale.
///
/// Simule une charge de travail de type "rendu 2D" : décodage d'image,
/// upscale/downscale, puis compositing alpha-blend sur un framebuffer,
/// mesuré en mégapixels composités par seconde.
pub struct Gfx2DRaster;
impl Benchmark for Gfx2DRaster {
    fn name(&self) -> &str {
        "GFX 2D Raster"
    }

    fn weight(&self) -> u64 {
        3
    }

    fn run(&self) -> Result<SampleResult> {
        let source = image::load_from_memory(GRAIN_TEXTURE_BYTES)?;
        let (src_w, src_h) = source.dimensions();
        let src_rgba = source.to_rgba8();

        const FB_W: u32 = 960;
        const FB_H: u32 = 540;

        // Phase d'échauffement
        let mut framebuffer = vec![0u8; (FB_W * FB_H * 4) as usize];
        let warmup_start = Instant::now();
        let mut frame: u32 = 0;
        
        while warmup_start.elapsed() < WARMUP_DURATION {
            let offset_x = (frame.wrapping_mul(7)) % src_w.max(1);
            let offset_y = (frame.wrapping_mul(13)) % src_h.max(1);

            for y in 0..FB_H {
                let sy = (y + offset_y) % src_h.max(1);
                for x in 0..FB_W {
                    let sx = (x + offset_x) % src_w.max(1);
                    let px = src_rgba.get_pixel(sx, sy);

                    let idx = ((y * FB_W + x) * 4) as usize;
                    let alpha = px[3] as u32;
                    let inv_alpha = 255 - alpha;

                    for c in 0..3 {
                        let src_c = px[c] as u32;
                        let dst_c = framebuffer[idx + c] as u32;
                        framebuffer[idx + c] = ((src_c * alpha + dst_c * inv_alpha) / 255) as u8;
                    }
                    framebuffer[idx + 3] = 255;
                }
            }
            frame = frame.wrapping_add(1);
        }

        // Échantillonnage
        let mut raw_samples: Vec<u64> = Vec::with_capacity(SAMPLE_COUNT);
        for _ in 0..SAMPLE_COUNT {
            let mut framebuffer = vec![0u8; (FB_W * FB_H * 4) as usize];
            let start = Instant::now();
            let mut pixels_composited: u64 = 0;
            let mut frame: u32 = 0;

            while start.elapsed() < SAMPLE_DURATION {
                let offset_x = (frame.wrapping_mul(7)) % src_w.max(1);
                let offset_y = (frame.wrapping_mul(13)) % src_h.max(1);

                for y in 0..FB_H {
                    let sy = (y + offset_y) % src_h.max(1);
                    for x in 0..FB_W {
                        let sx = (x + offset_x) % src_w.max(1);
                        let px = src_rgba.get_pixel(sx, sy);

                        let idx = ((y * FB_W + x) * 4) as usize;
                        let alpha = px[3] as u32;
                        let inv_alpha = 255 - alpha;

                        for c in 0..3 {
                            let src_c = px[c] as u32;
                            let dst_c = framebuffer[idx + c] as u32;
                            framebuffer[idx + c] = ((src_c * alpha + dst_c * inv_alpha) / 255) as u8;
                        }
                        framebuffer[idx + 3] = 255;
                    }
                }

                pixels_composited += (FB_W * FB_H) as u64;
                frame = frame.wrapping_add(1);
            }

            black_box(&framebuffer);
            let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
            let megapixels_per_sec = (pixels_composited as f64 / 1_000_000.0) / elapsed;
            raw_samples.push(megapixels_per_sec as u64);
        }

        Ok(SampleResult::from_samples(raw_samples))
    }
}

// --- Rastérisation 3D logicielle -------------------------------------------

#[derive(Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

/// Génère un maillage procédural en forme de "cristal hérissé" (pics
/// aléatoires sur une sphère) — silhouette qui colle à l'esthétique
/// horror punk / blackwork de R_M_X, sans dépendre d'un asset externe.
fn generate_spiky_mesh(segments: u32) -> Vec<[Vec3; 3]> {
    let mut triangles = Vec::new();
    let rings = segments;
    let sectors = segments * 2;

    let mut ring_points: Vec<Vec<Vec3>> = Vec::with_capacity((rings + 1) as usize);
    for r in 0..=rings {
        let phi = std::f32::consts::PI * (r as f32) / (rings as f32); // 0..PI
        let mut row = Vec::with_capacity((sectors + 1) as usize);
        for s in 0..=sectors {
            let theta = 2.0 * std::f32::consts::PI * (s as f32) / (sectors as f32);

            // Bruit pseudo-aléatoire déterministe (pas de dépendance rand ici)
            // pour hérisser la sphère de pics irréguliers.
            let seed = (r * 928_371 + s * 68_921) as f32;
            let spike = 0.15 * ((seed.sin() * 43758.5453).fract());
            let radius = 1.0 + spike.abs();

            let x = radius * phi.sin() * theta.cos();
            let y = radius * phi.cos();
            let z = radius * phi.sin() * theta.sin();
            row.push(Vec3::new(x, y, z));
        }
        ring_points.push(row);
    }

    for r in 0..rings as usize {
        for s in 0..sectors as usize {
            let a = ring_points[r][s];
            let b = ring_points[r + 1][s];
            let c = ring_points[r + 1][s + 1];
            let d = ring_points[r][s + 1];
            triangles.push([a, b, c]);
            triangles.push([a, c, d]);
        }
    }

    triangles
}

fn rotate_y(v: Vec3, angle: f32) -> Vec3 {
    let (s, c) = angle.sin_cos();
    Vec3::new(v.x * c + v.z * s, v.y, -v.x * s + v.z * c)
}

fn project(v: Vec3, width: f32, height: f32) -> (i32, i32, f32) {
    let dist = 4.0;
    let z = v.z + dist;
    let fov = 1.4;
    let sx = (v.x * fov / z) * (width * 0.4) + width / 2.0;
    let sy = (-v.y * fov / z) * (height * 0.4) + height / 2.0;
    (sx as i32, sy as i32, z)
}

/// 2. Benchmark 3D — transformation de sommets + rastérisation de triangles
/// avec z-buffer, en logiciel pur (aucune dépendance GPU/driver, pour rester
/// cohérent avec le reste de la suite qui tourne "headless").
///
/// Mesure en triangles rastérisés par seconde.
pub struct Gfx3DRaster;
impl Benchmark for Gfx3DRaster {
    fn name(&self) -> &str {
        "GFX 3D Raster"
    }

    fn weight(&self) -> u64 {
        3
    }

    fn run(&self) -> Result<SampleResult> {
        let mesh = generate_spiky_mesh(48); // ~9k triangles
        const W: usize = 480;
        const H: usize = 270;

        // Phase d'échauffement
        let mut color_buf = vec![0u8; W * H];
        let mut depth_buf = vec![f32::INFINITY; W * H];
        let warmup_start = Instant::now();
        let mut angle: f32 = 0.0;
        
        while warmup_start.elapsed() < WARMUP_DURATION {
            for px in depth_buf.iter_mut() {
                *px = f32::INFINITY;
            }
            for px in color_buf.iter_mut() {
                *px = 0;
            }

            for tri in &mesh {
                let a = rotate_y(tri[0], angle);
                let b = rotate_y(tri[1], angle);
                let c = rotate_y(tri[2], angle);

                let (ax, ay, az) = project(a, W as f32, H as f32);
                let (bx, by, bz) = project(b, W as f32, H as f32);
                let (cx, cy, cz) = project(c, W as f32, H as f32);

                rasterize_triangle(
                    &mut color_buf,
                    &mut depth_buf,
                    W,
                    H,
                    (ax, ay, az),
                    (bx, by, bz),
                    (cx, cy, cz),
                );
            }
            angle += 0.05;
        }

        // Échantillonnage
        let mut raw_samples: Vec<u64> = Vec::with_capacity(SAMPLE_COUNT);
        for _ in 0..SAMPLE_COUNT {
            let mut color_buf = vec![0u8; W * H];
            let mut depth_buf = vec![f32::INFINITY; W * H];
            let start = Instant::now();
            let mut triangles_rendered: u64 = 0;
            let mut angle: f32 = 0.0;

            while start.elapsed() < SAMPLE_DURATION {
                for px in depth_buf.iter_mut() {
                    *px = f32::INFINITY;
                }
                for px in color_buf.iter_mut() {
                    *px = 0;
                }

                for tri in &mesh {
                    let a = rotate_y(tri[0], angle);
                    let b = rotate_y(tri[1], angle);
                    let c = rotate_y(tri[2], angle);

                    let (ax, ay, az) = project(a, W as f32, H as f32);
                    let (bx, by, bz) = project(b, W as f32, H as f32);
                    let (cx, cy, cz) = project(c, W as f32, H as f32);

                    rasterize_triangle(
                        &mut color_buf,
                        &mut depth_buf,
                        W,
                        H,
                        (ax, ay, az),
                        (bx, by, bz),
                        (cx, cy, cz),
                    );
                    triangles_rendered += 1;
                }

                angle += 0.05;
            }

            black_box(&color_buf);
            let elapsed = start.elapsed().as_secs_f64().max(MIN_ELAPSED_SEC);
            raw_samples.push((triangles_rendered as f64 / elapsed) as u64);
        }

        Ok(SampleResult::from_samples(raw_samples))
    }
}

#[allow(clippy::too_many_arguments)]
fn rasterize_triangle(
    color_buf: &mut [u8],
    depth_buf: &mut [f32],
    w: usize,
    h: usize,
    a: (i32, i32, f32),
    b: (i32, i32, f32),
    c: (i32, i32, f32),
) {
    let min_x = a.0.min(b.0).min(c.0).max(0);
    let max_x = a.0.max(b.0).max(c.0).min(w as i32 - 1);
    let min_y = a.1.min(b.1).min(c.1).max(0);
    let max_y = a.1.max(b.1).max(c.1).min(h as i32 - 1);

    if min_x > max_x || min_y > max_y {
        return;
    }

    let edge = |x0: i32, y0: i32, x1: i32, y1: i32, px: i32, py: i32| -> i32 {
        (x1 - x0) * (py - y0) - (y1 - y0) * (px - x0)
    };

    let area = edge(a.0, a.1, b.0, b.1, c.0, c.1);
    if area == 0 {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let w0 = edge(b.0, b.1, c.0, c.1, x, y);
            let w1 = edge(c.0, c.1, a.0, a.1, x, y);
            let w2 = edge(a.0, a.1, b.0, b.1, x, y);

            let inside = (w0 >= 0 && w1 >= 0 && w2 >= 0) || (w0 <= 0 && w1 <= 0 && w2 <= 0);
            if !inside {
                continue;
            }

            let inv_area = 1.0 / area as f32;
            let l0 = w0 as f32 * inv_area;
            let l1 = w1 as f32 * inv_area;
            let l2 = w2 as f32 * inv_area;
            let z = l0 * a.2 + l1 * b.2 + l2 * c.2;

            let idx = (y as usize) * w + (x as usize);
            if z < depth_buf[idx] {
                depth_buf[idx] = z;
                // Ombrage minimal basé sur la profondeur, suffisant pour
                // garantir que le compilateur ne peut pas éliminer le calcul.
                color_buf[idx] = (255.0 / (1.0 + z.max(0.0))) as u8;
            }
        }
    }
}
