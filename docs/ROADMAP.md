# Keepers: roadmap

| NN | Module | Goal | Tier | Depends on | Status |
|---|---|---|---|---|---|
| 00 | [scaffold](modules/00-scaffold.md) | repository, documents, `verify`, hygiene test, golden harness, fixture generator, CI, hello-world window | load-bearing (hygiene, harness) | none | Done (2026-10-27) |
| 01 | [ingest-and-previews](modules/01-ingest-and-previews.md) | open a folder, get every frame with EXIF, identity and a thumbnail from the embedded preview | load-bearing | 00 | Planned |
| 02 | [technical-scores](modules/02-technical-scores.md) | sharpness, motion, exposure and noise per frame, versioned and cached | load-bearing | 01 | Planned |
| 03 | [faces-and-eyes](modules/03-faces-and-eyes.md) | model registry and downloads; faces, face sharpness, eyes open or closed | load-bearing | 02 | Planned |
| 04 | [groups-and-duplicates](modules/04-groups-and-duplicates.md) | bursts, near-duplicates and scenes | load-bearing | 02, 03 (registry) | Planned |
| 05 | [ranking-and-auto-cull](modules/05-ranking-and-auto-cull.md) | composite score, automatic cull with reasons, the decision journal | load-bearing | 03, 04 | Planned |
| 06 | [culling-ui](modules/06-culling-ui.md) | grid, loupe, compare, keyboard, filters, cull panel, models screen | interface | 05 | Planned |
| 07 | [export](modules/07-export.md) | XMP sidecars for three editors, merges, backups, undo, copy and move | load-bearing | 05, 06 | Planned |
| 08 | [performance-and-gates](modules/08-performance-and-gates.md) | bounded parallel pipeline, accelerators, gates with agreement against a human cull | load-bearing | 07 | Planned |
| 09 | [publik-second-opinion](modules/09-publik-second-opinion.md) | optional vision second opinion through publik API, own key or a local model | load-bearing (money path) | 04, 05, 06 | Planned |
| 10 | [learn-my-style](modules/10-learn-my-style.md) | the cull adapts to the photographer's past decisions, locally | load-bearing | 05, 06 | Planned |
| 11 | [packaging-and-release](modules/11-packaging-and-release.md) | installers, tag-driven releases, update check, listing prepared | interface (release gates load-bearing) | all | Planned |

The order follows the data: nothing can be scored before it is ingested, grouped before it is scored,
or culled before it is grouped, so 01 to 05 are a chain, each leaving a CLI command and golden cases
the next one builds on. The UI (06) comes only when the core can produce a whole cull, so it never
invents logic of its own, and export (07) follows the UI because its dialog lives there. Performance
(08) is measured on the finished pipeline, before any optional feature adds work. The Publik module
(09) is the only one that makes an AI call and comes after the core product is complete, because the
app must be fully useful without it; it ends with the maintainer's app-token step. Style learning (10)
needs journals from real use, and packaging (11) is last.
