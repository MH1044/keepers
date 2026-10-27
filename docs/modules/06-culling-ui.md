# 06: Culling UI

## Goal

A photographer reviews 3,000 frames without touching the mouse: open a folder, watch the grid fill,
steer the automatic cull with a few controls, check faces in the loupe, compare near-identical frames
side by side, and flag, rate and label with single keys. The UI shows only what the core returns.

## Depends on

05.

## Scope

1. The Tauri commands of ARCHITECTURE §5 except the AI, style and update ones, in
   `app/src-tauri/src/commands.rs`, each a thin call into `keepers-core`, plus `keepers://progress`
   and `keepers://job-done` events. `app/src/api.ts` is the only file that calls `invoke`.
2. The `keepers` URI scheme (`protocol.rs`): `thumb/<shoot>/<frame>` (image/jpeg from the cache),
   `preview/<shoot>/<frame>` (the embedded preview bytes, header `x-keepers-orientation: 1..8`),
   `face/<shoot>/<frame>/<n>` (512 px crop); 404 for unknown ids; `Cache-Control: max-age=3600`.
3. Screens of PRODUCT §5: Home (open folder, drop zone, recent shoots), Models setup, Grid, Loupe,
   Compare, Cull panel, Settings (General, Models). Empty, loading and error states with the PRODUCT strings.
4. Grid: virtualised with `@tanstack/react-virtual`; pages of up to 500 rows through `frames_page`;
   groups collapsed to their best frame with a "×n" badge; flag, stars, label and first reason chip.
5. Loupe: fit and 100% (Z toggles; Shift+Z steps through faces at 100%); face boxes with eye state;
   all reasons; EXIF line "1/250 s · f/2.8 · ISO 800 · 85 mm".
