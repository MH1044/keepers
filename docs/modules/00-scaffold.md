# 00: Scaffold

## Goal

A contributor can clone the repository, run one `verify` command and see it green on Windows and
Ubuntu; CI proves the same on every push. The repository documents, the hygiene test, the
golden harness with its first case, the seeded fixture generator and a hello-world Tauri window
exist, so module 01 starts with product code only.

## Depends on

Nothing.

## Scope

1. A Cargo workspace with `crates/keepers-core` (library), `crates/keepers-cli` (binary `keepers`),
   `crates/keepers-fixtures` (binary `keepers-fixtures`) and `app/src-tauri` (the Tauri host), and
   `rust-toolchain.toml` pinning the stable toolchain current at this module.
2. A React 19 + TypeScript + Vite app in `app/` with ESLint, Prettier and Vitest, showing a window
   titled "Keepers" with the text "Keepers {version}" from the `app_info` command.
3. `scripts/verify.sh` and `scripts/verify.ps1` running, in order and stopping at the first failure,
   the steps listed in `docs/TESTING.md` §2 (frontend build first, so `frontendDist` exists).
4. The licence gate: `deny.toml` allowing MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib,
   Unicode-3.0, MPL-2.0 and refusing everything else, GPL, AGPL, SSPL and BUSL named as refused;
   the npm checker with the same allow list.
5. The source-hygiene test in `crates/keepers-core/tests/hygiene.rs` as specified in `docs/TESTING.md` §8.
6. `keepers --version --json` printing `{"schema":"keepers.version/1","name":"keepers","version":"<crate version>"}`,
   and the usage error document for an unknown command.
7. The golden harness `crates/keepers-cli/tests/golden.rs` with the `case.toml` format of
   `docs/TESTING.md` §3, its comparison logic unit-tested, and the first case `tests/cases/00-scaffold/version/`.
8. The fixture generator writing `tests/fixtures/gen/basic/` (seed 1) with a minimal in-house EXIF
   writer, `--check <manifest>` and `--write-manifest <manifest>`, bit-identical on Windows and Linux.
9. `keepers_core::env`: the `Clock` trait with `SystemClock` and `FixedClock`, and `CancelToken`.
10. CI `.github/workflows/ci.yml` (`verify` on windows-latest and ubuntu-latest) and
    `.github/workflows/release.yml` whose first step runs `scripts/release-check.sh <version>`.
11. The documents: README, CONTRIBUTING.md, SECURITY.md, RELEASING.md, THIRD_PARTY.md, LICENSE-MIT
    and LICENSE-APACHE, docs/HANDBOOK.md (stub), the ADR index, template and ADRs 0001 to 0007 as
    listed in `docs/ARCHITECTURE.md` §9.

## Files

- `Cargo.toml`, `rust-toolchain.toml`, `deny.toml`, `.gitignore`, `.gitattributes`
- `crates/keepers-core/src/{lib.rs,env.rs,error.rs}`, `crates/keepers-core/tests/hygiene.rs`
- `crates/keepers-cli/src/main.rs`, `crates/keepers-cli/tests/golden.rs`, `crates/keepers-cli/tests/support/compare.rs`
- `crates/keepers-fixtures/src/{main.rs,scene.rs,exif_writer.rs,blur.rs}`
- `app/{package.json,vite.config.ts,tsconfig.json,eslint.config.js,index.html}`, `app/src/{main.tsx,App.tsx,api.ts}`
- `app/src-tauri/{Cargo.toml,build.rs,tauri.conf.json,capabilities/default.json,src/main.rs,src/commands.rs}`
- `scripts/{verify.sh,verify.ps1,release-check.sh}`
- `tests/cases/00-scaffold/`, `tests/fixtures/gen.sha256`, `.github/workflows/{ci.yml,release.yml}`
- the documents in Scope 11

## Interfaces

```rust
// keepers-core::env
pub trait Clock: Send + Sync { fn now_utc(&self) -> Timestamp; }   // RFC 3339, milliseconds
pub struct SystemClock; pub struct FixedClock(pub Timestamp);
#[derive(Clone, Default)] pub struct CancelToken(Arc<AtomicBool>);
impl CancelToken { pub fn cancel(&self); pub fn is_cancelled(&self) -> bool; }
// keepers-cli: KEEPERS_FIXED_CLOCK=<rfc3339> selects FixedClock (tests only)
// Tauri: #[tauri::command] fn app_info() -> AppInfo { version: String }
```

