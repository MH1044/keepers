# 05: Ranking and automatic cull

## Goal

A photographer gets an automatic first cull they can steer with a few plain settings and understand
from a reason on every decision; their own decisions always win, are journaled append-only, survive
cache deletion and can be undone. `keepers analyse <dir>` runs the whole pipeline in one command.

## Depends on

03, 04.

## Scope

1. Components in [0, 1]: `focus = min(1, tile_max / max(P90(tile_max), 1e-9))` with P90 by nearest
   rank over the shoot's scored frames; `face_focus = min(1, max_face_sharpness / P90 of the same over
   frames with a judged face)`; `exposure = clamp(1 − clip_high_pct/10 − crush_low_pct/40 −
   max(0, |mean_luma − 0.45| − 0.25), 0, 1)`; `eyes = 1` with no judged face, else
   `(open + 0.5 × uncertain) / judged`.
2. `composite = 0.5 × focus + 0.2 × exposure + 0.3 × eyes`.
3. Rules, all firing ones listed: with `T` = 0.25 / 0.40 / 0.55 for focus lenient / normal / strict,
   `motion_blur` when `anisotropy ≥ 3.0` and `focus < T + 0.2`; otherwise `out_of_focus` when
   `focus < T`; `subject_out_of_focus` when a face is judged, `face_focus < T` and `focus ≥ T`;
   `eyes_closed` when `reject_closed_eyes` and `closed ≥ 1`, except in a group where every member has
   `closed ≥ 1`; `overexposed` / `underexposed` when `reject_exposure` and the flag is set.
4. Group rule (`one_per_group`): among members not rejected by a rule, the highest composite (lowest
   index on a tie) is the best; the others get `not_best_in_group` with `better_by` = the component with
   the largest weighted lead of the best (0.5 focus, 0.3 eyes, 0.2 exposure; ties focus > eyes >
   exposure), or `order` when every lead is zero.
5. Keep target, `mode: "keep_percent"`: per scene (or the whole shoot with `per: "shoot"`),
   `K = max(1, round_half_up(keep_percent × n / 100))` with n the frames in the scene; the top K
   remaining candidates by composite become picks, the other candidates `none`. `mode: "threshold"`:
   candidates with `composite ≥ min_composite` are picks, the rest `none`.
6. Reasons: every automatic decision has at least one, with the exact codes and texts below.
7. CullSettings (ARCHITECTURE §4) with defaults `keep_percent 30`, `min_composite 0.6`, `per "scene"`,
   `one_per_group true`, `reject_closed_eyes true`, `focus "normal"`, `reject_exposure false`,
   `strictness "normal"`, `burst_gap_s 2.0`, `style false`; validation messages below; `settings_hash`.
8. The decision journal (format, events and replay rules in ARCHITECTURE §4): append with `fsync`,
   torn-line tolerance, undo, redo, reset, `accept_auto`, `cull_settings`; the effective decision.
9. `cull` table; `keepers cull <dir> --json` with `--keep <1–100> --per scene|shoot --focus
   lenient|normal|strict --strictness loose|normal|strict --no-reject-closed-eyes --reject-exposure
   --no-one-per-group --mode threshold --min <0–1>`, printing `keepers.cull/1`; `keepers analyse
   <dir> --json` runs scan, score, faces (if installed), group and cull. Settings come from the flags,
   else the journal's last `cull_settings`, else `settings.json` `cull_defaults`, else the defaults;
   the settings used are journaled as `cull_settings` when they differ from the last recorded ones.
10. `keepers decide <dir> (--frame <index|frame_id>)… (--flag pick|reject|none | --stars 0-5 |
    --label red|yellow|green|blue|purple|none | --reset | --accept-auto) --json` → `keepers.decide/1`
    `{"changed":[{"index","frame_id","decision"}]}`; `keepers undo <dir>` and `keepers redo <dir>`.
11. The `wedding-mini/` fixture set.

| Code | Text | Extra fields |
|---|---|---|
| `out_of_focus` | out of focus | `focus`, `threshold` |
| `motion_blur` | motion blur | `angle_deg` |
| `subject_out_of_focus` | face out of focus | `face_focus`, `threshold` |
| `eyes_closed` | eyes closed: {closed} of {faces} faces ("1 of 1 face") | `closed`, `faces` |
| `overexposed` | overexposed: {pct}% clipped highlights (integer) | `clip_high_pct` |
| `underexposed` | underexposed: {pct}% crushed shadows | `crush_low_pct` |
| `not_best_in_group` | a sharper frame of this burst was kept / a frame with open eyes in this burst was kept / a better-exposed frame of this burst was kept / an earlier frame of this burst with the same score was kept | `group`, `best`, `better_by` |
| `best_in_group` | best of a burst of {size} | `group`, `size` |
| `top_percent` | in the top {p}% of this scene / of the shoot | `percent`, `rank`, `of` |
| `below_keep_target` | below the keep target | `rank`, `of` |
| `above_threshold` / `below_threshold` | score {c} is above / below the threshold {t} (two decimals) | `composite`, `threshold` |

Validation: "keep must be between 1 and 100 (got {v})"; "min must be between 0 and 1 (got {v})";
"unknown focus setting '{v}' (use lenient, normal or strict)"; "no frame {v} in this shoot".

## Files

- `crates/keepers-core/src/rank/{mod.rs,settings.rs,composite.rs,rules.rs,cull.rs,reasons.rs}`
- `crates/keepers-core/src/journal/{event.rs,append.rs,replay.rs}`
- `crates/keepers-cli/src/commands/{cull.rs,analyse.rs,decide.rs}`
- `crates/keepers-core/tests/05_ranking_and_auto_cull.rs`, `tests/cases/05-ranking-and-auto-cull/`
- `crates/keepers-fixtures/src/wedding_mini.rs`

## Interfaces

```rust
pub fn cull(frames: &[FrameFeatures], groups: &[Group], scenes: &[Scene], s: &CullSettings) -> Vec<AutoDecision>;
pub struct FrameFeatures { pub index: usize, pub tile_max: f64, pub anisotropy: f64, pub angle_deg: f64,
    pub exposure: Exposure, pub faces: Option<FacesSummary> }
