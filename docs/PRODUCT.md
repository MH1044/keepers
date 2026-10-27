# Keepers: product

## 1. Summary

Keepers is a desktop app that culls a photo shoot on the photographer's own computer: it flags blur,
closed eyes and bad exposure, groups bursts and near-duplicates, marks the best frame of each, and
writes ratings that Lightroom Classic, Capture One and darktable read. It replaces the culling
subscriptions (Aftershoot Select, Narrative Select, FilterPixel) and the culling half of Photo Mechanic.

## 2. Why this exists

- **Aftershoot** Select (culling only) costs $10 a month on annual billing or $15 monthly; Complete is
  $45 or $55 ([Aftershoot pricing tiers, updated 21 May 2026](https://aftershoot.com/blog/aftershoot-pricing-tiers/)).
  Its marketing says the culling runs on the user's computer (re-check before publishing), so there
  is no server cost behind the subscription.
- **Narrative Select** runs from $10 a month (Lite, annual) to $60 (Ultra, annual), $15 to $79
  monthly; no permanent free plan ([narrative.so/pricing](https://narrative.so/pricing), read 29 September 2026).
- **Open source**: the GitHub topic `photo-culling` lists 55 repositories; the two most starred are
  Python projects (facet, 194 stars, which also does face recognition; pixcull, 97 stars) and the rest
  have fewer than 25 ([github.com/topics/photo-culling](https://github.com/topics/photo-culling), read
  29 September 2026). None is an installable, keyboard-first desktop app with RAW preview extraction,
  per-decision reasons and a guarantee that originals are never written.
- **Publik**: no automatic culler is listed. PixelRevive is photo repair (rated WEAK in the audit:
  classic filters only). **LightCraft** (listed 26 October 2026, MIT OR Apache-2.0, built on publik
  API) is a full Lightroom-style library and raw developer with manual culling (Compare and Survey
  views, ratings, pick and reject flags, auto-stacks, duplicate detection on import) and AI keywords
  and captions, but nothing scores sharpness, eyes or exposure, or picks the best frame of a burst.
  Keepers is the automatic first pass that LightCraft lacks; it never grows a library or a raw
  developer, and its XMP sidecars are meant to be read by LightCraft as well as Lightroom Classic
  (module 07).

The gap: a fast desktop culler a working photographer would trust with a wedding, free, private by
construction, that explains every decision.

## 3. Who it is for

- **Wedding photographer**, 3,000 to 6,000 frames per wedding: get from card to a first cull in minutes
  and spend the evening on the 800 that matter.
- **Sports and event photographer**, long bursts at 10 to 30 frames a second: keep the sharpest frame
  of each burst and reject the rest without looking at every one.
- **Enthusiast** with Lightroom or darktable: stop paying a subscription to reject blurry holiday photos.

## 4. v1 features

**F1 Open a shoot** (01, 06). As a photographer I point Keepers at a folder or card and see every frame.
- Given a folder with JPEG, CR2, CR3, NEF, ARW, RAF, ORF, RW2 or DNG files, when I open it, then every
  supported file is listed; RAW+JPEG pairs with the same name show as one frame.
- Unreadable or unsupported files are listed with a reason and never stop the scan.
- Renaming the folder, or adding a second card's files to it, keeps the same shoot and my decisions.

**F2 Fast previews** (01, 06). Thumbnails come from the preview embedded in each RAW, not a RAW decode.
- The first screen of thumbnails appears within 3 s for a 3,000-frame folder on an SSD (target, 08).

**F3 Technical scores** (02). Every frame gets sharpness (overall and best region), motion blur,
exposure and noise values.
- Given the same frame blurred more, its sharpness is strictly lower.
- Frames with more than 5% clipped highlights are flagged overexposed.

**F4 Faces and eyes** (03). Faces are found and each is judged eyes open, closed or uncertain.
- The loupe shows each face's box and eye state; the grid can filter "eyes closed".
- With no face models installed, everything else still works and the app says faces were not checked.
- Keepers never identifies who a person is.

**F5 Model download** (03, 06). Models download on my click, with their size shown first.
- A damaged or tampered file is refused by checksum with a clear message.

**F6 Bursts and near-duplicates** (04). Similar frames are grouped so only the best needs a look.
- Frames of one burst end up in one group; distinct photos are never merged.
- Groups do not depend on the order files were found.

**F7 Automatic first cull** (05, 06). I get picks and rejects I can steer with plain controls: "keep
about 30%", "reject closed eyes", "focus: lenient, normal, strict", "one per burst".
- Changing a control updates the cull in under a second for 6,000 frames (target, 08).

**F8 Reasons** (05, 06). Every automatic decision shows why: "eyes closed: 2 of 3 faces", "motion
blur", "a sharper frame of this burst was kept", "best of a burst of 5".

**F9 Review views** (06). Grid, loupe (fit, 100%, jump to faces) and compare 2 to 4 up with synced zoom,
all keyboard first: P pick, X reject, U unflag, 0 to 5 stars, 6 to 9 colour labels.

**F10 Decisions kept safe** (05, 06). My decisions override the automatic cull, survive re-analysis and
cache deletion, and can be undone.
- Given I pick a frame the cull rejected, when I change the cull controls, then it stays picked.

**F11 Export to my editor** (07). Ratings, colour labels and rejects are written as XMP sidecars for
Lightroom Classic, Capture One or darktable.
- A dry run shows how many sidecars will be created, merged and backed up before anything is written.
- An existing sidecar keeps every field Keepers does not own; the prior version is backed up.
- The last export can be undone.

**F12 Copy or move keepers** (07). Picks (and optionally rejects) are copied or moved to folders.
- A copy never overwrites an existing file; names get " (2)" instead.

**F13 CLI** (00 to 07). `keepers analyse <dir> --json` runs the whole cull in a terminal and prints the
same decisions and reasons the app shows.

**F14 Speed and hardware** (08). Analysis runs in parallel with bounded memory; DirectML (Windows) or
CoreML (macOS) can be switched on.

**F15 Second opinion** (09, optional, off by default). For a burst of sharp, open-eyed frames I can ask
a vision model which has the best expression or moment, and see its ranking with a one-line reason per
frame beside Keepers' own ranking.
- It never changes a decision by itself; "Pick this frame" is my click.
- It works through publik API, my own key, or a local vision model; the three are offered side by side.
- The enable dialog warns that photos of people are personal data.
- Asking about several groups shows the number of calls and the cost first.

**F16 Learn my style** (10). After a few shoots, the cull adapts to how I cull, on this computer.
- With no history the cull is unchanged; Reset returns to the default cull.

**F17 Install and update** (11). A per-user Windows installer (no administrator password), a macOS DMG,
Linux builds; an optional update check I can switch off.

## 5. Screens and flows

**Home.** Purpose: open a shoot. Elements: "Open folder…", drop zone, recent shoots (name, frames,
last opened, a "folder not found" badge). Empty: "Open a folder of photos to start. Keepers never
changes your original files." Loading: none (instant). Error: the folder message from module 01.
Offline: unaffected.

**Models setup** (first run, and Settings > Models). Purpose: download face and similarity models.
Elements: one row per model group with size, licence and status; "Download" and "Skip for now".
Loading: progress bar with bytes. Error: checksum or connection message with "Try again". Offline:
"Models need a connection to download. Keepers works without them: faces will not be checked."

**Grid.** Purpose: see the whole shoot. Elements: virtualised thumbnails with flag, stars, label,
reason chip and group badge ("×5"); groups collapsed to their best frame (G expands); filter bar
(flag, stars, reason, group, faces, eyes closed); cull panel; status bar ("2,412 frames · 734 picks ·
1,120 rejects · analysing faces 1,203 of 2,412"). Empty filter: "No frames match these filters." with
"Clear filters". Loading: grey cells fill in as thumbnails arrive; analysis progress with Cancel. Error:
"{n} files could not be read" opens the list.

**Loupe.** Purpose: judge one frame. Elements: the largest embedded preview, fit or 100% (Z), face
boxes with eye state, reasons, EXIF line (shutter, aperture, ISO, focal length), decision controls.
Loading: thumbnail first, then the preview. Error: "The preview of this frame could not be read."

**Compare.** Purpose: choose between 2 to 4 frames. Elements: synced zoom and pan, per-frame
composite and reasons, "Second opinion…" button. Empty: opens with the current group's top four, or
the selection; more than four selected: "Compare shows up to 4 frames." Loading and error: as the
loupe. Offline: unaffected (the second opinion states its own).

**Cull panel.** Purpose: steer the automatic cull. Elements: "Keep about" slider (5% to 80%), "Per
scene / whole shoot", "Focus: lenient, normal, strict", "Reject closed eyes", "One per burst",
"Grouping: loose, normal, strict", "Use my style" (module 10). Shows how many decisions changed.
Loading: "Updating the cull…" while groups are recomputed. Error: the validation message from
module 05. Before analysis finishes the controls are disabled with "Available when analysis finishes."

**Export dialog.** Purpose: send the cull to an editor. Elements: target (Lightroom Classic, Capture
One, darktable), which frames (picks; picks and unflagged; rated at least N stars), mapping options,
copy or move to a folder, the dry-run summary, "Export". Loading: progress with Cancel. Error:
per-file list; nothing half-written. Offline: unaffected.

**Settings.** General (threads, advance after flag, update check), Models, Accelerator (08), Second
opinion (09: the three providers, balance line), Style (10).

**Second opinion** (09). Enable dialog, $0.00 card, results panel and batch confirmation: exact strings
in `docs/PUBLIK-INTEGRATION.md` §4 and §9. Offline: "A second opinion needs a connection or a local
model."

**First-run flow.**
1. Launch: Home with the empty-state sentence. No network request.
2. "Open folder…" and choose a folder: frames appear as the scan runs.
3. If face models are missing, a banner: "Face and eye checks are off: the face models are not
   installed." with "Download (size)". Declining keeps the banner; everything else works.
4. Analysis runs; the grid shows the automatic cull with reasons as soon as it finishes.
5. Review with the keyboard; decisions are saved as they are made.
6. Export: choose the editor, read the dry run, confirm.
7. The second opinion stays off until the user clicks "Second opinion…" and completes the enable dialog.

## 6. Non-goals

Editing and presets; cloud sync, galleries, client proofing or accounts; writing into original files
(JPEG or DNG metadata included); video; face recognition or naming people; a mobile app; a Lightroom
plug-in.

## 7. Success measures

| Measure | Target | Checked at |
|---|---|---|
| First thumbnails for a 3,000-frame SSD folder | ≤ 3 s | 08 gates |
| Full analysis throughput, 24 MP RAW, 4-core laptop CPU, no GPU | ≥ 300 frames a minute | 08 gates |
| Peak memory analysing 6,000 frames | ≤ 2 GB | 08 gates |
| Reject recall and precision against the maintainer's own cull | ≥ 0.80 each | 08 gates (thresholds recorded when first measured) |
| Best-frame agreement per group with the maintainer | ≥ 0.60 | 08 gates |
| Eye state on the labelled CC0 set | ≥ 90% of judged eyes correct | 03 tests |
| Time from install to the first cull of a folder, including model download | ≤ 5 minutes | 11 manual checklist |
| Cost per active user per month | $0 without the second opinion; about $1.65 for 60 groups asked | 09 measured |

## 8. Later

Writing into JPEG and DNG metadata (an ADR); an aesthetic score if a permissively licensed model
appears; a small eye-state classifier; face crops as the second-opinion input; a cheaper "quick
opinion" on `publik-fast` if it agrees with `publik-balanced`; HEIF and CR3 burst-roll support; a
Lightroom Classic plug-in that sets pick flags; tethered live culling.

## 9. Publik listing draft

| Field | Value |
|---|---|
| Name | Keepers (7 characters; check for conflicts at module 11) |
| Summary | Local AI photo culling: flags blur, closed eyes and duplicates. (63 characters) |
| Category | Photo |
| alternativeTo | Aftershoot, Narrative Select, FilterPixel |
| Platforms | Windows, macOS, Linux |
| Licence | MIT OR Apache-2.0 |
| Publik API products | `ai.chat` (optional second opinion) |

Publik lists an app that needs a service it covers only if the app runs on publik API. The second
opinion does: publik API is the first of its three providers and the only one that needs nothing set
up; the own key and the local model stay beside it (`docs/PUBLIK-INTEGRATION.md` §2).
