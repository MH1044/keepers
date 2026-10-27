# 07: Export

## Goal

The cull lands in the photographer's editor: ratings, colour labels and rejects are written as XMP
sidecars that Lightroom Classic, Capture One or darktable read, existing sidecars are merged without
losing anything Keepers does not own, prior versions are backed up, and the last export can be undone.
Keepers can also copy or move the keepers (and optionally the rejects) to another folder. Nothing is
written before the user has seen a dry-run summary, and no original file is ever changed.

## Depends on

05, 06.

## Scope

1. Targets (ADR-0005, **Proposed until this module verifies each row against the vendor's
   documentation and records the sources**):

   | Target | Sidecar name | JPEG and DNG frames | Rating | Label | Reject |
   |---|---|---|---|---|---|
   | `lightroom` | `<stem>.xmp` beside the RAW | not written (`embedded_only`) | `xmp:Rating` 0–5 | `xmp:Label` Red, Yellow, Green, Blue, Purple | `xmp:Rating="-1"` (verify that Lightroom Classic reads it as rejected on import or Read Metadata) |
   | `captureone` | `<stem>.xmp` beside the RAW | not written (verify) | `xmp:Rating` | `xmp:Label` names (verify the colour mapping) | `xmp:Rating="-1"` (verify) |
   | `darktable` | `<name>.<ext>.xmp` beside every file of the frame | written | `xmp:Rating` | `darktable:colorlabels` Seq, red 0, yellow 1, green 2, blue 3, purple 4 (verify) | `xmp:Rating="-1"` (verify) |

2. Mapping options: `unrated_pick_stars` (default 1: a pick with 0 stars is written as 1 star);
   `reject_as` `rating` (−1, default) | `red` (label Red) | `none`. Rating is always written for an
   exported frame; a label only when the frame has one; Keepers never deletes a field.
3. New sidecars are byte-exact per the templates below (UTF-8, no BOM, LF, final newline,
   `x:xmptk="Keepers XMP 1"`).
4. Merge into an existing sidecar with `quick-xml` events: an owned property present as an attribute or
   as an element has only its value replaced; an absent one is added to the first `rdf:Description`
   (declaring its namespace if needed; a self-closing Description is opened for `darktable:colorlabels`);
   every other byte is copied unchanged. A sidecar that is not well-formed is left alone.
5. Backups: before any change to an existing sidecar, a copy goes to
   `data/backups/<shoot>/<export_id>/<rel_path>`; an export log `data/exports/<shoot>/<export_id>.jsonl`
   records every operation (`sidecar_create`, `sidecar_merge`, `copy`, `move`) with paths and SHA-256.
6. `export undo <export_id>`: operations reversed in reverse order when the current file still has the
   hash the log recorded; otherwise it is left and reported.
7. Selection: `picks` (default), `picks-and-rejects`, `all`, `stars:N` (effective decisions).
8. Copy or move keepers to a destination, optionally rejects to a second one: RAW, companion and any
   sidecar travel together; collision-safe names `IMG_0001 (2).CR3`; identical content already there is
   skipped; move is a rename on the same volume, else copy, `fsync`, SHA-256 check, then delete; a free
   space check first. Sidecars go to `source`, `destination` or `both` (default: source without copy,
   destination with copy or move).
9. Plan then run: `export_plan` records each source file's size and mtime and returns the summary;
   `export_run` refuses a stale plan.
10. Guards: never write a path equal to any original; the destination must be outside the shoot folder.
11. CLI `keepers export <dir> --target <t> [--select …] [--unrated-pick-stars N] [--reject-as …]
    [--copy-to D | --move-to D --confirm-move] [--rejects-to D] [--sidecars …] [--dry-run] --json`,
    `keepers export undo <export_id> --json`, `keepers export list <dir> --json`.
12. The export dialog (PRODUCT §5) with the dry-run summary and a move confirmation, and Ctrl+E.

Templates (Adobe targets, a pick with 0 stars and a green label):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Keepers XMP 1">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
        xmlns:xmp="http://ns.adobe.com/xap/1.0/"
        xmp:Rating="1"
        xmp:Label="Green"/>
  </rdf:RDF>
