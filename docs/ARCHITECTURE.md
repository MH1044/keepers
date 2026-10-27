# Keepers: architecture

Keepers is a Rust core library driven by two front ends: a Tauri 2 desktop app and a CLI. Everything
that decides anything lives in the core. This file is capped at five pages; module specs hold the detail.

## 1. Stack

| Component | Choice | Licence (verify at module) | Why |
|---|---|---|---|
| Desktop shell | Tauri 2 | MIT OR Apache-2.0 (00) | low memory with thousands of thumbnails; shares the Rust core with the CLI |
| UI | React 19, TypeScript, Vite | MIT (00) | widely known; typed boundary to the core |
| Grid virtualisation | `@tanstack/react-virtual` | MIT (06) | 6,000-cell grid without 6,000 DOM nodes |
| Core | Rust stable (pinned in `rust-toolchain.toml`) | n/a | one implementation for app, CLI and tests |
| CLI | `clap` | MIT OR Apache-2.0 (00) | |
| JSON | `serde`, `serde_json` | MIT OR Apache-2.0 (00) | |
| Derived cache | `rusqlite` with bundled SQLite | MIT; SQLite public domain (01) | one file per shoot, easy to delete |
| Hashing | `sha2` | MIT OR Apache-2.0 (01) | identities, cache keys |
| JPEG decode, encode, resize | `image` (pure Rust) | MIT OR Apache-2.0 (01) | no C toolchain; scalar resize gives the same result on every CPU |
| RAW containers, EXIF | in-house TIFF/IFD, ISO-BMFF and RAF readers | n/a (01) | preview extraction differs per format; `kamadak-exif` (BSD-2-Clause) allowed for tag decoding if module 01 prefers it |
| Parallelism | `rayon` | MIT OR Apache-2.0 (02) | fixed-order reduction keeps results deterministic |
| ML runtime | `ort` (ONNX Runtime) | MIT OR Apache-2.0; ONNX Runtime MIT (03) | CPU everywhere; DirectML and CoreML optional (08) |
| Model downloads | `ureq` with rustls | MIT OR Apache-2.0 (03) | small blocking client, no OpenSSL |
| XMP | `quick-xml` | MIT (07) | event stream lets a merge keep foreign content byte for byte |
| JSON Schema | `jsonschema` | MIT (09) | validates every AI answer |
| Publik transport | system `curl`, as the reference module does | curl licence (09) | keep `publik.rs` close to the reference |
| Logging | `tracing`, `tracing-subscriber`, `tracing-appender` | MIT (00) | |
| Tests | `proptest`; `tiny_http` as a localhost fake server; Vitest; `tauri-driver` + WebdriverIO | MIT / Apache-2.0 (00, 03, 06) | |
| Fixture maths | `libm` | MIT OR Apache-2.0 (00) | bit-identical fixtures on Windows and Linux |
| Licence gate | `cargo-deny`; `license-checker-rseidelsohn` | MIT OR Apache-2.0; BSD-3-Clause (00) | |

