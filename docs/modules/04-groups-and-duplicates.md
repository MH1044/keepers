# 04: Groups and duplicates

## Goal

Bursts and near-duplicates are grouped so that only the best frame of each needs a look, and the
shoot is cut into scenes (ceremony, portraits, reception) so the keep target can apply per scene.
Grouping works without the optional similarity model, and says when it ran in that simpler mode.

## Depends on

02, and 03 for the model registry and downloader.

## Scope

1. `dhash@1`: the working image's luma resized to 9 × 8 with the `Triangle` filter; bit
   `i = 8y + x` (MSB first) is 1 when `p(x, y) > p(x + 1, y)`; a 64-bit value in `hashes`.
2. CLIP ViT-B/32 image embedding (512 dimensions, L2-normalised, f32 LE in `embeddings`): resize the
   shorter side to 224 (bicubic), centre-crop 224 × 224, normalise with mean (0.48145466, 0.4578275,
   0.40821073) and std (0.26862954, 0.26130258, 0.27577711). Registry entry `clip-vit-b32-visual`,
   group `similarity`, optional.
3. Candidate pairs: frames adjacent in index order whose capture times differ by ≤ `burst_gap_s`
   (default 2.0); and any pair with dHash Hamming distance ≤ 12 whose capture times differ by ≤ 300 s
   (or both lack a time).
4. Confirmation: with CLIP, cosine ≥ 0.88 / 0.92 / 0.95 for strictness loose / normal / strict; without
   CLIP (`grouping: "hash_only"`), Hamming ≤ 14 / 10 / 6.
5. Groups: connected components of confirmed pairs by union-find, processing pairs in (i, j) index
   order. A component larger than 40 frames is cut at its largest capture-time gap (earliest on a tie),
   repeatedly, until every part has ≤ 40 frames.
6. Group ids `g0001`, `g0002`, … in order of each group's first member index; a single frame is never a
   group (`group: null`). Members are ranked later by module 05; here `rank` is the member's index order.
7. Scenes: a boundary between consecutive frames i and i + 1 when the capture gap is > 60 s, or CLIP
   cosine < 0.75; frames without a capture time form the last scene. Ids `s001`, `s002`, ….
8. Strictness, `burst_gap_s` and the grouping mode are part of CullSettings, so the `settings_hash`
   changes when they change and groups are recomputed.
9. `keepers group <dir> --json [--strictness loose|normal|strict] [--gap <s>]` printing
   `keepers.groups/1`: `{"schema","shoot_id","grouping":"clip"|"hash_only","settings_hash",
   "groups":[{"id","members":[index…]}],"scenes":[{"id","first","last"}],"singletons":n}`.
10. Without CLIP installed, stderr: "Grouping by similarity is limited: the similarity model is not
    installed. Download it with `keepers models download --group similarity` for better groups."
11. Fixture sets `bursts/` and `long-burst/`.

## Files

- `crates/keepers-core/src/group/{mod.rs,dhash.rs,clip.rs,candidates.rs,components.rs,scenes.rs}`
- `crates/keepers-core/registry/models.toml` (the CLIP row filled in)
- `crates/keepers-cli/src/commands/group.rs`
- `crates/keepers-core/tests/04_groups_and_duplicates.rs`, `tests/cases/04-groups-and-duplicates/`
- `crates/keepers-fixtures/src/{bursts.rs,long_burst.rs}`

## Interfaces

```rust
pub fn dhash(luma_working: &GrayImage) -> u64;
pub fn hamming(a: u64, b: u64) -> u32;                      // (a ^ b).count_ones()
pub fn embed(session: &Clip, working: &RgbImage) -> [f32; 512];
pub fn group_shoot(cache: &mut ShootCache, settings: &GroupSettings, models: &Models,
    progress: &dyn Progress, cancel: &CancelToken) -> Result<GroupsResult, GroupError>;
pub struct GroupSettings { pub strictness: Strictness, pub burst_gap_s: f64, pub max_group: usize /* 40 */ }
pub struct GroupsResult { pub mode: GroupingMode, pub groups: Vec<Group>, pub scenes: Vec<Scene> }
```

## Out of scope

- Choosing the best frame of a group (05); showing groups (06); grouping across shoots.

## Design notes

- The CLIP export (ONNX of the vision tower only) and its licence are chosen here and recorded in
  ADR-0004 and `THIRD_PARTY.md`; prefer a quantised export if it keeps cosine within 0.01 of fp32 on the
  generated bursts. Record its size in the registry.
- All-pairs Hamming within the 300 s window is cheap (64-bit popcount); no index structure is needed.
- Further decisions are filled in here during implementation.

## Acceptance tests

Golden cases in `tests/cases/04-groups-and-duplicates/`; code tests in
`crates/keepers-core/tests/04_groups_and_duplicates.rs`. References: Krawetz 2013 for dHash, Tarjan
1975 for union-find, and the fixtures' construction (`truth.toml`).

1. dHash unit test: a 9 × 8 image whose rows decrease left to right gives `0xFFFF_FFFF_FFFF_FFFF`; one
   whose rows increase gives 0; a hand-built pattern gives the value worked out in the test.
2. Hamming: `hamming(0b1011, 0b0001) = 2`; `hamming(u64::MAX, 0) = 64`.
3. Case `bursts` (hash-only mode, no models needed): groups exactly {A1–A5}, {B1–B3}, {C1–C4} and the
   near-duplicate pair, as listed in `truth.toml`; the 4 singletons have `group: null`.
4. The same case with CLIP installed (skips otherwise) gives the same groups.
5. Distinct images are never merged: property test over seeds 1 to 20, 10 distinct base scenes 30 s
   apart per seed, gives 10 singletons and no groups.
6. Order invariance: shuffling the directory listing (injected lister, 10 random orders with a fixed
   seed) gives byte-identical `keepers.groups/1` output.
7. Case `long-burst`: 50 frames split into exactly two groups, indices 0–24 and 25–49.
8. Two shifted copies of one scene 0.5 s apart form a group; two distinct scenes 1 s apart (a time
   candidate that fails confirmation) do not; two shifted copies 250 s apart form a group through the
   hash candidate; the same copies 400 s apart do not.
9. Strictness: on `bursts/` the number of confirmed pairs under strict ≤ normal ≤ loose (property of the
   thresholds), and the `settings_hash` differs between the three.
10. Scenes: frames 30 s apart stay in one scene; a 90 s gap starts `s002`; frames without a capture
    time form the last scene.
11. Without CLIP, the output has `grouping: "hash_only"` and stderr carries the message above.
12. CLIP preprocessing unit test: a 448 × 300 image becomes 335 × 224 before the crop (shorter side 224,
    width `round(448 × 224 / 300)` = 335) and 224 × 224 after; a mid-grey pixel of 128 normalises to
    `(128/255 − mean) / std` per channel within 1e-6.
13. Embeddings are stored L2-normalised: every stored vector's norm is 1 within 1e-5.
14. `originals_untouched.rs` passes after `group`.

## Status

Planned
