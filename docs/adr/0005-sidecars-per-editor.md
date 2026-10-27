# ADR-0005: Sidecar naming, rating, label and reject mapping and merge rules per editor

- **Status:** Proposed until module 07 (the byte-exact examples per editor are verified against
  the editors' own documentation and one real export each)
- **Date:** 2026-10-27

## Context

The output of a cull is what the photographer's editor reads. Lightroom Classic, Capture One and
darktable all read XMP sidecars, but they differ in the sidecar's name for a given file, in
which namespace carries the rating, the colour label and the reject flag, in how a reject is
expressed, and in how much of a foreign sidecar they keep (`docs/PRODUCT.md` §4, F11;
`docs/modules/07-export.md`). A sidecar may already exist, written by the editor or by another
tool, and it must keep every field Keepers does not own.

## Decision

Export writes one XMP sidecar per frame for the chosen target editor, with the naming and the
field mapping recorded per editor in `docs/modules/07-export.md`, each with a byte-exact example.
Ratings map to `xmp:Rating`, labels to the editor's label vocabulary, and rejects to the editor's
own convention (a rating of −1 where the editor uses it, otherwise the editor's flag). An existing
sidecar is merged: Keepers rewrites only the elements it owns and keeps every other element,
namespace and attribute byte for byte, using an event stream (`quick-xml`) rather than a tree
rewrite. Every changed sidecar is backed up first, the export is recorded in an undo log, and
undo restores the prior files. A dry run reports what will be created, merged and backed up
before anything is written.

## Consequences

- A photographer sees the same picks in whichever editor they open, with their existing edits
  and keywords intact.
- The mapping table is three editors wide and must be verified by a real export in each at
  module 07; a vendor documentation change is a new record.
- Byte-for-byte preservation of foreign content means Keepers never "cleans up" a sidecar, even
  a malformed one it could repair.
- Writing into JPEG or DNG files, which some editors prefer for those formats, is out of v1.

## Alternatives considered

- **One generic sidecar for all editors**: simpler. Rejected because the editors disagree on
  names, labels and rejects, and a sidecar that one editor ignores is a silent failure.
- **Rewriting sidecars through an XML tree**: easier to implement. Rejected because tree
  libraries normalise whitespace, attribute order and namespace prefixes, which editors then see
  as changes.
- **Writing into the originals' metadata**: no sidecar files to manage. Rejected by ADR-0003.
