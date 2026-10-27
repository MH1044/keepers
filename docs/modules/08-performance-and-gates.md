# 08: Performance and gates

## Goal

Keepers is fast enough for a wedding and the numbers are measured, not felt: a bounded parallel
pipeline, optional GPU acceleration, and a `gates` command that reports throughput, memory and, on the
maintainer's own private shoot, how often Keepers' rejects and best frames agree with a human cull.

## Depends on

07.

## Scope

1. One pipeline for `analyse`: a reader stage (preview bytes), a decode stage (working image and face
   crops from one decode), and scorer, face and embedding stages on a `rayon` pool of `threads`
   workers (default: physical cores − 1, at least 1); bounded channels of capacity `2 × threads`
   between stages, so at most that many decoded previews exist at once; results reduced by frame index.
2. ONNX sessions: one per worker (or one shared, if the pinned `ort` version allows concurrent runs;
   record which), with intra-op threads set to 1 when there is more than one worker.
3. Accelerator setting `cpu` (default) | `auto`: `auto` tries DirectML on Windows and CoreML on macOS
   and falls back to CPU with the notice "GPU acceleration is not available here; using the CPU."
   Scores with an accelerator stay within 1e-3 relative of CPU; goldens always run on CPU.
4. Timing and memory probes in the core: per-stage wall time through the injected clock, peak resident
   memory from the OS (VmHWM on Linux, PeakWorkingSetSize on Windows, `ru_maxrss` on macOS).
5. `keepers gates --set generated|raw|eval:<name> --json` printing `keepers.gates/1`:
   `{"set","frames","threads","accelerator","stages":{"scan_ms","thumbs_ms","scores_ms","faces_ms",
   "groups_ms","cull_ms"},"frames_per_minute","first_100_thumbs_ms","peak_rss_mb","agreement":{
   "reject_precision","reject_recall","reject_f1","best_frame_agreement","groups_compared"}|null}`.
   Only aggregates: no path, file name or frame id appears in the output or the log.
6. Evaluation sets: `eval-data/<name>/photos/` and `eval-data/<name>/cull.csv` with header
   `path,flag,stars` (`flag` in pick, reject, none; `path` relative to `photos/`). Rejects: human reject
   vs Keepers reject. Best-frame agreement: over Keepers groups where the human picked exactly one
   member, the share where that member is Keepers' best.
7. `keepers gates import-xmp <photos> --out <csv> [--pick-min-rating N]` builds `cull.csv` from the
   maintainer's existing sidecars (Rating −1 → reject; Rating ≥ N (default 1) → pick; else none).
8. `scripts/gates.sh` and `scripts/gates.ps1`: generated set always; the raw set and every
   `eval-data/*` set when present; exit 1 when a recorded threshold is missed.
9. The scaled-down gate in the normal suite (`docs/TESTING.md` §6) and the thresholds recorded in
   `CONTRIBUTING.md` under "Quality gates" once measured, with the date and machine class.
10. The accelerator switch in Settings > General, and "{n} frames a minute" shown in the status bar
    when an analysis finishes.

## Files

- `crates/keepers-core/src/pipeline/{mod.rs,stages.rs,channel.rs}`, `crates/keepers-core/src/sysinfo.rs`
- `crates/keepers-core/src/gates/{mod.rs,agreement.rs,import_xmp.rs}`, `crates/keepers-cli/src/commands/gates.rs`
- `scripts/{gates.sh,gates.ps1}`, `CONTRIBUTING.md` (Quality gates section), `app/src/views/Settings.tsx`
- `crates/keepers-core/tests/08_performance_and_gates.rs`, `tests/cases/08-performance-and-gates/`

## Interfaces