</x:xmpmeta>
```

darktable, the same frame (`IMG_0001.CR3.xmp`; namespace `http://darktable.sf.net/` as darktable
itself writes it; verify that darktable imports a sidecar without its own history elements):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Keepers XMP 1">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
        xmlns:xmp="http://ns.adobe.com/xap/1.0/"
        xmlns:darktable="http://darktable.sf.net/"
        xmp:Rating="1">
      <darktable:colorlabels>
        <rdf:Seq>
          <rdf:li>2</rdf:li>
        </rdf:Seq>
      </darktable:colorlabels>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
```

A reject with no label: the Adobe template with `xmp:Rating="-1"/>` and no `xmp:Label` line; the
darktable template with `xmp:Rating="-1"/>` and no `darktable:colorlabels` element.

Messages: `embedded_only` "{name}: Lightroom Classic and Capture One read ratings inside JPEG and DNG
files, not from sidecars; not written."; `sidecar_name_conflict` "{name}: another file uses the sidecar
name {sidecar}; not written."; `sidecar_unreadable` "{name}: the existing sidecar is not valid XMP; left
unchanged."; `destination_inside_source` "Choose a destination outside the shoot folder.";
`not_enough_space` "Not enough free space at the destination: {need} needed, {free} free.";
`plan_stale` "Files changed since the export was planned. Plan again."; `move_verify_failed` "{name}:
the copy did not match the original, so the original was kept."; `undo_conflict` "{path} changed after
the export; left as it is."; dry-run summary "Will write {new} new sidecars and update {merged}
(backing up {merged}), copy {copies} files ({size}) to {dest}. No original file is changed."; move
confirmation "Move {n} files? They will be removed from the shoot folder after each copy is verified."

## Files

- `crates/keepers-core/src/export/{mod.rs,plan.rs,targets.rs,xmp_write.rs,xmp_merge.rs,copy.rs,log.rs,undo.rs}`
- `crates/keepers-cli/src/commands/export.rs`, `app/src/views/ExportDialog.tsx`, `app/src/state/export.test.ts`
- `crates/keepers-core/tests/07_export.rs`, `tests/cases/07-export/`, `crates/keepers-fixtures/src/export_set.rs`
- `docs/adr/0005-sidecars.md` (status Accepted once verified, with sources)

## Interfaces

```rust
pub fn plan_export(cache: &ShootCache, decisions: &[FrameDecision], opts: &ExportOptions,
    fs: &dyn FsInfo) -> Result<ExportPlan, ExportError>;
