# 01: Ingest and previews

## Goal

A user (through `keepers scan`, and later the app) points Keepers at a folder or card and gets every
supported frame listed with its EXIF, a stable identity and a 256 px thumbnail taken from the preview
embedded in the file, in seconds, without the originals ever being opened for writing. Module 02 gets
a function that turns any frame into a 1024 px working image.

## Depends on

00.

## Scope

1. Recursive scan (depth ≤ 16) with a case-insensitive extension allow-list: `jpg jpeg cr2 cr3 nef
   arw raf orf rw2 dng`. Skips names starting with `.`, `@eaDir`, `$RECYCLE.BIN`, `System Volume
   Information`; never follows symbolic links.
2. `file_id`, frame pairing (same directory and case-insensitive stem; RAW primary, JPEG companion)
   and `frame_id` exactly as `docs/ARCHITECTURE.md` §4. A byte-identical second file (same `file_id`)
   is skipped with `duplicate_file`. Two RAWs with one stem are separate frames and the JPEG stays alone.
3. Shoot matching (ADR-0006) against `index.sqlite`, the journal's `shoot` line for a new shoot and a
   `frames_added` line for frames new to a known shoot; `index.sqlite` rebuilt from journals when missing.
4. EXIF: Make 0x010F, Model 0x0110, Orientation 0x0112, DateTimeOriginal 0x9003, SubSecTimeOriginal
   0x9291, OffsetTimeOriginal 0x9011, ExposureTime 0x829A, FNumber 0x829D, PhotographicSensitivity
   0x8827, FocalLength 0x920A, LensModel 0xA434. The GPS IFD (0x8825) and serial-number tags (0xA431,
   0xA435) are never followed or decoded.
5. Preview extraction per format (table below): every JPEG candidate is checked (starts `FF D8`, has a
   SOF marker, fits inside the file) and the largest by SOF pixel count is chosen; invalid candidates
   fall back to the next.
6. Orientation 1 to 8 applied to thumbnails and to the working image; reported sizes are oriented.
7. Thumbnails: 256 px long edge, JPEG quality 80, written to `thumbs/<frame_id>.jpg` through `tmp/` and
   a rename, so a cancelled run never leaves half a file.
8. `working_image(frame) -> RgbImage`: preview decoded, oriented, resized to 1024 px long edge with the
   `image` crate's `Triangle` filter (never upscaled). Not stored.
9. The cache tables `meta`, `files`, `frames`, `file_errors` (ARCHITECTURE §4); a `user_version`
   mismatch deletes and rebuilds the cache.
10. Incremental rescan: a file whose size and mtime are unchanged is not read again.
11. Per-file errors never stop the run (messages below). Folder errors stop it.
12. `keepers scan <dir> --json [--data-dir D] [--progress jsonl] [--strict]` printing `keepers.scan/1`.
13. Progress as JSON Lines on stderr: `{"event":"stage","stage":"scan","total":N}`,
    `{"event":"progress","stage":"scan","done":n,"total":N}`, `{"event":"done","stage":"scan"}`.
14. Cancellation through `CancelToken`; the CLI maps Ctrl-C to it and exits 5 with code `cancelled`.
15. `originals_untouched.rs`, `source_rules.rs` and the log-content test (`docs/TESTING.md` §7).
16. Fixture generator sets `exif/`, `containers/`, `pairs/`; `tests/fixtures/raw-samples.toml`,
    `scripts/fetch-raw-samples.sh` and the `raw-samples` CI job.

Preview locations (verify each against format documentation at this module; record sources):

