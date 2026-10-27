# 02: Technical scores

## Goal

Every frame gets sharpness, motion, exposure and noise numbers that order frames the way a person
would, computed once, cached with the scorer's version, and printed by `keepers score`. Module 05
turns them into cull components.

## Depends on

01.

## Scope

1. Luma of the working image: `Y = (54R + 183G + 19B + 128) >> 8` on 8-bit sRGB, then `Y/255` as f64.
2. `sharpness@1`: `lap_var` = population variance of the 3 × 3 Laplacian `[0 1 0; 1 −4 1; 0 1 0]` over interior
   pixels; `tile_max` = the largest `lap_var` of an 8 × 8 tile grid (tile size `floor(W/8)` ×
   `floor(H/8)`, remainder dropped); `tenengrad` = mean of `Gx² + Gy²` with 3 × 3 Sobel kernels.
3. `motion@1`: structure tensor sums `Sxx = ΣGx²`, `Syy = ΣGy²`, `Sxy = ΣGxGy`; eigenvalues λ1 ≥ λ2;
   `anisotropy = λ1 / max(λ2, 1e-12)`; dominant gradient angle `θ = ½·atan2(2Sxy, Sxx − Syy)`;
   `angle_deg` = the blur direction `(θ + 90°) mod 180`, in degrees, using `libm`.
4. `exposure@1`: `mean_luma`; `clip_high_pct` = 100 × share of pixels with max(R,G,B) ≥ 250;
   `crush_low_pct` = 100 × share with Y ≤ 5; flags `overexposed` when `clip_high_pct > 5.0`,
   `underexposed` when `crush_low_pct > 20.0` or `mean_luma < 0.10`.
5. `noise@1`: Immerkær's estimate `σ = sqrt(π/2) · Σ|Y * N| / (6 (W−2)(H−2))` with
   `N = [1 −2 1; −2 4 −2; 1 −2 1]`, on luma in 0..255 units.
6. A scorer registry: each scorer has a name and version; results go to `scores` (ARCHITECTURE §4);
   a frame is recomputed only when a scorer's row is missing or has an older version.
7. Parallel scoring with `rayon`; results are sorted by frame index before they are stored or printed.
8. `keepers score <dir> --json [--threads N] [--progress jsonl]` printing `keepers.scores/1`: per frame
   `{"index","frame_id","sharpness":{…},"motion":{…},"exposure":{…},"noise":{…}}`, plus `errors`
   and `summary {"frames","scored","recomputed"}`.
9. Frames whose preview cannot be decoded get no scores and an entry in `errors` with code
   `preview_decode_failed`: "{name}: the embedded preview could not be decoded; not scored."
10. Fixture sets `blur-ladder/`, `exposure/`, `motion/`, `dof/`.

## Files

- `crates/keepers-core/src/score/{mod.rs,working.rs,luma.rs,sharpness.rs,motion.rs,exposure.rs,noise.rs,registry.rs}`
- `crates/keepers-cli/src/commands/score.rs`
- `crates/keepers-core/tests/02_technical_scores.rs`, `tests/cases/02-technical-scores/`
- `crates/keepers-fixtures/src/{blur_ladder.rs,exposure_sets.rs,motion.rs,dof.rs}`

## Interfaces

```rust
pub trait Scorer: Sync { fn name(&self) -> &'static str; fn version(&self) -> u32;
    fn score(&self, luma: &LumaF64, rgb: &RgbImage) -> serde_json::Value; }
pub fn score_shoot(cache: &mut ShootCache, scorers: &[&dyn Scorer], opts: &ScoreOptions,
    progress: &dyn Progress, cancel: &CancelToken) -> Result<ScoreSummary, ScoreError>;
pub struct Sharpness { pub lap_var: f64, pub tile_max: f64, pub tenengrad: f64 }
pub struct Motion { pub anisotropy: f64, pub angle_deg: f64 }
pub struct Exposure { pub mean_luma: f64, pub clip_high_pct: f64, pub crush_low_pct: f64, pub flags: Vec<ExposureFlag> }
pub struct Noise { pub sigma: f64 }
```

