# 11: Packaging and release

## Goal

A photographer can install Keepers without building it: a per-user Windows installer that needs no
administrator password, a macOS DMG and Linux builds, produced by a tag-driven release that refuses to
publish without release notes and green checks. An optional update check says when a new version is
out. The Publik listing is prepared for the maintainer to review; only the maintainer publishes it.

## Depends on

All earlier modules.

## Scope

1. Tauri bundle configuration: Windows NSIS with `installMode: "currentUser"`; macOS DMG for arm64 and
   x64; Linux AppImage and `.deb`. Models are not bundled (the first-run screen downloads them).
2. `macos-latest` added to the `verify` job of CI.
3. `.github/workflows/release.yml` on a `v*` tag: `scripts/release-check.sh <version>`, `verify` on
   all three runners, bundles built, a draft release created with `docs/releases/<version>.md` as its
   body, artifacts uploaded with SHA-256 files, published only when every job is green.
4. `scripts/release-check.sh <version>` also checks that `Cargo.toml` (workspace), `app/package.json`
   and `app/src-tauri/tauri.conf.json` carry the same version as the tag.
5. `RELEASING.md` with the steps and the tester notes: on Windows, "Windows protected your PC" →
   **More info** → **Run anyway**; on macOS, "cannot be opened because the developer cannot be verified"
   → Control-click the app → **Open**, or System Settings > Privacy & Security → **Open Anyway**.
6. The update check: `GET` this repository's public "latest release" API, reading only `tag_name` and
   `html_url`; no identifying header beyond the client's user agent `keepers/<version>`; compiled out
   of debug builds; Settings > General "Check for updates" (on by default in release builds); a
   notice "Keepers {version} is available." with **Open release page**; offline is silent.
7. `THIRD_PARTY.md` complete (every crate, npm package, model and dataset with licence and source),
   generated in part by `cargo deny list` and the npm checker; licence texts bundled in the installers.
8. `docs/releases/0.1.0.md`, `docs/HANDBOOK.md` complete for v1, README install section.
9. The Publik listing prepared from PRODUCT §9 (name, summary with its character count, category,
   alternativeTo, platforms, licence, products) following `https://publikhq.com/skills/publish.md`
   **up to the human review step only**, shown to the maintainer.

## Files

- `app/src-tauri/tauri.conf.json` (bundle section), `.github/workflows/{ci.yml,release.yml}`
- `scripts/release-check.sh`, `RELEASING.md`, `THIRD_PARTY.md`, `README.md`, `docs/HANDBOOK.md`, `docs/releases/0.1.0.md`
- `crates/keepers-core/src/update.rs`, `app/src/views/Settings.tsx`
- `crates/keepers-core/tests/11_packaging_and_release.rs`, `app/src/state/update.test.ts`

## Interfaces

```rust
pub fn check_for_update(current: &Version, http: &dyn HttpGet, url: &str) -> Result<UpdateInfo, UpdateError>;
pub struct UpdateInfo { pub available: bool, pub version: Option<Version>, pub url: Option<String> }
pub enum UpdateError { Offline, BadResponse }   // both silent in the UI
```

Tauri command `update_check() -> {available, version, url}`; the URL is opened through `open_link`,
which accepts only this repository's release page.

## Out of scope

- Code signing and notarisation (**maintainer decides**: certificates cost money; until then installers
  are unsigned and `RELEASING.md` says so); auto-install of updates; app stores; publishing the Publik
  listing (the maintainer does it).

## Design notes

- **Maintainer decides**: the final name (check for conflicts with existing apps and trademarks) and
  whether to buy signing certificates. This module stops until the maintainer has reviewed the listing.
- Further decisions are filled in here during implementation.

## Acceptance tests

Code tests in `crates/keepers-core/tests/11_packaging_and_release.rs`; UI in
`app/src/state/update.test.ts`; the release workflow proven on a `v0.1.0-rc.1` tag. References: the
Tauri bundler documentation for `installMode`, and the GitHub releases API response shape.

1. `release-check.sh 0.1.0` fails with "Version mismatch: tauri.conf.json has 0.0.9, the tag is 0.1.0."
   when one file differs, and with the missing-notes message when `docs/releases/0.1.0.md` is absent.
2. The bundle config test reads `tauri.conf.json`: NSIS `installMode` is `currentUser`; targets include
   `nsis`, `dmg`, `appimage` and `deb`; no model file is listed as a resource.
3. Update check against a localhost fake: `{"tag_name":"v0.2.0","html_url":"https://…"}` with current
   0.1.0 gives `available: true`; `v0.1.0` gives `available: false`; a 500 or garbage body gives
   `BadResponse`; connection refused gives `Offline`; the request carries no header other than
   `User-Agent` and `Accept`.
4. With "Check for updates" off, no request is made (panicking transport); in a debug build the
   function is compiled out (a `cfg(debug_assertions)` test asserts `update_check` returns
   `available: false` without calling the transport).
5. UI: an available update shows "Keepers 0.2.0 is available." with **Open release page**; `Offline`
   shows nothing.
6. `THIRD_PARTY.md` has a row for every crate in `cargo metadata` and every package in
   `app/package-lock.json` production dependencies, and for each model in the registry.
7. The release workflow on `v0.1.0-rc.1` produces the four bundle types and a draft release whose body
   equals `docs/releases/0.1.0-rc.1.md` (checked once; recorded in the commit body).
8. Manual checklist (`docs/TESTING.md` §10, Install): a per-user install on Windows shows no
   administrator prompt and the SmartScreen steps match `RELEASING.md`.
9. CI `verify` is green on macos-latest as well as windows-latest and ubuntu-latest.
10. Documents: `docs/HANDBOOK.md` has a heading for each of F1 to F17; the listing name in PRODUCT §9
    has at most 18 characters and the summary at most 80 (a test reads the table); every command shown
    in `README.md` and the handbook is one the golden or smoke tests run.

## Status

Planned