| Format | Where the largest preview is |
|---|---|
| JPEG | the file itself |
| CR2 | IFD0 strip (StripOffsets 0x0111 / StripByteCounts 0x0117, old-style JPEG); IFD1 thumbnail |
| NEF | SubIFDs (0x014A) with JPEGInterchangeFormat 0x0201 / length 0x0202 |
| ARW | IFD0 JPEGInterchangeFormat; SubIFDs if present |
| ORF | TIFF variant with magic `IIRO`/`MMOR`; preview in the maker note's camera-settings IFD |
| RW2 | TIFF variant with magic `IIU\0`; JpgFromRaw tag 0x002E (a JPEG with its own EXIF) |
| DNG | IFD0 and SubIFDs with NewSubfileType 1, Compression 7 and PhotometricInterpretation 2 or 6 (never CFA or LinearRaw data) |
| CR3 | ISO-BMFF: `moov` → `uuid 85c0b687-820f-11e0-8111-f4ce462b6a48` (CMT1/CMT2 EXIF, THMB); `uuid eaf42b5e-1c98-4b88-b9fb-b7dc406e4d16` → PRVW; track 1 in `mdat` (full-size JPEG via `stco`/`co64` and `stsz`) |
| RAF | magic `FUJIFILMCCD-RAW `; big-endian u32 JPEG offset at byte 84 and length at 88; EXIF inside that JPEG |

## Files

- `crates/keepers-core/src/ingest/{mod.rs,scan.rs,identity.rs,tiff.rs,bmff.rs,raf.rs,jpeg.rs,exif.rs,preview.rs,thumbs.rs}`
- `crates/keepers-core/src/cache/{mod.rs,schema.sql,index.sql}`, `crates/keepers-core/src/journal/{event.rs,append.rs}` (the `shoot` and `frames_added` events only)
- `crates/keepers-core/src/progress.rs`, `crates/keepers-cli/src/commands/scan.rs`
- `crates/keepers-core/tests/{01_ingest_and_previews.rs,originals_untouched.rs,source_rules.rs}`
- `crates/keepers-fixtures/src/{containers.rs,exif_sets.rs}`, `tests/fixtures/raw-samples.toml`, `scripts/fetch-raw-samples.sh`

## Interfaces

```rust
pub fn scan(root: &Path, opts: &ScanOptions, env: &Env, progress: &dyn Progress, cancel: &CancelToken)
    -> Result<ScanResult, ScanError>;
pub struct ScanOptions { pub data_dir: DataDir, pub threads: usize }
pub struct ScanResult { pub shoot_id: ShootId, pub matched: Matched /* New | Existing */,
    pub frames: Vec<FrameInfo>, pub errors: Vec<FileError>, pub cancelled: bool }
pub struct FileError { pub rel_path: String, pub code: FileErrorCode, pub message: String }
pub fn working_image(cache: &ShootCache, frame: &FrameId) -> Result<RgbImage, PreviewError>;
pub fn read_preview_bytes(cache: &ShootCache, frame: &FrameId) -> Result<(Vec<u8>, Orientation), PreviewError>;
pub enum ScanError { NotAFolder(PathBuf), NoSupportedFiles, Io(io::Error), JournalWrite(io::Error) }
```

Messages (`{name}` is the path relative to the shoot root, with `/`):

| Code | Message |
|---|---|
| `unsupported_format` | "{name}: this file's contents do not match its type; skipped." |
| `truncated_file` | "{name}: the file ends early (truncated); skipped." |
| `corrupt_container` | "{name}: the file structure is damaged; skipped." |
| `no_preview` | "{name}: no usable embedded preview was found; skipped." |
| `unreadable` | "{name}: could not be read ({os error}); skipped." |
| `duplicate_file` | "{name}: identical to {other}; skipped." |
| `not_a_folder` | "Not a folder: {path}" (exit 1) |
| `no_supported_files` | "No supported photos in this folder (JPEG, CR2, CR3, NEF, ARW, RAF, ORF, RW2, DNG)." (exit 1) |

`keepers.scan/1` frame: `{"index","frame_id","files":[{"path","format","role"}],"capture_time",
"capture_offset","make","model","lens","focal_mm","shutter_s","aperture","iso","orientation",
"preview":{"width","height","source"}}` (absent EXIF values are `null`); top level `{"schema",
"shoot_id","matched","frame_count","frames","errors"}`.

## Out of scope

- Any score or face work (02, 03); full RAW decoding (never in v1); HEIF; writing anything into the
  source folder (07); UI (06).

## Design notes

- Parsers take `&[u8]` windows read through one `OriginalReader` that opens files read-only; every
  offset and length is checked before use, so random bytes return an error, never a panic.