`keepers-fixtures --out <dir> [--seed <u64>] [--check <manifest>] [--write-manifest <manifest>]`;
`--check` exits 1 with "fixture <path> differs from the manifest" on the first mismatch.
`scripts/release-check.sh <version>` exits 1 with "docs/releases/<version>.md is missing; write the
release notes first." CLI usage error, exit 1:
`{"schema":"keepers.error/1","code":"usage","message":"unknown command 'frobnicate'. Run keepers --help."}`.

## Out of scope

- Any product logic (scanning, scores, UI beyond the hello-world window): modules 01 to 07.
- The e2e smoke job (06), the macOS runner (11), the raw-samples and models CI jobs (01, 03).
- Creating the GitHub repository or pushing: needs the maintainer's approval.

## Design notes

- Tauri: `verify` builds the frontend before any `cargo` step; the Ubuntu job installs Tauri's
  documented Linux prerequisites. The app identifier and data directory name are `keepers`.
- The fixture generator uses integer maths for pixels and `libm` for the Gaussian kernel, quantised to
  16-bit fixed point, so outputs are bit-identical; JPEGs come from the `image` encoder at quality 95.
  The base scene is seeded value noise in three octaves plus 12 hard-edged rectangles and circles.
- The EXIF writer emits a big-endian TIFF header in APP1 with IFD0 (Make, Model, Orientation,
  ExifIFD pointer) and the Exif IFD (DateTimeOriginal, SubSecTimeOriginal, OffsetTimeOriginal),
  entries sorted by tag as TIFF 6.0 requires.
- Decisions made during implementation:
  - **Licence gate.** cargo-deny 0.14 and later have no deny list: every licence outside `allow` is
    refused. `deny.toml` therefore names the refused licences in a comment and the test checks that
    none of them is allowed and that the npm check carries the same allow list.
    `Apache-2.0 WITH LLVM-exception` is allowed for `target-lexicon`, a build-time dependency of the
    Linux GTK bindings. The npm check covers production packages (what ships in the window); the
    development tooling is not distributed.
  - **Toolchain.** `rust-toolchain.toml` pins 1.98.1, the stable toolchain installed when module 00
    was built, with `rustfmt` and `clippy` as components; CI installs it with `rustup toolchain
    install`. Bumping is a deliberate change.
  - **Timestamps.** `Timestamp` is milliseconds since the epoch, printed as RFC 3339 with three
    fractional digits and `Z`, parsed from any RFC 3339 form with an offset; the civil-date maths is
    Howard Hinnant's `days_from_civil` and `civil_from_days`, so no date crate is needed.
  - **CLI.** clap with its own version flag disabled; an external subcommand becomes the usage
    document; `--json` is read from the raw arguments so it applies after an unknown command; no
    command at all is the usage error "no command given. Run keepers --help."; `KEEPERS_FIXED_CLOCK`
    is parsed on every run, so a bad value is a usage error everywhere.
  - **Error document.** `keepers_core::error::ErrorDocument` (`schema`, `code`, `message` in that
    order) and `ExitCode` with the codes of `docs/ARCHITECTURE.md` §5.
  - **Golden harness.** Cases are discovered under `tests/cases/<module>/<case>/case.toml`;
    `GOLDEN_FILTER` matches the module directory name; the harness sets `KEEPERS_FIXED_CLOCK` and
    `KEEPERS_DATA_DIR` (the fresh data directory) for every case; a case may name no fixture; the
    comparison rules live in `tests/support/compare.rs`, included with `#[path]` so Cargo does not
    compile it as a test of its own; when no case matches the filter the test fails rather than
    passing vacuously.
  - **Fixture generator.** Each set derives its own stream from the seed and its name (FNV-1a), so
    adding a set later never changes another set's bytes. SplitMix64 is the generator. Value noise
    uses cells of 128, 32 and 8 px weighted 4:2:1 with bilinear interpolation in Q16; twelve shapes
    alternate rectangles and circles. The sRGB curve comes from `libm::pow`, the exposure factor from
    `libm::exp2`, both quantised to Q16; the inverse maps to the nearest code. Gaussian taps are
    `libm::exp` quantised so they sum to exactly 65,536 (largest-remainder rounding); the horizontal
    pass keeps eight fractional bits and the vertical pass rounds once. JPEGs come from `image` 0.25
    at quality 95; the APP1 segment is inserted immediately after SOI, before the encoder's JFIF
    APP0, as EXIF 2.32 places it. The manifest is `<sha256>  <path>` per line, sorted. The dev
    profile builds `keepers-fixtures`, `image` and `zune-jpeg` at opt-level 2 so `verify` does not
    wait on the generator; the output is identical at opt-level 0 and 2 (checked by hash).
  - **EXIF writer.** Values longer than four bytes follow their IFD in tag order, each starting on an
    even offset as TIFF 6.0 requires; the Exif IFD starts on the next even offset after IFD0's
    values. The APP1 payload for the acceptance values is 167 bytes.
  - **Tauri.** The identifier is `keepers` (the single-word form is valid because the GTK
    application id is not registered); placeholder icons are generated and committed; bundling is
    off until module 11; the CSP is unset until the window has content (06).
  - **Scripts.** `verify.ps1` passes clippy's and the generator's `--` arguments with `--%`, since
    Windows PowerShell 5.1 would otherwise swallow them. `release-check.sh` resolves the notes
    relative to its own location, so its test runs it from a temporary copy.
  - **Tests outside the Files list.** `crates/keepers-cli/tests/release_check.rs` covers acceptance
    test 11; the deny.toml check is `licence_gate_refuses_copyleft` in `hygiene.rs`.
  - **Hygiene test.** Key prefixes are matched at a word boundary so ordinary words ending in `sk-`
    cannot trip the secret rule, and `pat_` tokens are refused outside
    `crates/keepers-publik/publik-app-token.txt`. The walk threshold is 45 files.
  - **Deferred.** `tracing` and its subscriber arrive with module 01 beside the first log test;
    `proptest` and `tiny_http` with their first use.