## Out of scope

- Face-region sharpness (03); turning values into cull components or thresholds for rejection (05);
  GPU paths (08).

## Design notes

- The working image is decoded once per frame and every scorer reads the same buffer.
- Cross-platform tolerance is 1e-4 relative instead of the earlier plan's 1e-6: SIMD JPEG decoding
  paths may differ by one level in a few pixels between machines. Runs on one machine are exact.
- Further decisions are filled in here during implementation.

## Acceptance tests

Golden cases in `tests/cases/02-technical-scores/`; code tests in
`crates/keepers-core/tests/02_technical_scores.rs`. References: Pech-Pacheco et al. 2000, Krotkov
1987, Bigün and Granlund 1987, Immerkær 1996, IEC 61966-2-1 for the sRGB transfer function, and
each fixture's construction.

1. Blur ladder: for the base scene at σ 0, 0.5, 1, 2, 4, 8, `lap_var`, `tile_max` and `tenengrad` are
   each strictly decreasing. Property test over seeds 1 to 20 of the in-memory scene generator (no JPEG).
2. `tile_max ≥ lap_var` for every frame of every generated set (property).
3. `dof/`: `lap_var(dof) ≤ 0.1 × lap_var(sharp base)` and `tile_max(dof) ≥ 0.5 × tile_max(sharp base)`,
   from the construction (a sharp region covering at least 2 × 2 whole tiles in a σ 4 surround).
4. Exposure ramp: for EV shifts −2, −1, 0, +1, +2, `clip_high_pct` and `crush_low_pct` equal the values
   the test computes analytically from the ramp and the sRGB formula, within ±1.5 percentage points:
   it counts the columns whose code c, shifted in linear light (`sRGB(min(1, lin(c)·2^EV))`), lands at
   or above 250 or at or below 5 (for a grey pixel Y equals c exactly, since 54 + 183 + 19 = 256).
5. Flags: the flat-patch image with 10% of its area at 255 has `clip_high_pct` 10.0 ± 0.5 and is flagged
   overexposed; one with 25% of its area at 0 is flagged underexposed; the 0 EV ramp has neither flag.
6. `mean_luma` is strictly increasing over the EV shifts.
7. Noise (in-memory, not JPEG): flat grey 128 plus seeded Gaussian noise of σ 2, 5 and 10, rounded and
   clamped, gives `sigma` within ±15% of the true σ; a noise-free flat image gives `sigma < 0.1`.
8. Motion: blur of length 15 at 0° gives `angle_deg` within ±10° of 0 (mod 180), at 90° within ±10° of
   90, at 45° within ±10° of 45; `anisotropy` of each blurred frame is at least 2 × that of the base.
9. Determinism: two `score` runs give byte-identical output; 1 and 8 threads give identical output;
   enumeration order shuffled by the injected lister gives identical output.
10. Case `blur-ladder` golden: the JSON matches with `rel = 1e-4` on every float; passes on both runners.
11. Caching: a second `score` run reports `recomputed: 0`; bumping a test scorer's version to 2
    recomputes that scorer only.
12. A frame whose preview bytes are replaced by random data (in a temp copy) gives
    `preview_decode_failed` with the message above; other frames are scored.
13. Luma unit test: (255, 255, 255) → 255, (0, 0, 0) → 0, (255, 0, 0) → 54, (0, 255, 0) → 183, (0, 0, 255) → 19.
14. Laplacian unit test: a 5 × 5 image with one centre pixel of 1.0 and zeros elsewhere gives interior
    responses −4 at the centre, 1 at its four neighbours and 0 at the four interior corners; `lap_var`
    (population variance over the 9 interior pixels, mean 0) is 20/9 within 1e-12.
15. `originals_untouched.rs` still passes after `score`.

## Status

Planned
