# ADR-0006: Content fingerprints for files, frames and shoots

- **Status:** Accepted
- **Date:** 2026-10-27

## Context

A photographer renames the folder, moves it to another drive, adds a second card's files to it,
or opens the same card twice. Decisions must follow the photos through all of that
(`docs/PRODUCT.md` §4, F1 and F10). A path-based identity breaks on the first rename; hashing
every byte of 6,000 RAW files (often 150 GB) takes longer than the cull. RAW+JPEG pairs must be
one frame, and a byte-identical duplicate must not become a second frame.

## Decision

Three identities, all derived from content (`docs/ARCHITECTURE.md` §4):

- `file_id`: the first 16 bytes, as lowercase hex, of SHA-256 over
  `"keepers-file-v1\n"`, the file size as a little-endian u64, and the content, where the content
  is the whole file up to 131,072 bytes, otherwise its first and last 65,536 bytes.
- A **frame** is the set of files with the same directory and the same case-insensitive stem; the
  RAW is primary and a JPEG is its companion. `frame_id` is the primary's `file_id`.
- `shoot_id`: when a folder is opened, the stored shoot whose frame set overlaps the folder's by
  at least 90% of the smaller set is matched (ties: earliest first seen, then by id); otherwise a
  new id is the first 16 bytes hex of SHA-256 over `"keepers-shoot-v1\n"` and the sorted frame
  ids joined by newlines.

## Consequences

- Renaming or moving the folder keeps every decision; adding a card appends frames to the same
  shoot; a duplicate file is skipped and reported.
- Hashing reads at most 128 KB per file, so identity costs a fraction of reading the previews.
- A file edited in place with the same size and unchanged head and tail keeps its id; for
  originals, which are never written, that is the right answer, and the versioned prefixes let a
  future scheme coexist.
- Two shoots that share more than 90% of their frames (an export of a selection, say) match the
  same shoot; that is by design and the home screen shows where it was last opened.

## Alternatives considered

- **Path-based ids**: free to compute. Rejected because a rename or a move loses every decision.
- **Full-file SHA-256**: unambiguous. Rejected because it reads every byte of every RAW, minutes on
  a card and longer on a network drive, for no gain over head, tail and size.
- **EXIF-based ids (camera serial plus shutter count)**: stable across copies. Rejected because
  not every file has them, serial numbers are personal data Keepers never reads, and a JPEG
  companion has its own values.