```rust
pub struct PipelineOptions { pub threads: usize, pub accelerator: Accelerator, pub stages: StageSet }
pub fn analyse(root: &Path, opts: &PipelineOptions, env: &Env, progress: &dyn Progress,
    cancel: &CancelToken) -> Result<AnalysisReport, PipelineError>;
pub fn agreement(human: &[HumanDecision], keepers: &[FrameDecision], groups: &[Group]) -> Agreement;
pub struct Agreement { pub reject_precision: f64, pub reject_recall: f64, pub reject_f1: f64,
    pub best_frame_agreement: f64, pub groups_compared: usize }
```

Errors: "eval-data/{name}/cull.csv is missing."; "cull.csv line {n}: unknown flag '{v}' (use pick,
reject or none)."; "cull.csv line {n}: {path} is not in photos/."

## Out of scope

- New scores or rules (their modules); cloud or remote benchmarking; publishing any evaluation data.

## Design notes

- **Maintainer decides**: an evaluation shoot of their own in `eval-data/` with its `cull.csv`. If it is
  absent when this module starts, the module stops until the maintainer has added one to `eval-data/`.
  If the maintainer says to proceed without it, the agreement thresholds are recorded as "not
  measured" and the rest of the module is done.
- Thresholds: when a metric is first measured, its threshold is set at about 90% of the measured
  value (throughput) or 0.05 below it (agreement) and written into `CONTRIBUTING.md`, and the
  measurement, date and machine class are recorded here.
- Further decisions are filled in here during implementation.

## Acceptance tests

Code tests in `crates/keepers-core/tests/08_performance_and_gates.rs`; golden cases in
`tests/cases/08-performance-and-gates/`. References: the definitions of precision, recall and F1, the
construction of the synthetic decision lists, and module 05's goldens (unchanged output).

1. Agreement, hand-computed: human rejects {1,2,3,4}, Keepers rejects {3,4,5} → precision 2/3, recall
   2/4, F1 4/7 within 1e-12; groups {0,1,2} (human picked 1, Keepers best 1) and {5,6} (human picked 5,
   Keepers best 6) and {7,8} (human picked both) → best-frame agreement 1/2 over 2 groups compared.
2. No human rejects: precision and recall are `null` rather than NaN; the output still parses.
3. The pipeline's `keepers.cull/1` for `wedding-mini` with 1, 2 and 8 threads is byte-identical to
   module 05's golden.
4. Bounded memory: with an instrumented decode stage, the number of decoded previews alive at once
   never exceeds `2 × threads` (checked with 1, 4 and 8 threads on the 40-frame set).
5. Scaled-down gate: `wedding-mini` analyses at ≥ 1 frame a second in a debug build with peak memory
   under 1 GB on both CI runners.
6. `keepers gates --set generated --json` prints a document with every key of Scope 5 and no string
   value containing a fixture file name or folder name (checked as text).
7. `cull.csv` errors: a missing file, a bad flag on line 3 and an unknown path give the three messages.
8. `import-xmp`: sidecars with Rating −1, 0, 1 and 4 give reject, none, pick and pick; with
   `--pick-min-rating 3` the 1-star frame becomes none.
9. Accelerator `auto` on a runner without a GPU falls back to CPU with the notice and identical output.
10. Cancelling `analyse` mid-run stops every stage within 2 s and leaves no half-written thumbnail.
11. `scripts/gates.sh` exits 1 when a threshold in `CONTRIBUTING.md` is set above the measured value (a test
    copy of the file), and 0 otherwise.
12. The evaluation set never leaves the machine: `eval-data/` is in `.gitignore` and the hygiene walk,
    and the gates log contains no path under it (checked on a synthetic eval set built by the test).
13. UI (Vitest, `api` mocked): the accelerator switch calls `settings_set` with `accelerator: "auto"`;
    a `job-done` result with `frames_per_minute: 412.4` shows "412 frames a minute".
14. On a machine with DirectML or CoreML (manual checklist, and the `models` CI job where a runner
    offers it): `auto` scores stay within 1e-3 relative of CPU on the `blur-ladder` set.

## Status

Planned