## Acceptance tests

Golden cases in `tests/cases/00-scaffold/`; code tests in `crates/keepers-cli/tests/support/compare.rs`,
`crates/keepers-fixtures/src/exif_writer.rs` (unit) and `crates/keepers-core/tests/hygiene.rs`.
References: TIFF 6.0 and EXIF 2.32 for the EXIF bytes; `docs/TESTING.md` for the harness rules.

1. `verify` on a clean clone exits 0 on windows-latest and ubuntu-latest (the CI run of the commit is green).
2. Case `version`: `keepers --version --json` exits 0; `schema` is `keepers.version/1`, `name` is
   `keepers`, `version` matches `^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$`.
3. Case `unknown-command`: `keepers frobnicate --json` exits 1 with exactly the usage document above.
4. Harness self-tests: actual 1.0005 against expected 1.0 with `abs = 1e-3` passes, 1.002 fails; an
   extra key fails; a missing key fails; an ignored path passes whatever its value; a `[match]` regex
   failure names the pointer; a `[files]` byte difference names the file and first differing offset.
5. `UPDATE_GOLDEN=1` without `GOLDEN_FILTER` exits with "UPDATE_GOLDEN needs GOLDEN_FILTER=<module>".
6. The hygiene test passes on the tree; with an untracked planted temp file (a home path, an email at
   a real-looking domain, a 40-character fake `sk-` key, a control character) it reports exactly four
   failures, each with its remedy text (recorded in the commit body); the file is then deleted.
7. The fixture generator run twice into two directories gives byte-identical trees; `--check` against
   the committed manifest passes on both CI runners.
8. EXIF writer unit test: for Make "Keepers Fixtures", Model "Generator 1", Orientation 1,
   DateTimeOriginal "2026:06:13 15:02:11", SubSecTimeOriginal "340", OffsetTimeOriginal "+01:00" the
   APP1 payload equals the byte string worked out by hand from TIFF 6.0 and EXIF 2.32 (tag order,
   counts, offsets), committed in the test.
9. `FixedClock("2026-01-01T00:00:00.000Z").now_utc()` returns that instant; `CancelToken` clones share
   state.
10. `app_info` unit test returns the crate version; `app/src/App.test.tsx` renders "Keepers 0.1.0"
    with `api.appInfo` mocked; `cargo build -p keepers-app` succeeds on both runners.
11. `scripts/release-check.sh 0.1.0` with no notes file exits 1 with the message above; with the file
    present it exits 0.
12. `cargo deny check licenses` and the npm licence check pass as `verify` steps; `deny.toml` names
    GPL-2.0, GPL-3.0, AGPL-3.0, SSPL-1.0 and BUSL-1.1 as refused and allows none of them, and the
    npm check uses the same allow list (a test reads both files).
13. `required_documents_exist` (in `hygiene.rs`): every file in Scope 11 and `docs/adr/0001` to `0007`
    exists and is non-empty.

## Status

Done (2026-10-27)
