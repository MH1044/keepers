# ADR-0003: Derived SQLite cache plus an append-only decision journal; originals never written; sidecars only on Export

- **Status:** Accepted
- **Date:** 2026-10-27

## Context

A photographer's decisions (picks, rejects, stars, labels) are the only thing Keepers produces
that cannot be recomputed. Everything else (EXIF, thumbnails, scores, faces, groups, the cull) is
derived from the files and the models and can be rebuilt. A single database that holds both means
one corrupt page, one schema migration gone wrong or one "clear cache" button can lose an
evening's work. The originals themselves must never be opened for writing: writing into a RAW or
a JPEG is irreversible and unverifiable, and editors already read XMP sidecars
(`docs/PRODUCT.md` §4, F10 and F11; `docs/ARCHITECTURE.md` §4 and §7).

## Decision

Three kinds of state, each in its own place (`docs/ARCHITECTURE.md` §4):

1. **The decision journal**, `decisions/<shoot_id>.jsonl`: one JSON object per line, UTF-8, LF,
   append-only, `fsync` after each append, `seq` increasing by one. It is the source of truth
   for decisions and cull settings. Undo appends an `undo` event; nothing is ever rewritten. A
   torn last line is skipped on read with a warning.
2. **Derived caches**: `cache/shoots/<shoot>/cache.sqlite` and thumbnails per shoot, and
   `data/index.sqlite` for the list of shoots. Any `user_version` mismatch deletes and rebuilds
   the cache; the index is rebuilt from the journals when missing. Deleting both and re-running
   gives a byte-identical cull document after the journal replay.
3. **Originals are read-only.** Keepers writes XMP sidecars only on an explicit Export, backs up
   any sidecar it changes, keeps an undo log, and refuses a target path equal to an original's.
   A test (`originals_untouched.rs`) hashes every fixture original before and after every
   operation; another (`source_rules.rs`) fails if a write-capable file API appears in the core
   outside the allowed modules.

## Consequences

- Decisions survive cache deletion, schema changes, crashes mid-write and a moved folder.
- Everything derived can be thrown away, so cache bugs are cheap: "clear the cache" is always
  safe.
- Replay is a cost: opening a shoot with a long journal replays it; the journal of a 6,000-frame
  wedding is a few thousand lines and replays in milliseconds, but a journal is never compacted,
  so it only grows.
- Undo is an event, so the journal records what the user undid; that is also what style learning
  (module 10) learns from.
- Writing ratings into JPEG or DNG metadata, which some users will ask for, is out of v1 and
  would need its own record.

## Alternatives considered

- **One SQLite database for everything** (decisions as a table beside the caches): simplest to
  query. Rejected because a cache bug, a migration or a "clear cache" could lose decisions, and
  because a single file cannot be both freely deletable and precious.
- **Decisions in the sidecars only**: what the editors read anyway. Rejected because sidecars are
  shared with other programs that rewrite them, cannot record undo or cull settings, and do not
  exist until Export.
- **Writing into the originals**: no sidecar clutter. Rejected because it is irreversible, differs
  per format, and breaks the one promise a culler must keep.
