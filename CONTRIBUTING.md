# Contributing

Keepers is built one module at a time: one specification in `docs/modules/`, one test directory
under `tests/cases/`, one commit. `docs/ROADMAP.md` says which module is next. Read
`docs/ARCHITECTURE.md` and `docs/TESTING.md` before changing anything; the rules below are the
ones that are easy to break by accident.

## The clean-room rule

Keepers is permissively licensed (MIT OR Apache-2.0). Never have the source of a GPL, AGPL, SSPL,
BUSL or other source-available project open, in an editor, a browser tab or an AI assistant's
context, while writing the corresponding part of Keepers. That includes exiv2, ExifTool, LibRaw,
rawloader, darktable and every other GPL image tool. Documentation, papers, issue trackers and
file-format specifications are fine. Permissive prior art (MIT, BSD, ISC, Apache-2.0, MPL-2.0,
CC0) may be studied and ported with attribution in `THIRD_PARTY.md`.

## Dependencies

- Every new dependency needs a one-line justification in the commit body and a row in
  `THIRD_PARTY.md`. Prefer boring, widely used, permissively licensed packages with more than one
  maintainer.
- The licence gate (`cargo deny check licenses` and the npm check in `app/package.json`) allows
  MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0 and MPL-2.0, and refuses
  everything else, transitive dependencies included: GPL, AGPL, SSPL and BUSL in particular. A
  change to that list needs a decision record that supersedes `docs/adr/0002-licence.md`. The
  npm check covers the packages that ship in the window; development tooling is not distributed.
- Models and datasets are dependencies too: each has a row in `THIRD_PARTY.md` with its licence
  and source. Weights that are non-commercial or research-only are never shipped. Model files are
  never committed; the registry pins each file's URL and SHA-256 and the app downloads on the
  user's click.
- A dependency with a single maintainer gets a decision record naming its exit.

## Determinism

- Core code never reads the clock, randomness, the filesystem or the network directly. They are
  injected (`keepers_core::env` for the clock and cancellation).
- Same files, models and settings give identical output on one machine, whatever the enumeration
  order or thread count, and output within 1e-4 relative across Windows and Linux.
- Fixtures are generated with integer maths; the few transcendental values come from `libm` and
  are quantised to fixed point, so every byte is identical on Windows and Linux. The committed
  manifest `tests/fixtures/gen.sha256` proves it on every push.
- Floats are compared with a stated tolerance, never with `==`.

## Tests

- `sh scripts/verify.sh` (or `scripts\verify.ps1`) must be green before every commit. It runs
  exactly what continuous integration runs minus the slow end-to-end smoke test.
- Golden files are the specification, not a cache. Regenerate an expected file only for a new
  case, or for a change the specification and the commit body describe, and only filtered to the
  module being worked on (`UPDATE_GOLDEN=1 GOLDEN_FILTER=<module>`). Read the diff line by line.
- Expected values come from a reference, never from running the code: a published algorithm, a
  reference implementation run once and recorded with its version, or a fixture whose truth is
  known by construction (`truth.toml` beside each generated set).
- Every new feature and every new user-facing error message gets at least one test.
- No test reaches the network. Every HTTP call is faked.
- The source-hygiene test (`docs/TESTING.md` §8) fails the build on a home-directory path, an
  email address, a secret-shaped string or a control character in any file that could be
  published. Use paths relative to the repository root and the documentation domains
  (`example.com`, `.invalid`, `.test`).

## Commits

- One logical change per commit, revertable on its own, with a conventional subject: `feat`,
  `fix`, `test`, `docs`, `chore`, `refactor`, `perf` or `ci`, scoped by module slug
  (`feat(01-ingest-and-previews): ...`).
- The body says what changed and why, in product terms, with one bullet per feature, then a
  `Spec:` line and a `Tests:` line.
- Nothing that identifies a person or a machine goes into a file or a commit message: no home
  paths, no email addresses but GitHub noreply ones, no keys, no hostnames.

## Quality gates

`keepers gates` and `scripts/gates.sh` (module 08) run the numbers a release is defined by:
throughput, time to first thumbnails, peak memory, reject precision and recall, best-frame
agreement, eye-state accuracy. They are too slow for `verify`; run them before a release and when
touching scoring, faces, grouping or ranking. Thresholds are recorded here, with the date and the
machine class, once first measured.

## publik API
- Base URL: https://publikhq.com/api/v1 (OpenAI Chat Completions, Responses, Anthropic Messages)
- Models: publik-fast, publik-balanced, publik-smart only; never a vendor model name
- Key: per-install `pk_` from POST /installs, stored at the publik credential path; env PUBLIK_API_KEY / PUBLIK_API_BASE_URL override
- App token: publik-app-token.txt (public by design, committed) or PUBLIK_APP_TOKEN
- 402: show error.message plus error.top_up_url only
- Never print or log a key
- Self-test: PUBLIK_SELF_TEST=1 makes the built app print one `publik self-test:` line and exit
- Docs: https://publikhq.com/llms.txt, https://publikhq.com/developers, https://publikhq.com/skills/setup.md
- Keepers: the only AI feature is the second opinion (F15), off by default, product ai.chat only.
  Tier publik-balanced on /chat/completions with image parts; the Gemini publik-vision route only if
  ADR-0007 says so. Never publik-smart in v1. No retries on publik calls; own key retries twice.
  A second opinion never writes the decision journal.