Models (ADR-0004, rows in `THIRD_PARTY.md`): **YuNet** face detector from OpenCV Zoo (the folder's
README says "All files in this directory are licensed under MIT License"); **MediaPipe face landmark**
model as ONNX (Apache-2.0 upstream; the conversion's licence is recorded at 03); **CLIP ViT-B/32** image
encoder as ONNX (MIT upstream; the export is chosen at 04). Never InsightFace or any `-NC` weights.

## 2. Repository layout

    Cargo.toml                  workspace: crates/*, app/src-tauri
    rust-toolchain.toml, deny.toml
    crates/keepers-core/        the library; nothing here knows about Tauri or the terminal
      src/env.rs                injected Clock and cancellation token
      src/ingest/               scan, identity, tiff.rs, bmff.rs, raf.rs, jpeg.rs, exif.rs, preview.rs, thumbs.rs
      src/cache/                schema.sql, index.sql, open/rebuild
      src/score/                sharpness.rs, motion.rs, exposure.rs, noise.rs, working.rs
      src/models/               registry.rs, download.rs, session.rs; registry/models.toml (compiled in)
      src/faces/                yunet.rs, landmarks.rs, eyes.rs
      src/group/                dhash.rs, clip.rs, components.rs, scenes.rs
      src/rank/                 settings.rs, composite.rs, cull.rs, reasons.rs
      src/journal/              event.rs, append.rs, replay.rs
      src/export/               plan.rs, xmp_write.rs, xmp_merge.rs, targets.rs, copy.rs, undo.rs
      src/pipeline/, src/gates/ bounded parallel analyse, gates and agreement (08); src/style/ (10); src/update.rs (11)
      tests/                    hygiene.rs, originals_untouched.rs, source_rules.rs, NN_slug.rs per module
    crates/keepers-cli/         binary `keepers`; tests/golden.rs is the golden harness
    crates/keepers-fixtures/    binary `keepers-fixtures`: seeded fixture generator
    crates/keepers-publik/      module 09: src/publik.rs (reference, copied), provider.rs, client.rs,
                                gemini.rs, prepare.rs, second_opinion.rs, cache.rs, status.rs;
                                publik-app-token.txt beside its Cargo.toml (read by include_str!)
    app/                        React UI: src/api.ts (the only file that calls invoke), src/state/*.ts
                                (pure logic, Vitest), src/views/*.tsx, src/theme.css
    app/src-tauri/              Tauri host: commands.rs (marshalling only), protocol.rs, tauri.conf.json,
                                capabilities/default.json, commands_ai.rs (module 09)
    prompts/                    second-opinion.v1.md, second-opinion.v1.schema.json, local-vision-probe.v1.md
    tests/cases/NN-slug/<case>/ golden cases: case.toml, expected.json, expected/ (files)
    tests/fixtures/             gen.sha256 (committed), gen/ (regenerated, ignored), cc0/, raw-samples.toml
    scripts/                    verify.sh, verify.ps1, gates.sh, gates.ps1, fetch-raw-samples.sh,
                                fetch-models.sh, release-check.sh, publik-proof.sh
    eval-data/                  ignored: the maintainer's private evaluation shoots (module 08)
    docs/                       this file and the rest; docs/modules/, docs/adr/, docs/releases/

## 3. Components and data flow

    React UI ──invoke / events / keepers:// images──► Tauri host (commands.rs, protocol.rs)
                                                            │ no logic, no scores
    keepers CLI (JSON out, JSON Lines progress) ─────────┐  ▼
                                                  keepers-core
      ingest ─► score ─► faces ─► group ─► rank ─► (journal) ─► export
         │        │        │        │        │         ▲
         └────────┴── cache.sqlite + thumbs (derived) ─┘   decisions/<shoot>.jsonl (source of truth)
                           models/ (ONNX, verified)         keepers-publik (module 09, optional)

**ingest** walks the folder, fingerprints files, pairs RAW+JPEG into frames, matches the shoot, reads
EXIF and the offsets of the largest embedded preview, and writes thumbnails. **score** decodes each
preview once into a 1024 px working image and computes versioned scorers. **faces** runs YuNet on the
working image, crops each face from the full preview and measures sharpness and eye openness.
**group** builds bursts and near-duplicates from time gaps, dHash and CLIP similarity, and cuts scenes.
**rank** turns components into a composite, applies the cull settings and attaches reasons. **journal**
records every user decision, append-only. **export** plans, then writes sidecars and copies, with
backups and an undo log. **keepers-publik** prepares frames and asks a vision model for advice; it never
writes a decision. The Tauri host only marshals: every command is a thin call into the core.

## 4. Data model

Roots: data `<data_local_dir>/keepers/` (`%LOCALAPPDATA%\keepers`, `~/Library/Application Support/keepers`,
`$XDG_DATA_HOME/keepers`), cache `<cache_dir>/keepers/`. `--data-dir D` or `KEEPERS_DATA_DIR=D` puts
the data root at `D` and the cache root at `D/cache` (all tests do this).

    data/settings.json               user settings                               source of truth
    data/decisions/<shoot>.jsonl     decision journal                            SOURCE OF TRUTH
    data/index.sqlite                shoots and their frames                     derived (from journals)
    data/models/<id>/<file>          model weights, SHA-256 verified             re-downloadable
    data/exports/<shoot>/<export>.jsonl, data/backups/<shoot>/<export>/…         export undo log, prior sidecars
    data/secrets.json (0600), data/publik-status.json, data/ai-cache/*.sqlite    module 09
    data/style/model.json            module 10                                   derived
    data/logs/keepers.log            3 × 1 MB rotation, no content               local only
    cache/shoots/<shoot>/cache.sqlite, thumbs/<frame>.jpg, tmp/                   derived

**Identities (ADR-0006).** `file_id` = lowercase hex of the first 16 bytes of SHA-256 over
`"keepers-file-v1\n" ‖ u64le(size) ‖ content`, where content is the whole file up to 131,072 bytes,
else its first and last 65,536 bytes. Files with the same directory and case-insensitive stem form
one **frame**; the RAW is primary, a JPEG is its companion; `frame_id` = the primary's `file_id`.
Opening a folder matches the stored shoot whose frame set S has the largest overlap with the folder's
set F, when `|F∩S| ≥ 0.9 × min(|F|,|S|)` (ties: earliest `first_seen`, then `shoot_id`); otherwise a
new `shoot_id` = first 16 bytes hex of SHA-256 over `"keepers-shoot-v1\n"` and the sorted frame ids
joined by `\n`. Renaming the folder or adding a card keeps the id. Frame **index** = position in the
sort by (`capture_ms`, nulls last; primary relative path, bytewise).

**cache.sqlite** (`PRAGMA user_version = 1`; any mismatch deletes and rebuilds the cache):

```sql
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);   -- shoot_id, root, app_version
CREATE TABLE files (file_id TEXT PRIMARY KEY, frame_id TEXT NOT NULL, rel_path TEXT NOT NULL UNIQUE,
  format TEXT NOT NULL CHECK (format IN ('jpeg','cr2','cr3','nef','arw','raf','orf','rw2','dng')),
  role TEXT NOT NULL CHECK (role IN ('primary','companion')), size_bytes INTEGER NOT NULL,
  mtime_ns INTEGER NOT NULL);
CREATE INDEX files_frame ON files(frame_id);
CREATE TABLE frames (frame_id TEXT PRIMARY KEY, idx INTEGER NOT NULL UNIQUE,
  capture_time TEXT, capture_offset TEXT, capture_ms INTEGER,  -- camera local time; ms treat it as UTC
  make TEXT, model TEXT, lens TEXT, focal_mm REAL, shutter_s REAL, aperture REAL, iso INTEGER,
  orientation INTEGER NOT NULL DEFAULT 1 CHECK (orientation BETWEEN 1 AND 8),
  preview_file_id TEXT NOT NULL, preview_offset INTEGER NOT NULL, preview_length INTEGER NOT NULL,
  preview_source TEXT NOT NULL, preview_w INTEGER NOT NULL, preview_h INTEGER NOT NULL, -- oriented
  thumb_ok INTEGER NOT NULL DEFAULT 0);
CREATE INDEX frames_time ON frames(capture_ms);
CREATE TABLE file_errors (rel_path TEXT PRIMARY KEY, code TEXT NOT NULL, message TEXT NOT NULL);
CREATE TABLE scores (frame_id TEXT NOT NULL REFERENCES frames ON DELETE CASCADE,
  scorer TEXT NOT NULL, version INTEGER NOT NULL, value TEXT NOT NULL,  -- JSON per scorer
  PRIMARY KEY (frame_id, scorer));
CREATE TABLE faces (frame_id TEXT NOT NULL REFERENCES frames ON DELETE CASCADE, n INTEGER NOT NULL,
  x REAL NOT NULL, y REAL NOT NULL, w REAL NOT NULL, h REAL NOT NULL,   -- 0..1, oriented preview
  det_score REAL NOT NULL, sharpness REAL, ear_left REAL, ear_right REAL,
  eyes TEXT NOT NULL CHECK (eyes IN ('open','closed','uncertain','too_small')),
  version INTEGER NOT NULL, PRIMARY KEY (frame_id, n));                  -- no face vectors, ever
CREATE INDEX faces_eyes ON faces(eyes);
CREATE TABLE hashes (frame_id TEXT PRIMARY KEY, dhash INTEGER NOT NULL, version INTEGER NOT NULL);
CREATE TABLE embeddings (frame_id TEXT PRIMARY KEY, model TEXT NOT NULL, dim INTEGER NOT NULL,
  vector BLOB NOT NULL);                                                 -- f32 LE, L2-normalised
CREATE TABLE groups (group_id TEXT PRIMARY KEY, settings_hash TEXT NOT NULL, best_frame_id TEXT NOT NULL);
CREATE TABLE group_members (group_id TEXT NOT NULL REFERENCES groups ON DELETE CASCADE,
  frame_id TEXT NOT NULL UNIQUE, rank INTEGER NOT NULL, PRIMARY KEY (group_id, frame_id));
CREATE TABLE scenes (scene_id TEXT PRIMARY KEY, first_idx INTEGER NOT NULL, last_idx INTEGER NOT NULL);
CREATE TABLE cull (frame_id TEXT PRIMARY KEY, settings_hash TEXT NOT NULL, composite REAL NOT NULL,
  flag TEXT NOT NULL CHECK (flag IN ('pick','reject','none')), reasons TEXT NOT NULL);  -- JSON array
```

Scorer values (JSON in `scores.value`): `sharpness@1` `{"lap_var","tile_max","tenengrad"}`;
`motion@1` `{"anisotropy","angle_deg"}`; `exposure@1` `{"mean_luma","clip_high_pct","crush_low_pct","flags":[]}`;
`noise@1` `{"sigma"}`; `faces@1` `{"checked","count","judged","closed","uncertain","too_small","max_face_sharpness"}`.

**index.sqlite**: `shoots(shoot_id TEXT PRIMARY KEY, display_name TEXT NOT NULL, root_path TEXT,
first_seen TEXT NOT NULL, last_opened TEXT NOT NULL, frame_count INTEGER NOT NULL)` and
`shoot_frames(shoot_id TEXT NOT NULL, frame_id TEXT NOT NULL, PRIMARY KEY (shoot_id, frame_id))` with
an index on `frame_id`. Rebuilt from the journals when missing.

**Decision journal** `decisions/<shoot_id>.jsonl`: UTF-8, one JSON object per line, LF, append-only,
`fsync` after each append. `v` is the format version, `seq` increases by one, `ts` comes from the
injected clock. A line that is not valid JSON (a torn write) is skipped on read with a warning, wherever
it is; before appending to a file that does not end in a newline, the writer adds one. The effective flag of a frame is its latest non-undone user flag after
its latest `reset`, else the automatic flag; stars and label default to 0 and none.

```jsonl
{"v":1,"seq":0,"ts":"2026-06-13T21:04:10.512Z","type":"shoot","shoot_id":"5c1f0e9a2b7d4c61a0e3f9b28d7c4e15","app":"keepers 0.1.0","frames":["0b6f3a9c1d2e4f5061728394a5b6c7d8","19a2b3c4d5e6f708192a3b4c5d6e7f80"]}
{"v":1,"seq":1,"ts":"2026-06-13T21:09:44.020Z","type":"flag","frames":["0b6f3a9c1d2e4f5061728394a5b6c7d8"],"value":"pick"}
{"v":1,"seq":2,"ts":"2026-06-13T21:09:45.101Z","type":"stars","frames":["0b6f3a9c1d2e4f5061728394a5b6c7d8"],"value":4}
{"v":1,"seq":3,"ts":"2026-06-13T21:09:47.300Z","type":"label","frames":["19a2b3c4d5e6f708192a3b4c5d6e7f80"],"value":"green"}
{"v":1,"seq":4,"ts":"2026-06-13T21:10:02.000Z","type":"undo","target":3}
{"v":1,"seq":5,"ts":"2026-06-13T21:11:30.000Z","type":"cull_settings","settings":{"version":1,"mode":"keep_percent","keep_percent":25,"min_composite":0.6,"per":"scene","one_per_group":true,"reject_closed_eyes":true,"focus":"normal","reject_exposure":false,"strictness":"normal","burst_gap_s":2.0,"style":false}}
{"v":1,"seq":6,"ts":"2026-06-13T21:12:00.000Z","type":"reset","frames":["0b6f3a9c1d2e4f5061728394a5b6c7d8"]}
{"v":1,"seq":7,"ts":"2026-06-13T21:15:00.000Z","type":"accept_auto","items":[{"frame":"19a2b3c4d5e6f708192a3b4c5d6e7f80","flag":"reject"}]}
{"v":1,"seq":8,"ts":"2026-06-14T08:00:00.000Z","type":"frames_added","frames":["2c3d4e5f60718293a4b5c6d7e8f90a1b"]}
{"v":1,"seq":9,"ts":"2026-06-14T08:01:00.000Z","type":"flag","frames":["2c3d4e5f60718293a4b5c6d7e8f90a1b"],"value":"pick","via":"second_opinion"}
```

Types: `shoot` (first line only), `frames_added`, `flag` (`pick|reject|none`), `stars` (0–5), `label`
(`red|yellow|green|blue|purple|null`), `reset` (drop the user flag; stars and label stay), `undo`
(`target` = a seq; redo appends a copy of the undone event), `accept_auto` (freezes automatic flags as
user flags), `cull_settings`. `via` is informational and never changes replay; the only `via` value
is `second_opinion`, on the single `pick` that **Pick this frame** writes for the chosen frame. Unknown `type` or a
higher `v` makes the shoot read-only with "This shoot was saved by a newer Keepers. Update Keepers to
edit it."

**Model registry** `crates/keepers-core/registry/models.toml`, compiled in:

```toml
schema = 1
[[model]]
id = "yunet-2026may"
task = "face_detection"            # face_detection | face_landmarks | image_embedding
file = "face_detection_yunet_2026may.onnx"
url = "https://github.com/opencv/opencv_zoo/raw/<pinned-commit>/models/face_detection_yunet/face_detection_yunet_2026may.onnx"
sha256 = "<64 hex, filled at module 03 from the downloaded file>"
size_bytes = 0                      # filled at module 03
licence = "MIT"
source = "OpenCV Zoo"
group = "faces"                     # faces (needed for eye checks) | similarity (optional)
[model.input]
layout = "NCHW"
colour = "BGR"
dynamic = true
```

**XMP sidecars** (written only by Export): byte-exact examples for Lightroom Classic, Capture One and
darktable, and the merge rules, are in `docs/modules/07-export.md` (ADR-0005).

**Settings** `settings.json`: `{"version":1,"threads":0,"accelerator":"cpu","advance_after_flag":true,
"update_check":true,"cull_defaults":{…CullSettings…},"second_opinion":{"enabled":false,"provider":null,
"own":{"base_url":null,"model":null},"local":{"base_url":"http://localhost:11434/v1","model":null,"multi_image":null},
"permission_confirmed":false},"style":{"enabled":false}}`. CullSettings is the object in the journal
example; its canonical JSON (sorted keys, no spaces) hashed with SHA-256, first 16 hex, is the
`settings_hash`.

## 5. Interfaces

**CLI** (`keepers`; JSON on stdout with `--json`, progress as JSON Lines on stderr with
`--progress jsonl`; every document starts with `"schema"`). Exit codes: 0 ok, 1 usage, 2 per-file
errors with `--strict`, 3 setup needed, 4 needs credit, 5 cancelled, 6 export conflict or stale plan,
7 AI error, 10 internal. Errors print `{"schema":"keepers.error/1","code":"…","message":"…"}`.

| Command | Document | Module |
|---|---|---|
| `keepers --version --json` | `keepers.version/1` `{name, version}` | 00 |
| `keepers scan <dir>` | `keepers.scan/1` frames with files, EXIF, preview; errors | 01 |
| `keepers score <dir>` | `keepers.scores/1` per frame sharpness, motion, exposure, noise | 02 |
| `keepers models list\|download\|verify` | `keepers.models/1` | 03 |
| `keepers faces <dir>` | `keepers.faces/1` per frame faces and eye states | 03 |
| `keepers group <dir>` | `keepers.groups/1` groups, scenes, grouping mode | 04 |
| `keepers cull <dir>` / `keepers analyse <dir>` | `keepers.cull/1` components, composite, auto, decision, reasons | 05 |
| `keepers decide <dir> --frame <i\|id> …`, `keepers undo\|redo <dir>` | `keepers.decide/1` changed frames | 05 |
| `keepers export <dir> --target … [--dry-run]`, `keepers export undo <id>` | `keepers.export.plan/1`, `keepers.export.report/1` | 07 |
| `keepers gates …` | `keepers.gates/1` aggregates only | 08 |
| `keepers second-opinion <dir> --group <g> [--dry-run]` | `keepers.second_opinion.plan/1`, `keepers.second_opinion/1` | 09 |
| `keepers style fit\|show\|reset` | `keepers.style/1` | 10 |

`keepers.cull/1` top level: `{"schema","shoot_id","settings","settings_hash","faces_checked","grouping",
"frames","groups":[{"id","members","best"}],"scenes":[{"id","first","last"}],"summary":{"frames","pick",
"reject","none","user_decisions"}}`. Other documents' fields are in their module specs. One frame, the
shape golden tests compare most:

```json
{"index":3,"frame_id":"0b6f3a9c1d2e4f5061728394a5b6c7d8","path":"bursts/a/IMG_0004.JPG","group":"g0001",
 "scene":"s001","components":{"focus":0.412,"exposure":0.97,"eyes":1.0},"composite":0.6002,
 "auto":{"flag":"reject","reasons":[{"code":"not_best_in_group","text":"a sharper frame of this burst was kept",
 "group":"g0001","best":1,"better_by":"focus"}]},"decision":{"flag":"reject","stars":0,"label":null,"source":"auto"}}
```

**Tauri commands** (errors are `{code, message}`; every argument is validated in the core):

| Command | Arguments | Result | Errors |
|---|---|---|---|
| `app_info` | | `{version, models: ModelStatus[], accelerators}` | |
| `shoot_open` | `{path}` | `{shoot_id, name, frame_count, matched: "new"\|"existing", job_id}` | `not_a_folder`, `no_supported_files`, `io` |
| `shoot_recent` / `shoot_forget` | / `{shoot_id}` | `RecentShoot[]` / `()` (cache only) | `not_found` |
| `analysis_start` / `job_cancel` | `{shoot_id, stages?}` / `{job_id}` | `{job_id}` / `()` | `shoot_not_open`, `not_found` |
| `frames_page` | `{shoot_id, offset, limit ≤ 500, filter, collapse_groups}` | `{total, rows: FrameRow[]}` | `invalid_argument` |
| `frame_detail` / `group_detail` | `{shoot_id, index}` / `{shoot_id, group_id}` | `FrameDetail` / `GroupDetail` | `not_found` |
| `decision_set` | `{shoot_id, frames, change: {flag}\|{stars}\|{label}\|{reset:true}\|{accept_auto:true}}` | `FrameDecision[]` | `invalid_argument`, `journal_write_failed`, `read_only_shoot` |
| `decision_undo` / `decision_redo` | `{shoot_id}` | `FrameDecision[]` | `nothing_to_undo`, `nothing_to_redo` |
| `cull_get` / `cull_set` | `{shoot_id}` / `{shoot_id, settings}` | `CullSettings` / `CullSummary` | `invalid_argument` |
| `models_status` / `models_download` | / `{ids}` | `ModelStatus[]` / `{job_id}` | `offline`, `checksum_mismatch` |
| `export_plan` / `export_run` / `export_undo` | `{shoot_id, options}` / `{plan_id}` / `{export_id}` | `ExportPlan` / `{job_id}` / `ExportReport` | `destination_inside_source`, `plan_stale`, `undo_conflict` |
| `settings_get` / `settings_set` | / `{settings}` | `Settings` | `invalid_argument` |
| `ai_status` | | `AiStatus` | |
| `ai_enable` | `{provider: "publik"\|"own"\|"local", permission_confirmed: bool}` (publik provisions if no credential) | `AiStatus` | `permission_required` (publik or own without it), `no_app_token`, `provision_failed {message}`, `own_key_missing`, `local_unreachable`, `local_no_vision_model` |
| `ai_disable` | | `AiStatus` (`enabled: false`; credentials kept) | |
| `ai_set_own_key` | `{base_url, api_key, model}` | `AiStatus` (the key is stored, never returned) | `invalid_argument` |
| `ai_local_models` / `ai_set_local_model` | `{base_url?}` / `{name}` (runs the multi-image probe) | `LocalModel[] {name, size_bytes, vision: bool\|null, multi_image: bool\|null}` / `AiStatus` | `local_unreachable`, `local_probe_failed` |
| `ai_publik_reconnect` / `ai_publik_disconnect` / `ai_clear_answers` | | `AiStatus` / `AiStatus` / `{removed}` | `provision_failed {message}` |
| `second_opinion_preview` | `{shoot_id, target}`; `target` is `{group_id}`, `{frame_ids}` (2–6) or `{close_calls: true}` | `SecondOpinionPlan {calls, cached_calls, provider, model, route, items: [{group_id\|null, labels, frame_ids, cached}], estimate_micros_each, estimate_micros_total, estimate_measured: bool}` | `too_few_frames`, `needs_consent` |
| `second_opinion_run` | `{shoot_id, target, confirmed: bool, retry_after_invalid?: bool}`; `confirmed` must be true when the plan has more than one uncached call | `{job_id}`; per call `keepers://second-opinion` `{job_id, group_id, ok, result?: SecondOpinion, error?: {code, message, link_url?, link_label?}}`; `keepers://job-done` `{done, total, spent_micros, stopped: null\|"needs_credit"\|"disconnected"\|"rate_limited"\|"busy"\|"timeout"\|"offline"\|"error"}` | `confirmation_required`, `needs_consent`, `too_few_frames` |
| `second_opinion_get` | `{shoot_id, target}` | `SecondOpinion \| null` `{ranking: [{label, frame_id, reason}], confidence, note, provider, model, charge_micros, created_at}` | |
| `open_link` | `{url}` | `()` | `link_not_allowed` (only URLs from the status object) |
| `style_status` / `style_fit` / `style_reset` | | `StyleStatus {status, examples, shoots, skipped_shoots, alpha, changed?}` | `not_enough_history` (fit) |
| `update_check` | | `{available, version, url}` | `offline` |

`AiStatus` (the only AI object the webview sees; no key field exists): `{enabled, provider:
"publik"|"own"|"local"|null, permission_confirmed, disclosure, data_path, publik: {state:
"not_set_up"|"ready"|"needs_credit"|"disconnected", claim_state: "anonymous"|"claimed"|null,
balance_micros, balance_line, cost_sentence, link_label, link_url, last_charge_micros,
week_used_micros, week_budget_micros, week_resets_at}, own: {configured, base_url, model}, local:
{reachable, base_url, model, multi_image}}` (unknown values are `null`; `link_url` is `claim_url`,
`add_credit_url` or, after a 402, `top_up_url`).

Events: `keepers://progress` `{job_id, stage, done, total}`, `keepers://job-done` `{job_id, ok, error?,
result?}`. Images reach the webview through the `keepers` URI scheme only: `thumb/<shoot>/<frame>`,
`preview/<shoot>/<frame>` (largest embedded preview; orientation in a response header), and
`face/<shoot>/<frame>/<n>` (512 px crop).

## 6. Core algorithms

- Container parsing: TIFF 6.0, EXIF 2.32 (CIPA DC-008), DNG 1.7, ISO/IEC 14496-12 (ISO-BMFF); JPEG ITU-T T.81 and JFIF.
- Luma: integer `Y = (54R + 183G + 19B + 128) >> 8` on 8-bit sRGB; sRGB transfer from IEC 61966-2-1.
- Sharpness: variance of the Laplacian (Pech-Pacheco et al., ICPR 2000) globally and as the maximum
  over an 8 × 8 tile grid; Tenengrad (Krotkov, 1987). Motion direction: structure tensor (Bigün and Granlund, 1987).
- Noise: Immerkær, "Fast Noise Variance Estimation", CVIU 1996.
- Faces: YuNet (Wu et al., Machine Intelligence Research, 2023), decoded as OpenCV's `FaceDetectorYN`
  (Apache-2.0, may be ported with attribution); landmarks: MediaPipe Face Mesh (Kartynnik et al., 2019);
  eye aspect ratio (Soukupová and Čech, CVWW 2016).
- Similarity: dHash (Krawetz, "Kind of Like That", 2013); CLIP (Radford et al., 2021); connected
  components by union-find (Tarjan, 1975). Percentiles by the nearest-rank method.
- Style: L2-regularised logistic regression by IRLS (Hastie, Tibshirani and Friedman, ESL §4.4).
- XMP: ISO 16684-1 and the Adobe XMP Specification Part 3 (storage in files, sidecars).

## 7. Invariants (each has a test)

1. **Originals are read-only.** SHA-256 and mtime of every original are unchanged after analyse and
   export (`originals_untouched.rs`, from module 01). `source_rules.rs` fails if `File::create` or a
   write-mode `OpenOptions` appears in `keepers-core` outside `cache/`, `journal/`, `export/`,
   `ingest/thumbs.rs`, `models/download.rs`, `gates/`, `style/` and the settings writer; `export`
   refuses a target path equal to any original's path.
2. **Decisions are the only precious state.** The journal is append-only: after any operation the old
   file is a byte prefix of the new one. Deleting the cache and the index and re-running gives a
   byte-identical `keepers.cull/1` document (after the journal replay).
3. **Explainable.** Every automatic decision carries at least one reason (property test).
4. **Deterministic.** Same files, models and settings give identical output on one machine, whatever
   the enumeration order or thread count; across Windows and Linux within 1e-4 relative. The core
   reads no clock or randomness except through `env.rs`.
5. **The UI never computes a score.** It renders values and reasons the core returns.
6. **No network without a user action.** A full analyse and export with a transport that panics on use passes.
7. **The key never reaches the webview.** No command result, event or log contains it (module 09).
8. **Advice only.** A second opinion never writes the journal; only the user's own click does.
9. **No face recognition.** No face vector is computed or stored; the schema test checks `faces`.
10. **Logs carry no content.** A run on a fixture with distinctive folder and file names leaves a log
    that contains none of them, and no EXIF value.

## 8. Errors, logging and privacy

Per-file problems are values (`file_errors`), never panics; a run finishes with a list. User-facing
messages are spelled out in the module specs and tested. Logs (`tracing`, file in `data/logs/`) record
stage timings, counts, error codes and frame indices; never paths, file names, EXIF values, pixels,
keys or balances. Every network request the app can make, and nothing else (`SECURITY.md`):

1. model downloads from the registry URLs, on the user's click, checksum-verified (03);
2. the optional update check against this repository's public releases API, off in development
   builds, switchable off (11);
3. second opinions, only after the user enabled them and chose a provider: publikhq.com (`/installs`,
   `/installs/revoke`, `/wallet` (at start only when the provider is publik), `/chat/completions` or the
   Gemini route), the user's own base URL, or localhost (`/api/tags`, `/api/show`,
   `/v1/chat/completions`) (09);
4. the builder's self-test: one `POST /installs` to publikhq.com, only when the app is started with
   `PUBLIK_SELF_TEST` set, after which it exits (09).

## 9. Decisions (ADRs written by module 00)

| ADR | Decision | Status | Rejected, and why |
|---|---|---|---|
| 0001 Stack | Tauri 2 + Rust core + CLI + React | Accepted | Electron (memory with thousands of thumbnails); Python sidecar (install size, start-up) |
| 0002 Licence | MIT OR Apache-2.0 | Accepted | copyleft (limits embedding in studio tools) |
| 0003 Storage | derived SQLite cache + append-only journal; originals never written; sidecars only on Export | Accepted | one database for everything (a cache bug could lose decisions) |
| 0004 Model set | YuNet, MediaPipe landmarks + EAR, CLIP ViT-B/32; no aesthetic model in v1 | Proposed until 03 and 04 | InsightFace (non-commercial weights); an eye classifier (needs a licensed training set: maintainer decides) |
| 0005 Sidecars per editor | naming, rating, label and reject mapping, merge rules (module 07) | Proposed until 07 | writing into JPEG/DNG files (out of v1) |
| 0006 Identity | content fingerprints for files, frames and shoots | Accepted | path-based ids (break on rename); full-file hashes (too slow for 6,000 RAW files) |
| 0007 Second-opinion route | chat completions image parts on `publik-balanced`; Gemini `publik-vision` if the probe says so | Proposed until 09 | automatic dialect switching at run time (a retry can charge twice) |

Maintainer decides: the evaluation shoot (08), the eye-state fallback if EAR fails its gate (03), the
listing and app token (09), the app name check and signing certificates (11).

## 10. Platforms

Windows 10 and 11 x64 first (WebView2; per-user NSIS installer; DirectML optional). macOS 12+ on arm64
and x64 from module 11 (DMG; CoreML optional; unsigned until a certificate exists). Linux builds from
CI (AppImage, .deb; webkit2gtk 4.1). Paths are handled as `OsString`; relative paths in JSON use `/`.
Extensions match case-insensitively. Network drives work but are slow; the scan says so when the first
100 files take longer than 10 s.
