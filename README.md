# Keepers

Keepers culls a photo shoot on your own computer. It flags blur, closed eyes and bad exposure,
groups bursts and near-duplicates, marks the best frame of each, explains every decision, and
writes ratings that Lightroom Classic, Capture One and darktable read. Your original files are
never written.

**Status: pre-alpha.** The repository, build, tests and continuous integration exist (module 00
of 12). Nothing culls yet. `docs/ROADMAP.md` lists what comes next and in which order.

## What it replaces

The culling subscriptions: Aftershoot Select, Narrative Select, FilterPixel, and the culling half
of Photo Mechanic. Culling runs on the photographer's computer in all of them, so a free local app
gives up nothing. Keepers adds what none of the open source cullers has: previews from the JPEG
embedded in each RAW so a 3,000-frame card is on screen in seconds, a reason on every automatic
decision, decisions kept in an append-only journal that survives anything, originals never opened
for writing, a keyboard-first review built for 6,000 frames, and export with merge, backup and
undo. `docs/PRODUCT.md` has the features, the screens and the evidence.

## Build from source

Prerequisites:

- Rust, through [rustup](https://rustup.rs). The toolchain in `rust-toolchain.toml` installs itself
  on first use.
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny): `cargo install cargo-deny --locked`.
- Node.js 24 and npm.
- Windows: the Visual Studio C++ Build Tools and the WebView2 runtime (part of Windows 11).
- Linux (Debian or Ubuntu): `libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev
  libssl-dev libayatana-appindicator3-dev librsvg2-dev`. Other distributions: Tauri's
  [prerequisites page](https://tauri.app/start/prerequisites/).
- macOS builds come with module 11.

Then:

```sh
git clone https://github.com/MH1044/keepers
cd keepers
npm ci --prefix app
sh scripts/verify.sh
```

`verify` runs everything continuous integration runs: the front-end build, type check, lint,
format check and tests; Rust format, clippy, the licence gate, the npm licence check, the
fixture generator against its manifest, and every Rust test. On Windows, `scripts\verify.ps1` does
the same from PowerShell.

The command line:

```sh
cargo run -q -p keepers-cli -- --version --json
```

prints `{"schema":"keepers.version/1","name":"keepers","version":"0.1.0"}`.

The desktop window (a title and the version, for now):

```sh
npm --prefix app run build
cargo run -p keepers-app
```

For development with live reload of the window: `npm --prefix app run tauri dev`.

## Layout

| Path | What it is |
|---|---|
| `crates/keepers-core/` | the library: everything that decides anything |
| `crates/keepers-cli/` | the `keepers` command; `tests/golden.rs` is the golden harness |
| `crates/keepers-fixtures/` | the seeded fixture generator, bit-identical on Windows and Linux |
| `app/` | the React window; `app/src-tauri/` the Tauri host that only marshals |
| `tests/cases/` | golden cases, one directory per module |
| `tests/fixtures/` | the fixture manifest; generated files are not committed |
| `scripts/` | `verify.sh`, `verify.ps1`, `release-check.sh` |
| `docs/` | product, architecture, testing, roadmap, one spec per module, decision records |

## Documents

`docs/PRODUCT.md` (what and for whom), `docs/ARCHITECTURE.md` (stack, data model, interfaces,
invariants), `docs/TESTING.md` (test layers, fixtures, the source-hygiene test),
`docs/ROADMAP.md` (modules and status), `docs/modules/` (one specification per module),
`docs/adr/` (decision records), `docs/HANDBOOK.md` (the user guide, grown with each module),
`docs/PUBLIK-INTEGRATION.md` and `docs/PUBLIK-CONTRACT.md` (the optional second opinion through
the Publik API). `CONTRIBUTING.md` has the rules for changes; `SECURITY.md` lists every network
request the app can make; `RELEASING.md` says how a release is cut.

## Licence

MIT OR Apache-2.0, at your option: `LICENSE-MIT` and `LICENSE-APACHE`. Dependencies, models and
datasets are listed with their licences in `THIRD_PARTY.md`.
