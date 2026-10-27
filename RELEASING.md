# Releasing

Releases are cut by pushing a tag. There is no release yet: installers arrive with module 11
(packaging and release). Until then this file says what the release workflow already does and
what it will do.

## How a release is cut

1. Write the release notes at `docs/releases/<version>.md`, for example `docs/releases/0.1.0.md`.
   The notes say what a user can now do, what changed in files Keepers writes (sidecars, the
   journal), and what is known to be broken.
2. Make sure `main` is green on every runner and `sh scripts/verify.sh` is green locally.
3. Set the version in `Cargo.toml` (`[workspace.package]`) and `app/package.json`, commit.
4. Tag and push: `git tag v<version>` then `git push origin v<version>`.

The release workflow (`.github/workflows/release.yml`) starts with
`scripts/release-check.sh <version>`, which refuses to go on when `docs/releases/<version>.md` is
missing. With module 11 it will then run `verify` on every runner, build the installers, draft a
GitHub release from the notes, upload the installers, and publish only when every step is green.

## What a tester sees with an unsigned installer

Installers are unsigned until a signing certificate exists.

- **Windows**: SmartScreen shows "Windows protected your PC". Click "More info", then "Run
  anyway". The installer is per-user and asks for no administrator password.
- **macOS** (from module 11): Gatekeeper says the app "cannot be opened because the developer
  cannot be verified". Right-click the app, choose "Open", and confirm; or allow it in System
  Settings > Privacy & Security.
- **Linux**: AppImages need the executable bit (`chmod +x`); `.deb` packages install with the
  usual package tools.

A checksum file is published beside each installer so a tester can verify the download.

## Before a release

Run the quality gates (`sh scripts/gates.sh`, module 08) and the manual checklist in
`docs/TESTING.md` §10 on a real shoot of your own.
