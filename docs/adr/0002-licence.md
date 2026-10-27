# ADR-0002: MIT OR Apache-2.0, with a licence gate that refuses copyleft dependencies

- **Status:** Accepted
- **Date:** 2026-10-27

## Context

Keepers replaces paid culling tools and wants to be embedded: in a studio's own scripts, in a
tethering setup, next to LightCraft or any other raw developer that reads its sidecars
(`docs/PRODUCT.md` §2). Many of the best image tools are GPL (exiv2, ExifTool, LibRaw,
darktable), so the choice of licence decides which prior art may be read while writing the
readers, and the licence of every dependency, model and dataset must be checked on every push or
it drifts.

## Decision

Keepers is dual-licensed MIT OR Apache-2.0, at the user's option, with
`Copyright (c) 2026 The Keepers contributors`. A licence gate runs in `verify` and in continuous
integration: `cargo deny check licenses` for Rust and `license-checker-rseidelsohn` for the npm
packages that ship in the window, both allowing exactly MIT, Apache-2.0, BSD-2-Clause,
BSD-3-Clause, ISC, Zlib, Unicode-3.0 and MPL-2.0 (plus `Apache-2.0 WITH LLVM-exception`, an
Apache licence whose exception concerns only LLVM binaries) and refusing everything else,
transitive dependencies included. GPL-2.0, GPL-3.0, AGPL-3.0, SSPL-1.0 and BUSL-1.1 are named in
`deny.toml` as refused, and a test checks none of them is allowed. The clean-room rule in
`CONTRIBUTING.md` follows: no GPL or source-available code is open while the corresponding part
of Keepers is written. Models and datasets follow the same gate by hand, with a row in
`THIRD_PARTY.md`; non-commercial weights are never shipped.

## Consequences

- Anyone may embed Keepers, including in closed tools; a studio can ship it inside its own
  workflow without legal review.
- The in-house TIFF, ISO-BMFF and RAF readers are written from the format specifications, not
  ported from exiv2, ExifTool or LibRaw. That is more work and the reason module 01 verifies each
  preview location against documentation and real samples.
- A dependency with a copyleft licence cannot be added, even for a build-time tool that links in,
  without superseding this record. MPL-2.0 is allowed because it is file-level copyleft that does
  not reach Keepers' own files.
- Changing the allow list is a new decision record, not an edit of `deny.toml`.

## Alternatives considered

- **GPL-3.0**: would allow porting from exiv2, LibRaw and darktable, and would make the readers
  faster to write. Rejected because it limits embedding in studio tools and closed workflows,
  which is where a culler is used.
- **A single licence (MIT only or Apache-2.0 only)**: simpler to state. Rejected because the
  Rust ecosystem's convention is the dual licence, which lets Apache users keep the patent grant
  and MIT users keep the short text.
- **Allowing any OSI-approved licence**: fewer gate failures. Rejected because "OSI-approved"
  includes copyleft licences and the gate would have to be read by a person every time.
