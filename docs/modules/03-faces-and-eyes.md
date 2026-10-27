# 03: Faces and eyes

## Goal

Faces are found in every frame, each face gets a sharpness value and an eye state (open, closed,
uncertain or too small), and the frame gets a summary such as "3 faces, 1 with eyes closed". Models
are downloaded on request with their size shown first and are verified by checksum. With no models
installed, everything else still works and the result says faces were not checked.

## Depends on

02.

## Scope

1. The model registry `crates/keepers-core/registry/models.toml` (format in ARCHITECTURE §4) with
   rows for `yunet-2026may` (group `faces`), the face landmark model (group `faces`) and, reserved for
   module 04, the CLIP image encoder (group `similarity`). URL pinned to a commit or release, SHA-256
   and size filled in from the downloaded files; licences recorded in `THIRD_PARTY.md` and ADR-0004.
2. Downloads with `ureq` (rustls): to `<id>/<file>.part`, resumed with a `Range` header, SHA-256
   verified, then renamed; a mismatch deletes the part file. Nothing downloads without a command.
3. `keepers models list|download [--group faces|similarity|all]|verify --json` printing
   `keepers.models/1` `{"models":[{"id","task","group","size_bytes","installed","verified","licence"}]}`.
4. ONNX sessions through `ort` on the CPU execution provider, loaded lazily; a model file is
   re-hashed when its size or mtime changed since the last verification.
5. Face detection: YuNet on the working image at two scales (long edge 640 and 320, because YuNet
   finds faces of about 10 to 300 px), score threshold 0.8, top 50, merged by NMS at IoU 0.3; boxes
   normalised to 0..1 of the oriented preview.
6. Face crops from the full-size embedded preview: box expanded 25% on each side, made square, clamped
   to the image, resized to 192 × 192 for landmarks. A face whose crop is under 64 px wide in the
   preview gets `eyes: "too_small"` and no eye judgement.
7. Face sharpness: `lap_var` (module 02 definition) of the 192 × 192 crop's luma.
8. Eye aspect ratio per eye from the landmark model's eye points (MediaPipe indices: right eye 33,
   160, 158, 133, 153, 144; left eye 362, 385, 387, 263, 373, 380; verify against the model card):
   `EAR = (|p2 − p6| + |p3 − p5|) / (2 |p1 − p4|)`. Eye state: closed if EAR < 0.18, open if ≥ 0.25,
   else uncertain. Face state: `closed` if both eyes closed; `open` if both open; else `uncertain`.
   A face whose narrower eye is less than half the width of the other (turned head) is `uncertain`.
9. Per-frame `faces@1` summary (ARCHITECTURE §4) and the text "{count} faces, {closed} with eyes closed"
   ("1 face", "no faces"); `faces` rows stored without any face vector.
10. `keepers faces <dir> --json` printing `keepers.faces/1`: per frame `{"index","frame_id","checked",
    "count","judged","closed","uncertain","too_small","summary","faces":[{"n","box":[x,y,w,h],
    "det_score","sharpness","ear_left","ear_right","eyes"}]}`.
11. No models: `checked: false`, `reason: "models_missing"`, and the CLI prints on stderr "Face and eye
    checks are off: the face models are not installed. Download them with `keepers models download`."
12. The CC0 set `tests/fixtures/cc0/` with `labels.toml`, `scripts/fetch-models.sh`, and the `models`
    CI job caching files by the registry's hash.

## Files

- `crates/keepers-core/src/models/{mod.rs,registry.rs,download.rs,session.rs}`, `crates/keepers-core/registry/models.toml`
- `crates/keepers-core/src/faces/{mod.rs,yunet.rs,nms.rs,crop.rs,landmarks.rs,eyes.rs}`
- `crates/keepers-cli/src/commands/{models.rs,faces.rs}`, `scripts/fetch-models.sh`
- `crates/keepers-core/tests/03_faces_and_eyes.rs`, `tests/cases/03-faces-and-eyes/`, `tests/fixtures/cc0/`

## Interfaces

```rust
pub struct Registry { pub models: Vec<ModelEntry> }
pub fn download(entry: &ModelEntry, dir: &Path, http: &dyn HttpGet, progress: &dyn Progress,
    cancel: &CancelToken) -> Result<PathBuf, ModelError>;
pub fn verify(entry: &ModelEntry, dir: &Path) -> Result<(), ModelError>;
pub fn detect_faces(session: &YuNet, working: &RgbImage) -> Vec<FaceBox>;
pub fn eye_state(landmarks: &[Point2]) -> (Option<f64>, Option<f64>, EyeState); // 468 or 478 points
pub fn faces_shoot(cache: &mut ShootCache, models: &Models, progress: &dyn Progress,
    cancel: &CancelToken) -> Result<FacesSummary, FacesError>;
pub enum ModelError { Offline, ChecksumMismatch { id: String }, Damaged { id: String }, Io(io::Error), Http(u16) }
```

