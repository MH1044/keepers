//! Gaussian blur with a kernel quantised to 16-bit fixed point.
//!
//! The taps come from `libm::exp` (pure Rust, the same on every platform) and are rounded so
//! they sum to exactly 65,536; the convolution itself is integer maths with edge clamping.

use image::RgbImage;

/// The 1-D Gaussian taps for `sigma_milli` thousandths of a pixel, radius `ceil(3σ)`, in Q16.
/// The taps sum to exactly 65,536 (largest-remainder rounding), so a flat image stays flat.
pub fn gaussian_kernel_q16(sigma_milli: u32) -> Vec<u32> {
    if sigma_milli == 0 {
        return vec![65_536];
    }
    let sigma = f64::from(sigma_milli) / 1_000.0;
    let radius = libm::ceil(3.0 * sigma) as i64;
    let raw: Vec<f64> = (-radius..=radius)
        .map(|i| libm::exp(-((i * i) as f64) / (2.0 * sigma * sigma)))
        .collect();
    let total: f64 = raw.iter().sum();
    let scaled: Vec<f64> = raw.iter().map(|w| w / total * 65_536.0).collect();
    let mut taps: Vec<u32> = scaled.iter().map(|s| libm::floor(*s) as u32).collect();
    let mut remainder = 65_536 - taps.iter().sum::<u32>();
    let mut order: Vec<usize> = (0..taps.len()).collect();
    order.sort_by(|&a, &b| {
        let fa = scaled[a] - libm::floor(scaled[a]);
        let fb = scaled[b] - libm::floor(scaled[b]);
        fb.partial_cmp(&fa).unwrap().then(a.cmp(&b))
    });
    for &i in &order {
        if remainder == 0 {
            break;
        }
        taps[i] += 1;
        remainder -= 1;
    }
    taps
}

/// Separable Gaussian blur. The horizontal pass keeps eight fractional bits; the vertical pass
/// rounds once to 8-bit.
pub fn gaussian_blur(img: &RgbImage, sigma_milli: u32) -> RgbImage {
    let taps = gaussian_kernel_q16(sigma_milli);
    let radius = (taps.len() / 2) as i64;
    let (width, height) = img.dimensions();
    let (w, h) = (width as usize, height as usize);
    let src = img.as_raw();

    let clamped = |coordinate: i64, limit: usize| coordinate.clamp(0, limit as i64 - 1) as usize;
    let x_sources: Vec<Vec<usize>> = (0..w)
        .map(|x| {
            (0..taps.len())
                .map(|k| clamped(x as i64 + k as i64 - radius, w))
                .collect()
        })
        .collect();
    let y_sources: Vec<Vec<usize>> = (0..h)
        .map(|y| {
            (0..taps.len())
                .map(|k| clamped(y as i64 + k as i64 - radius, h))
                .collect()
        })
        .collect();

    let mut mid = vec![0u16; w * h * 3];
    for y in 0..h {
        let row = y * w;
        for x in 0..w {
            for c in 0..3 {
                let acc: u64 = taps
                    .iter()
                    .zip(&x_sources[x])
                    .map(|(&tap, &sx)| u64::from(src[(row + sx) * 3 + c]) * u64::from(tap))
                    .sum();
                mid[(row + x) * 3 + c] = ((acc + 128) >> 8) as u16;
            }
        }
    }

    let mut out = vec![0u8; w * h * 3];
    for y in 0..h {
        for x in 0..w {
            for c in 0..3 {
                let acc: u64 = taps
                    .iter()
                    .zip(&y_sources[y])
                    .map(|(&tap, &sy)| u64::from(mid[(sy * w + x) * 3 + c]) * u64::from(tap))
                    .sum();
                out[(y * w + x) * 3 + c] = ((acc + (1 << 23)) >> 24).min(255) as u8;
            }
        }
    }
    RgbImage::from_raw(width, height, out).expect("buffer size matches")
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    #[test]
    fn kernel_sums_to_one_and_is_symmetric() {
        for sigma_milli in [500, 1_000, 2_000, 4_000, 8_000] {
            let taps = gaussian_kernel_q16(sigma_milli);
            assert_eq!(taps.iter().sum::<u32>(), 65_536, "σ {sigma_milli}");
            let radius = libm::ceil(3.0 * f64::from(sigma_milli) / 1_000.0) as usize;
            assert_eq!(taps.len(), 2 * radius + 1);
            let centre = taps[radius];
            assert!(taps.iter().all(|&t| t <= centre));
            for i in 0..radius {
                assert!(
                    taps[i].abs_diff(taps[taps.len() - 1 - i]) <= 1,
                    "σ {sigma_milli}"
                );
            }
        }
        assert_eq!(gaussian_kernel_q16(0), vec![65_536]);
    }

    #[test]
    fn sigma_two_centre_tap_matches_the_gaussian() {
        // For σ = 2 the normalised centre weight is 1 / Σ exp(-i²/8) over i in -6..=6, about
        // 0.19947; in Q16 that is about 13,073.
        let taps = gaussian_kernel_q16(2_000);
        assert_eq!(taps.len(), 13);
        assert!((13_000..=13_150).contains(&taps[6]), "{}", taps[6]);
    }

    #[test]
    fn flat_image_stays_flat_and_an_edge_softens() {
        let flat = RgbImage::from_pixel(32, 16, Rgb([77, 150, 201]));
        assert_eq!(gaussian_blur(&flat, 2_000).as_raw(), flat.as_raw());

        let mut edge = RgbImage::from_pixel(64, 8, Rgb([0, 0, 0]));
        for y in 0..8 {
            for x in 32..64 {
                edge.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
        let soft = gaussian_blur(&edge, 2_000);
        assert_eq!(soft.get_pixel(0, 4).0, [0, 0, 0]);
        assert_eq!(soft.get_pixel(63, 4).0, [255, 255, 255]);
        let at_edge = soft.get_pixel(31, 4).0[0];
        assert!((60..=200).contains(&at_edge), "{at_edge}");
        assert!(soft.get_pixel(30, 4).0[0] < soft.get_pixel(31, 4).0[0]);
        assert!(soft.get_pixel(32, 4).0[0] < soft.get_pixel(33, 4).0[0]);
    }
}