6. Compare: 2 to 4 frames (the selection, or the current group's top four), zoom and pan synced.
7. The keyboard map below as a pure reducer `app/src/state/keyboard.ts`
   `(state, key) → {state, commands[]}`; "Advance after flag" setting (default on).
8. Filters (`app/src/state/filters.ts`) built into the `frames_page` filter object: flag, minimum
   stars, label, reason code, group only, has faces, eyes closed; combined with AND.
9. Cull panel bound to `cull_get`/`cull_set`: Keep about (5–80%, step 5), Per scene / Whole shoot,
   Focus (3-way), Reject closed eyes, One per burst, Grouping (3-way); shows "{n} decisions changed".
10. Models setup screen and banner (sizes from `models_status`, download with progress and cancel).
11. Progress in the status bar with **Cancel**; errors from commands shown as their `message`.
12. One dark theme defined once as CSS custom properties in `theme.css`; no colour literal elsewhere.
13. The e2e smoke test and the `e2e-smoke` CI job; the manual checklist rows in `docs/TESTING.md` §10.

| Key | Action | Key | Action |
|---|---|---|---|
| ← → ↑ ↓ | move (grid); previous / next (loupe, compare) | Space | grid ↔ loupe |
| P / X / U | pick / reject / unflag | 0–5 | stars |
| 6 / 7 / 8 / 9 | red / yellow / green / blue label (same key again clears) | Z, Shift+Z | zoom 100%; next face |
| C | compare the selection or the group | G | expand or collapse the group |
| Shift+arrows | extend the selection (grid) | Ctrl+Z / Ctrl+Shift+Z (Cmd on macOS) | undo / redo |
| Esc | back to grid; clear selection | ? | shortcut overlay |
| Ctrl+E | export dialog (07) | Ctrl+O | open folder |

## Files

- `app/src-tauri/src/{commands.rs,protocol.rs,state.rs,main.rs}`, `app/src-tauri/capabilities/default.json`
- `app/src/api.ts`, `app/src/types.ts` (mirrors the Rust result types)
- `app/src/state/{store.ts,keyboard.ts,filters.ts,selection.ts}` and their `*.test.ts`
- `app/src/views/{Home.tsx,ModelsSetup.tsx,Grid.tsx,Loupe.tsx,Compare.tsx,CullPanel.tsx,Settings.tsx,StatusBar.tsx}`
- `app/src/theme.css`, `app/e2e/smoke.test.mjs`, `.github/workflows/ci.yml` (the `e2e-smoke` job)
- `app/src-tauri/tests/commands.rs`

## Interfaces

```ts
// app/src/api.ts (every function returns a Promise and rejects with {code, message})
openShoot(path: string): Promise<ShootSummary>
framesPage(shootId: string, offset: number, limit: number, filter: Filter, collapseGroups: boolean): Promise<{ total: number; rows: FrameRow[] }>
setDecision(shootId: string, frames: string[], change: DecisionChange): Promise<FrameDecision[]>
// app/src/state/keyboard.ts
export function reduce(state: ViewState, key: KeyInput): { state: ViewState; commands: Command[] }
```

`KEEPERS_E2E_OPEN=<folder>` (debug builds only) opens that folder at start so the smoke test needs no
native dialog.

## Out of scope

- Export dialog (07), accelerator setting (08), second-opinion UI (09), style panel (10), update
  check (11). No score, threshold or reason is computed in TypeScript.

## Design notes

- Decisions are sent to the core first; the UI updates from the returned `FrameDecision[]`, so a
  failed journal write never shows a decision that was not saved ("Your decision could not be saved:
  {message}").
- Further decisions are filled in here during implementation.

## Acceptance tests

UI logic in `app/src/state/*.test.ts` (Vitest); host tests in `app/src-tauri/tests/commands.rs`; the
smoke in `app/e2e/smoke.test.mjs`. References: the keyboard table above, PRODUCT §5 strings, and the
`wedding-mini/` fixture's frame count and truth.

1. Keyboard reducer, table-driven: each key in the map from a grid state and from a loupe state gives
   the listed state change and command (for example P in the loupe with advance on gives
   `[setDecision(flag: pick)]` and moves to the next frame; with advance off it stays).
2. 7 on a frame labelled yellow gives `label: none`; 7 on a green frame gives `label: yellow`.
3. Shift+→ three times from index 4 selects 4–7; C with that selection opens compare with 4 frames;
   C with 5 selected shows "Compare shows up to 4 frames."
4. Filters: flag pick + eyes closed + stars ≥ 2 produce the filter object `{flag:"pick",
   eyes_closed:true, min_stars:2}`; clearing gives `{}`; the empty result shows "No frames match these
   filters." with **Clear filters**.
5. `frames_page` host test on `wedding-mini`: `total` is 40; `collapse_groups: true` gives 40 minus
   the collapsed burst members from `truth.toml`; `limit: 501` gives `invalid_argument` "limit must
   be at most 500".
6. Protocol handler: a known thumbnail returns 200 with `image/jpeg` bytes equal to the cache file; an
   unknown frame returns 404; the preview response carries `x-keepers-orientation`.
7. `decision_set` then `decision_undo` through the host returns the prior decision; a journal made
   read-only on disk gives the error, and the UI test shows "Your decision could not be saved: …".
8. Cull panel: moving Keep about from 30 to 10 calls `cull_set` with `keep_percent: 10` and shows the
   returned changed count.
9. Serialisation test: every command result type round-trips through JSON with the field names in
   `types.ts` (a generated fixture of each type is parsed by the TypeScript test).
10. Theme: a lint test fails on a colour literal (`#`, `rgb(`, `hsl(`) in any `.tsx` or `.css` file
    other than `theme.css`.
11. Smoke (CI, Ubuntu): the app launches with `KEEPERS_E2E_OPEN=tests/fixtures/gen/wedding-mini`, the
    status bar reads "40 frames" within 60 s, pressing X on the first frame makes its flag chip read
    "reject", and the journal holds one more `flag` line.
12. No-models state: with an empty data directory the grid shows the banner "Face and eye checks are
    off: the face models are not installed." and still lists 40 frames.
13. EXIF line formatter: shutter 0.004, aperture 2.8, ISO 800, focal 85.0 give "1/250 s · f/2.8 ·
    ISO 800 · 85 mm"; shutter 2.0 gives "2 s"; missing values are left out with their separator.
14. Models setup and progress (component tests, `api` mocked): rows show the sizes from
    `models_status` as "4.1 MB"; **Download** calls `models_download` with the group's ids; a progress
    event updates the bar; **Cancel** calls `job_cancel` with the job id.

## Status

Planned