pub fn run_export(plan: &ExportPlan, env: &Env, progress: &dyn Progress, cancel: &CancelToken) -> Result<ExportReport, ExportError>;
pub fn undo_export(log: &Path, env: &Env) -> Result<ExportReport, ExportError>;
pub fn merge_sidecar(existing: &[u8], props: &OwnedProps, target: Target) -> Result<Vec<u8>, XmpError>;
pub fn new_sidecar(props: &OwnedProps, target: Target) -> Vec<u8>;
```

`keepers.export.plan/1`: `{"schema","plan_id","target","selection","frames","sidecars_new",
"sidecars_merged","backups","embedded_only","conflicts":[{"path","code","message"}],"copies","moves",
"bytes","destination","summary"}`. `keepers.export.report/1`: `{"schema","export_id","written",
"merged","backed_up","copied","moved","skipped":[{"path","code"}],"errors":[{"path","code","message"}]}`.
Export log line: `{"op":"sidecar_merge","path":"…","backup":"…","sha256_before":"…","sha256_after":"…"}`
(`sidecar_create` has `sha256`; `copy` and `move` have `from`, `to`, `sha256`, and `move` a `method`
of `rename` or `copy_delete`).

## Out of scope

- Writing metadata into JPEG, DNG or any original (a later ADR); keywords, captions and GPS; pick
  flags for Lightroom Classic (not in its XMP; a plug-in is a later idea); importing ratings from
  existing sidecars.

## Design notes

- Record here, per target, the vendor documentation page and date that confirms or changes each
  "verify" in the table, then set ADR-0005 to Accepted.
- **LightCraft** (MIT OR Apache-2.0, listed on Publik on 26 October 2026) reads and writes XMP
  sidecars and has pick and reject flags of its own. Its licence allows reading its source: before
  implementing, check which sidecar name and which rating, label and flag properties it reads, and
  record the answer here. If the `lightroom` target already serves it, say so in the export dialog's
  help text ("Lightroom Classic and LightCraft"); if not, add a `lightcraft` row to the table above
  with golden files of its own. Either way the release checklist gains one manual step: open an
  exported shoot in LightCraft and see the ratings and rejects. If Lightroom Classic ignores `-1`, the default
  `reject_as` for `lightroom` becomes `red` and the golden files change in this module only.
- Further decisions are filled in here during implementation.

## Acceptance tests

Golden cases in `tests/cases/07-export/` (with `copy_fixture = true` and `[files]` byte comparisons);
code tests in `crates/keepers-core/tests/07_export.rs`. References: the templates above (by
construction), ISO 16684-1, and the fixture's planted existing sidecars.

1. `lightroom-new`: pick, 0 stars, green → `IMG_0001.xmp` equals the Adobe template byte for byte.
2. `lightroom-reject`: reject, no label → the reject variant; `--reject-as red` gives `xmp:Rating="0"`
   and `xmp:Label="Red"`.
3. `darktable-new`: the same pick → `IMG_0001.CR3.xmp` equals the darktable template; its companion JPEG
   gets `IMG_0001.JPG.xmp` too.
4. `captureone-new` equals the Adobe template (same bytes as case 1).
5. Merge: an existing Lightroom-style sidecar carrying `crs:` develop settings, `dc:subject` keywords, a
   comment and `xmp:Rating="3"` as an attribute → output differs from input only in the value `3` → `4`
   (expected file made by hand from the input).
6. Merge variants: `<xmp:Rating>2</xmp:Rating>` as an element is replaced in place; an absent rating is
   added with its `xmlns:xmp` declaration; a self-closing darktable Description gains the colorlabels
   element; an existing `xmp:Label="Blue"` stays when the frame has no label.
7. A sidecar containing `<x:xmpmeta` without its closing tag is left byte-identical with `sidecar_unreadable`.
8. Round trip: every written sidecar parses with `quick-xml` and yields the planned Rating and Label.
9. JPEG and DNG frames with `--target lightroom` are counted `embedded_only` with the message; with
   `darktable` they are written.
10. Backups and undo: exporting over 2 existing sidecars backs both up; `export undo` restores them byte
    for byte and removes the new ones; editing one file after export makes undo report `undo_conflict`
    for it and restore the rest.
11. Copy: `--copy-to D` with `D/IMG_0001.CR3` already present with other content writes
    `IMG_0001 (2).CR3`, its companion `IMG_0001 (2).JPG` and sidecar `IMG_0001 (2).xmp`; with the same
    content present it is skipped as `already_there`.
12. Move: same-volume move uses rename (source gone, hash at destination equal); a copy made to fail
    verification (injected writer corrupting one byte) keeps the original and reports `move_verify_failed`;
    `--move-to` without `--confirm-move` exits 1 with "Moving needs --confirm-move."
13. Guards: `--copy-to <shoot>/sub` gives `destination_inside_source`; an injected free space of 1 KB
    gives `not_enough_space`; a planned sidecar path equal to an original's path is refused.
14. Plan then run: touching a source file between plan and run gives `plan_stale` and writes nothing.
15. Dry run writes nothing: the tree hash of the fixture copy and the data directory are unchanged, and
    the summary sentence has the planned counts.
16. `originals_untouched.rs`: after every case above, originals are unchanged (moved ones at their destination).
17. Export dialog test (Vitest, `api` mocked): choosing darktable and "picks and rejects" builds the
    options object; the move confirmation text is exact; **Export** is disabled until a plan exists.

## Status

Planned
