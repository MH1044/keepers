# 10: Learn my style

## Goal

After a few shoots the automatic cull adapts to how this photographer culls: a small model, fitted on
this computer from their own past decisions, nudges the composite score. The user sees how much it
changed and can switch it off or reset it. With no history the cull is exactly as before.

## Depends on

05, 06.

## Scope

1. Training examples from every journal in `data/decisions/`: frames whose effective flag is a user
   `pick` or `reject` (including `accept_auto`), label 1 for pick and 0 for reject; `none` is left out.
   Features come from that shoot's cache; a shoot whose cache is gone is skipped and counted.
2. Feature vector, standardised with the training mean and standard deviation: focus, face_focus
   (0 when no face is judged), exposure, eyes, noise σ / 10, is best in its group (0 or 1),
   ln(group size) (0 for a single frame), min(faces, 10) / 10, composite.
3. L2-regularised logistic regression (λ = 1.0 on the weights, not the intercept) fitted by IRLS in
   f64, at most 50 iterations, stopping when the largest weight change is below 1e-9; examples in
   (shoot_id, frame index) order.
4. Minimum history: at least 200 examples with at least 20 of each class; otherwise
   `not_enough_history`: "Not enough history yet: {n} decisions from {m} shoots (200 needed, with at
   least 20 picks and 20 rejects)."
5. Blend when `style` is on: `composite' = (1 − α) × composite + α × p`, `α = min(0.5, n / 2,000)`,
   with p the model's probability; the rest of the cull runs unchanged on `composite'`.
6. `data/style/model.json`: `{"version":1,"trained_at","examples":n,"shoots":m,"skipped_shoots":k,
   "mean":[…],"std":[…],"weights":[…],"intercept","alpha"}` (derived; rebuilt by fitting again).
7. The change report: "Your style changed {k} of {n} automatic decisions in this shoot." and a grid
   filter "changed by my style".
8. Reset deletes `model.json` and sets `style` off.
9. CLI `keepers style fit|show|reset --json` printing `keepers.style/1`, and `keepers cull --style`.
10. UI: "Use my style" in the cull panel (disabled with the not-enough-history sentence until a model
    exists) and Settings > Style with Fit again and Reset.

## Files

- `crates/keepers-core/src/style/{mod.rs,features.rs,logistic.rs,blend.rs}`
- `crates/keepers-cli/src/commands/style.rs`, `app/src/views/{CullPanel.tsx,SettingsStyle.tsx}`
- `crates/keepers-core/tests/10_learn_my_style.rs`, `tests/cases/10-learn-my-style/`, `app/src/state/style.test.ts`

## Interfaces

```rust
pub fn collect_examples(data: &DataDir) -> Result<(Vec<Example>, CollectStats), StyleError>;
pub fn fit(examples: &[Example]) -> Result<StyleModel, StyleError>;   // NotEnoughHistory { n, m }
pub fn blend(auto: &mut [AutoDecision], feats: &[FeatureVector], model: &StyleModel);
pub enum StyleError { NotEnoughHistory { n: usize, m: usize }, Io(io::Error), Corrupt }
```

`keepers.style/1`: `{"schema","status":"none"|"not_enough_history"|"ready","examples","shoots",
"skipped_shoots","alpha","weights":{"focus",…,"composite"},"intercept","message"}` (weights keyed by
feature name, `null` when there is no model). A damaged `model.json` gives "The style model could not be read; fit it again in Settings > Style."
and the cull runs without style.

## Out of scope

- Sharing or uploading a model; learning stars or labels; learning from other photographers; any
  network use.

## Design notes

- The fit is deterministic: fixed order, no randomness, f64 throughout; results are compared with a
  tolerance, never exactly, across platforms.
- Further decisions are filled in here during implementation.

## Acceptance tests

Code tests in `crates/keepers-core/tests/10_learn_my_style.rs`; golden cases in
`tests/cases/10-learn-my-style/`; UI in `app/src/state/style.test.ts`. References: IRLS for logistic
regression (Hastie, Tibshirani and Friedman, ESL §4.4) and synthetic data with a planted preference.

1. Planted preference: 400 synthetic examples with features drawn from a fixed-seed generator and label
   1 exactly when exposure > 0.6 → the exposure weight is the largest in absolute value and positive,
   and training accuracy ≥ 0.95.
2. IRLS unit test: on the 6-example, 1-feature data set x = (−2, −1, 0, 0, 1, 2), y = (0, 0, 0, 1, 1, 1)
   with λ = 1.0, the fitted weight and intercept match the values from a direct Newton iteration
   written independently in the test, within 1e-9.
3. No history: an empty `decisions/` gives `not_enough_history` with n = 0, m = 0, and `keepers cull`
   output is byte-identical to the same command without `--style`.
4. Threshold: 199 examples → `not_enough_history`; 200 with only 19 rejects → `not_enough_history`;
   200 with 20 of each → a model.
5. α: n = 400 gives 0.2; n = 1,000 gives 0.5; n = 5,000 gives 0.5.
6. Skipped shoots: two journals whose caches were deleted are counted in `skipped_shoots` and do not
   fail the fit.
7. Determinism: fitting twice gives identical `weights` and `intercept` on one machine.
8. Change report: with a model that raises every frame with exposure > 0.6, the report on `wedding-mini`
   counts exactly the frames whose flag differs with and without style (computed by the test from two
   `cull` runs).
9. Reset removes `model.json`; the next cull equals the no-style cull; `style show` says "No style
   model yet."
10. A truncated `model.json` gives the damaged-model message and a no-style cull.
11. UI: "Use my style" is disabled with the not-enough-history sentence when `style_status` reports no
    model, and enabled otherwise.

## Status

Planned