pub struct AutoDecision { pub index: usize, pub components: Components, pub composite: f64,
    pub flag: Flag, pub reasons: Vec<Reason> }
pub struct Journal; impl Journal {
    pub fn open(path: &Path) -> Result<(Journal, Replay), JournalError>;
    pub fn append(&mut self, ev: EventBody, clock: &dyn Clock) -> Result<u64, JournalError>; // returns seq
}
pub fn effective(replay: &Replay, auto: &[AutoDecision]) -> Vec<FrameDecision>;
pub enum JournalError { Io(io::Error), NewerFormat /* read-only */, NothingToUndo, NothingToRedo }
```

## Out of scope

- The review UI (06); export (07); style learning (10, which only reads `style`); second opinions (09).

## Design notes

- `cull` is a pure function of its inputs; the cull table is a cache of it.
- `wedding-mini/` (40 frames, 3 scenes 120 s apart, all channel values of the base scene within
  [20, 235] so every exposure component is 1.0 unless a fault is planted): "distinct" frames are the
  base scene flipped, rotated 180° and colour-inverted (equal sharpness by construction) with planted
  Gaussian blur σ 0, 0.5, 1.0, … so their order is known; faults are σ 8 blur and a +3 EV shift; bursts
  are pixel-identical copies with different capture times (ties, so the lowest index is best).
- Further decisions are filled in here during implementation.

## Acceptance tests

Golden cases in `tests/cases/05-ranking-and-auto-cull/`; code tests in
`crates/keepers-core/tests/05_ranking_and_auto_cull.rs`. References: the formulas above applied by hand
to synthetic `FrameFeatures` (exact), the blur-ladder ordering proved in module 02, and `truth.toml`.

1. Components: tile_max values {1, 2, …, 10} give P90 = 9 and focus 1/9 … 1, with frame 10 capped at
   1; clip 5% and crush 20% give exposure 0.0; mean 0.80 alone gives 0.90; faces open 1, uncertain 1,
   judged 2 give eyes 0.75.
2. Composite of focus 0.8, exposure 1.0, eyes 0.5 is 0.75 within 1e-12.
3. Rules on synthetic features: focus 0.39 → `out_of_focus` at normal, not at lenient; focus 0.5 with
   anisotropy 4 → `motion_blur` at normal (0.5 < 0.6); focus 0.7 and face_focus 0.3 →
   `subject_out_of_focus`; closed 2 of 3 → `eyes_closed` with text "eyes closed: 2 of 3 faces".
4. Eyes exception: a group whose three members each have one closed face rejects none by
   `eyes_closed`; the group rule keeps the best.
5. Group rule: members with composites 0.9, 0.7 (focus lead) and 0.9 (tie, higher index) give best =
   the first; the second has `better_by: "focus"` and "a sharper frame of this burst was kept"; the
   third has `better_by: "order"`.
6. Keep target: one scene of 40 candidates at 30% gives exactly 12 picks, the 12 highest composites;
   scenes of 12 and 14 give 4 and 4 (`round_half_up(3.6)`, `round_half_up(4.2)`); a scene of one frame
   gives one pick; `per: "shoot"` counts over all 40.
7. Threshold mode with `min 0.6`: composites 0.61 and 0.59 give pick and none with the texts
   "score 0.61 is above the threshold 0.60" and "score 0.59 is below the threshold 0.60".
8. Property (proptest, 1,000 random shoots of synthetic features and settings): every decision has at
   least one reason; picks never exceed K per scene; a stricter focus setting rejects a superset.
9. Case `wedding-mini` (default settings; composites and numeric reason fields ignored): flags and
   reason codes equal `truth.toml`: σ 8 frames `out_of_focus`; burst copies after the first
   `not_best_in_group` with `better_by: "order"`; per scene the least-blurred K frames are picks.
10. Override: `keepers decide wedding-mini --frame 5 --flag pick` on an automatic reject, then `cull
    --keep 10`: frame 5 stays `pick` with `source: "user"`; `--reset` returns it to automatic.
11. Undo and redo: flag, stars and label, then undo twice: flag only remains; redo restores the stars;
    undo with nothing left gives "Nothing to undo."
12. Journal golden: `tests/cases/05-ranking-and-auto-cull/journal-replay/decisions.jsonl` (the ten
    lines of ARCHITECTURE §4 with a matching fixture) replays to the effective decisions listed by hand
    in `expected.json`.
13. Append-only: after each of 20 random operations the previous journal bytes are a prefix of the new.
14. Torn line: a journal ending in a half-written line replays all complete events; the next append
    starts on a new line; replay still skips the fragment.
15. A journal with `"v":2` opens read-only with "This shoot was saved by a newer Keepers. Update
    Keepers to edit it."; `decide` then exits 1 with the same message.
16. Delete the cache and `index.sqlite`, run `analyse` again: byte-identical `keepers.cull/1`.
17. `settings_hash` of the defaults is stable across runs and platforms and changes when any field changes.
18. Validation: `--keep 0`, `--min 1.5`, `--focus blurry`, `--frame 999` give exit 1 with the messages above.
19. `originals_untouched.rs` passes after `analyse` and `decide`.

## Status

Planned