- `capture_ms` treats the camera's local time as UTC; it is used only for gaps and ordering.
- Filled in here during implementation: whether `kamadak-exif` is used for tag decoding, the source of
  each preview location in the table, and any further decisions.

## Acceptance tests

Golden cases in `tests/cases/01-ingest-and-previews/`; code tests in
`crates/keepers-core/tests/01_ingest_and_previews.rs`. Expected values come from each fixture's
`truth.toml` (by construction), the TIFF/EXIF/ISO-BMFF specifications, and for real samples the
recorded ExifTool values in `raw-samples.toml`.

1. Case `exif`: `keepers scan gen/exif --json` lists 8 frames with Make, Model, LensModel, times,
   sub-seconds, offsets, shutter, aperture, ISO and focal length equal to `truth.toml`; the frame with
   no date has `capture_time: null` and sorts last.
2. The `exif/` frame carrying GPS and BodySerialNumber tags produces no key or value containing its
   planted latitude or serial string anywhere in the output (searched as text).
3. Orientation 6 and 8 frames report width and height swapped relative to the stored pixels; the
   thumbnail of orientation 3 has its planted top-left marker at the bottom right.
4. Case `containers`: each synthetic family file yields one frame whose preview is 1536 × 1024 (not the
   160 × 120 one), with `preview.source` naming the location in the table.
5. `containers/broken/`: a truncated CR3 gives `truncated_file`, a `.cr2` holding PNG bytes gives
   `unsupported_format`, a DNG whose only JPEG is CFA data gives `no_preview`; the other frames are still
   listed and the exit code is 0 (2 with `--strict`); each message matches the table.
6. Case `pairs`: `IMG_0001.dng` + `IMG_0001.jpg` are one frame, primary the DNG, companion the JPEG; the
   byte-identical copy gives `duplicate_file` naming the original.
7. `file_id` of a 100-byte file and of a 300,000-byte file equal SHA-256 values computed by hand in the
   test from the definition (prefix, size, first and last 65,536 bytes).
8. Renaming the fixture folder and scanning again gives the same `shoot_id` and `matched: "existing"`;
   adding 10% new frames keeps the id and appends one `frames_added` line; a disjoint folder gets a new id.
9. Deleting `index.sqlite` and scanning a known folder again matches the same shoot (rebuilt from the journal).
10. Incremental rescan: the second scan of an unchanged folder reads no preview bytes (instrumented
    reader counter is 0); touching one file's mtime re-reads that file only.
11. Property test (proptest, 2,000 cases, fixed seed): random byte strings and single-byte mutations of
    each `containers/` file, fed to every parser, never panic and never read outside the buffer.
12. Cancelling after the progress callback reports 5 of 40 returns `cancelled: true` within 1 s, leaves no
    file in `tmp/`, and every thumbnail present decodes.
13. JSON Lines progress: stderr lines parse as JSON; the last `progress` event has `done == total`.
14. Folder errors: a file path gives `not_a_folder`; an empty folder gives `no_supported_files`; exit 1.
15. `originals_untouched.rs`: SHA-256 and mtime of every fixture file are unchanged after `scan`.
16. `source_rules.rs`: write-capable file APIs appear only in the allowed modules; the test fails on a
    planted `File::create` in `ingest/` (checked against a temporary copy of the source, not committed).
17. Log content: the renamed-private-folder run leaves a log without the planted folder or file names.
18. Real samples (skipped with the message in `docs/TESTING.md` §4 when absent): for each entry in
    `raw-samples.toml` the make, model and preview size equal the recorded values.
19. `working_image` of the 1536 × 1024 `basic/` base scene is 1024 × 683; of a 640 × 480 JPEG it is
    640 × 480 (never upscaled).
20. A cache written with `user_version = 0` is deleted and rebuilt on open, and the scan result is unchanged.
21. Walk rules (temp tree built in the test): files under `.hidden/`, `@eaDir/` and at depth 17 are not
    listed; a symbolic link to another fixture folder is not followed (skipped on Windows runners
    without symlink rights, with a message); `IMG_0001.JPG` and `img_0002.Jpeg` are both found.

## Status

Planned