Messages: `ChecksumMismatch`: "The download of {id} did not match its checksum and was deleted. Try
again."; `Damaged`: "Model {id} is damaged (checksum mismatch). Download it again in Settings > Models.";
`Offline`: "Could not download models: no connection."; `Http(code)`: "Could not download {id}: the
server answered {code}."

## Out of scope

- CLIP embeddings and grouping (04); using eye states in the cull (05); the models screen (06);
  DirectML and CoreML (08); face recognition, face vectors or naming people (never).

## Design notes

- Prefer `face_detection_yunet_2026may.onnx` (dynamic input shape, per the OpenCV Zoo README); if it
  does not load in the pinned ONNX Runtime, use `2023mar` with 640 × 640 letterboxing and record why.
- YuNet output decoding follows OpenCV's `FaceDetectorYN` (Apache-2.0; may be ported with attribution).
- The landmark model's ONNX source and licence are chosen here. **Maintainer decides** (ADR-0004) if
  no permissively licensed conversion exists, or if EAR misses the 90% gate: the fallback is a small
  eye classifier, which needs a training set with a usable licence.
- Thresholds 0.18 and 0.25 are starting values; any change is justified here with the CC0 results.
- Further decisions are filled in here during implementation.

## Acceptance tests

Golden cases in `tests/cases/03-faces-and-eyes/`; code tests in
`crates/keepers-core/tests/03_faces_and_eyes.rs`. References: Soukupová and Čech 2016 for EAR, the
IoU and NMS definitions, `labels.toml` written by a person before any code ran, and SHA-256 (FIPS 180-4).

1. EAR unit test: eye points p1 (0,0), p4 (10,0), p2 (3,1.5), p6 (3,−1.5), p3 (7,1.5), p5 (7,−1.5) give
   EAR = (3 + 3) / 20 = 0.30 → open; heights 1.0 and 1.0 give 0.10 → closed; heights 2.2 and 2.2 give
   0.22 → uncertain.
2. Turned head: right-eye width 10 and left-eye width 4 give `uncertain` whatever the EARs.
3. NMS unit test: boxes (0,0,10,10) score 0.9 and (1,1,10,10) score 0.8 (IoU 81/119 ≈ 0.68) keep the
   first only; (0,0,10,10) and (20,20,10,10) keep both.
4. Crop geometry: a face box (100, 100, 40, 40) in a 1000 × 800 preview gives the square crop
   (90, 90, 60, 60) before resizing; a box at the image corner is clamped inside the image.
5. `too_small`: a box 50 px wide in the preview (crop under 64 px) gives `eyes: "too_small"` and
   `ear_left: null`.
6. CC0 face counts (needs models; skips otherwise): each image's `count` equals `labels.toml` for images
   labelled "clear" (all faces frontal, larger than 80 px).
7. CC0 eye states (needs models): at least 90% of the labelled eyes of judged faces match; closed-eye
   labelled faces are never reported `open`.
8. Face sharpness: the CC0 portrait blurred with σ 3 (in memory) has lower face sharpness than the
   original for every detected face.
9. Download from a localhost fake server: the right bytes install and verify; wrong bytes give the
   `ChecksumMismatch` message and leave no file; a connection dropped at half resumes with a `Range`
   request and completes; connection refused gives `Offline`; 404 gives the `Http` message.
10. A copy of an installed model with one byte flipped fails `verify` and loading with the `Damaged` message.
11. No models installed: `keepers faces gen/basic --json` exits 0 with `checked: false`,
    `reason: "models_missing"` on every frame and the stderr line above.
12. Case `models-list`: `keepers models list --json` in an empty data directory lists every registry
    entry with `installed: false`; sizes equal the registry.
13. Schema test: the `faces` table has no BLOB column and no column named like `embedding` or `vector`.
14. Summary text: counts (0,0), (1,0), (3,1) give "no faces", "1 face, 0 with eyes closed",
    "3 faces, 1 with eyes closed".
15. No test performs a real download: the HTTP client is injected and a test fails if a non-localhost
    host is requested.
16. `originals_untouched.rs` passes after `faces`.

## Status

Planned
