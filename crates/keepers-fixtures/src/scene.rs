//! The base scene and its exposure variant, in integer maths.
//!
//! The scene is seeded value noise in three octaves plus twelve hard-edged rectangles and
//! circles, so it has both texture (for sharpness measures) and edges (for blur to act on). The
//! only transcendental values, the sRGB curve and the exposure factor, come from `libm` and are
//! quantised to 16-bit fixed point, so the output is bit-identical on every platform.

use image::{Rgb, RgbImage};

/// A tiny seeded generator with no dependency: SplitMix64 (Steele, Lea and Flood, 2014).
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..n`. The modulo bias is irrelevant for fixtures and keeps the maths integer.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0, "below(0) has no answer");
        (self.next_u64() % u64::from(n)) as u32
    }
}

/// Folds a set's name into the seed (FNV-1a), so every set has its own stream and adding a set
/// never changes the bytes of another.
pub fn set_seed(seed: u64, set: &str) -> u64 {
    let mut hash: u64 = 0xCBF2_9CE4_8422_2325;
    for byte in set.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash ^ seed
}

/// Cell size in pixels and weight of each noise octave; the weights sum to 7.
const OCTAVES: [(u32, u32); 3] = [(128, 4), (32, 2), (8, 1)];
const OCTAVE_WEIGHT_SUM: u64 = 7;
const SHAPES: u32 = 12;

/// Linear interpolation between two Q16 values with a Q16 weight.
fn lerp_q16(a: i64, b: i64, t: i64) -> i64 {
    (a * (65_536 - t) + b * t) >> 16
}

/// The base scene: three octaves of value noise per channel, then twelve hard-edged shapes.
pub fn base_scene(width: u32, height: u32, rng: &mut Rng) -> RgbImage {
    let mut sum = vec![0u64; (width * height * 3) as usize];
    for (cell, weight) in OCTAVES {
        let lattice_w = width / cell + 2;
        let lattice_h = height / cell + 2;
        for channel in 0..3 {
            let lattice: Vec<i64> = (0..lattice_w * lattice_h)
                .map(|_| i64::from(rng.below(256)) << 16)
                .collect();
            for y in 0..height {
                let cy = y / cell;
                let ty = i64::from((y % cell) * 65_536 / cell);
                for x in 0..width {
                    let cx = x / cell;
                    let tx = i64::from((x % cell) * 65_536 / cell);
                    let at = |dx: u32, dy: u32| lattice[((cy + dy) * lattice_w + cx + dx) as usize];
                    let top = lerp_q16(at(0, 0), at(1, 0), tx);
                    let bottom = lerp_q16(at(0, 1), at(1, 1), tx);
                    let value = lerp_q16(top, bottom, ty);
                    sum[((y * width + x) * 3 + channel) as usize] +=
                        value as u64 * u64::from(weight);
                }
            }
        }
    }
    let scale = OCTAVE_WEIGHT_SUM << 16;
    let pixels: Vec<u8> = sum
        .iter()
        .map(|&s| ((s + scale / 2) / scale).min(255) as u8)
        .collect();
    let mut img = RgbImage::from_raw(width, height, pixels).expect("buffer size matches");

    for i in 0..SHAPES {
        let colour = Rgb([
            rng.below(256) as u8,
            rng.below(256) as u8,
            rng.below(256) as u8,
        ]);
        if i.is_multiple_of(2) {
            // Clamped so a shape always fits; the clamps never trigger on the full-size scene.
            let w = (64 + rng.below(257)).min(width);
            let h = (64 + rng.below(257)).min(height);
            let x0 = rng.below(width - w + 1);
            let y0 = rng.below(height - h + 1);
            for y in y0..y0 + h {
                for x in x0..x0 + w {
                    img.put_pixel(x, y, colour);
                }
            }
        } else {
            // A circle spans 2r + 1 pixels, so its centre has width - 2r valid columns.
            let r = (32 + rng.below(129))
                .min((width - 1) / 2)
                .min((height - 1) / 2);
            let cx = r + rng.below(width - 2 * r);
            let cy = r + rng.below(height - 2 * r);
            let r2 = i64::from(r) * i64::from(r);
            for dy in -i64::from(r)..=i64::from(r) {
                for dx in -i64::from(r)..=i64::from(r) {
                    if dx * dx + dy * dy <= r2 {
                        img.put_pixel(
                            (i64::from(cx) + dx) as u32,
                            (i64::from(cy) + dy) as u32,
                            colour,
                        );
                    }
                }
            }
        }
    }
    img
}

/// The sRGB transfer function (IEC 61966-2-1): 8-bit code to linear light in Q16 (0..=65536).
fn srgb_to_linear_q16() -> [u32; 256] {
    let mut table = [0u32; 256];
    for (code, entry) in table.iter_mut().enumerate() {
        let c = code as f64 / 255.0;
        let linear = if c <= 0.04045 {
            c / 12.92
        } else {
            libm::pow((c + 0.055) / 1.055, 2.4)
        };
        *entry = libm::round(linear * 65_536.0) as u32;
    }
    table
}

/// Shifts exposure by `ev_milli` thousandths of a stop in linear light, clipping at white.
pub fn exposure_shift(img: &RgbImage, ev_milli: i32) -> RgbImage {
    let table = srgb_to_linear_q16();
    let factor = libm::round(libm::exp2(f64::from(ev_milli) / 1_000.0) * 65_536.0) as u64;
    let to_code = |linear: u32| -> u8 {
        // The nearest code by linear value; ties go to the lower code.
        let upper = table.partition_point(|&t| t < linear).min(255);
        let lower = upper.saturating_sub(1);
        let d_upper = table[upper].abs_diff(linear);
        let d_lower = table[lower].abs_diff(linear);
        if d_lower <= d_upper {
            lower as u8
        } else {
            upper as u8
        }
    };
    let pixels: Vec<u8> = img
        .as_raw()
        .iter()
        .map(|&code| {
            let linear = (u64::from(table[code as usize]) * factor) >> 16;
            to_code(linear.min(65_536) as u32)
        })
        .collect();
    RgbImage::from_raw(img.width(), img.height(), pixels).expect("buffer size matches")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_is_deterministic_and_set_streams_differ() {
        let a: Vec<u64> = (0..4).map(|_| Rng::new(1).next_u64()).collect();
        let b: Vec<u64> = (0..4).map(|_| Rng::new(1).next_u64()).collect();
        assert_eq!(a, b);
        assert_ne!(set_seed(1, "basic"), set_seed(1, "exif"));
        assert_ne!(set_seed(1, "basic"), set_seed(2, "basic"));
        let mut rng = Rng::new(7);
        for _ in 0..1_000 {
            assert!(rng.below(10) < 10);
        }
    }

    #[test]
    fn base_scene_is_reproducible_and_textured() {
        let a = base_scene(256, 128, &mut Rng::new(set_seed(1, "basic")));
        let b = base_scene(256, 128, &mut Rng::new(set_seed(1, "basic")));
        assert_eq!(a.as_raw(), b.as_raw());
        let distinct: std::collections::BTreeSet<u8> = a.as_raw().iter().copied().collect();
        assert!(distinct.len() > 64, "the scene should use many values");
        let c = base_scene(256, 128, &mut Rng::new(set_seed(2, "basic")));
        assert_ne!(a.as_raw(), c.as_raw());
    }

    #[test]
    fn srgb_table_ends_match_the_standard() {
        let table = srgb_to_linear_q16();
        assert_eq!(table[0], 0);
        assert_eq!(table[255], 65_536);
        // Code 128 is 0.2158 in linear light (IEC 61966-2-1), 14,146 in Q16 after rounding.
        assert!((14_100..=14_200).contains(&table[128]), "{}", table[128]);
        assert!(table.windows(2).all(|w| w[0] < w[1]), "monotonic");
    }

    #[test]
    fn exposure_shift_of_zero_is_identity_and_plus_one_brightens() {
        let scene = base_scene(64, 32, &mut Rng::new(3));
        let same = exposure_shift(&scene, 0);
        assert_eq!(same.as_raw(), scene.as_raw());
        let brighter = exposure_shift(&scene, 1_000);
        assert!(
            brighter
                .as_raw()
                .iter()
                .zip(scene.as_raw())
                .all(|(b, s)| b >= s)
        );
        assert!(
            brighter
                .as_raw()
                .iter()
                .zip(scene.as_raw())
                .any(|(b, s)| b > s)
        );
        // Mid grey (code 128, 0.2158 linear, 14,147 in Q16) doubled is 28,294 in Q16; the
        // nearest code by the IEC 61966-2-1 curve is 176 (28,453), not 175 (28,095).
        let grey = RgbImage::from_pixel(1, 1, Rgb([128, 128, 128]));
        assert_eq!(
            exposure_shift(&grey, 1_000).get_pixel(0, 0).0,
            [176, 176, 176]
        );
        let white = RgbImage::from_pixel(1, 1, Rgb([255, 255, 255]));
        assert_eq!(
            exposure_shift(&white, 1_000).get_pixel(0, 0).0,
            [255, 255, 255]
        );
        let darker = exposure_shift(&white, -1_000);
        assert_eq!(darker.get_pixel(0, 0).0, [188, 188, 188]);
    }
}
