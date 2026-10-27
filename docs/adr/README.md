# Decision records

Append-only records of the decisions that are expensive to make and expensive to reverse. Each
says what forced it, what was decided, what it costs, and what was rejected and why. An accepted
record is never edited; a change is a new record that supersedes it. `template.md` is the shape.

| ADR | Decision | Status |
|---|---|---|
| [0001](0001-stack.md) | Tauri 2 desktop shell, Rust core shared with a CLI, React window | Accepted |
| [0002](0002-licence.md) | MIT OR Apache-2.0, with a licence gate that refuses copyleft dependencies | Accepted |
| [0003](0003-storage.md) | Derived SQLite cache plus an append-only decision journal; originals never written; sidecars only on Export | Accepted |
| [0004](0004-model-set.md) | YuNet, MediaPipe face landmarks with the eye aspect ratio, CLIP ViT-B/32; no aesthetic model in v1 | Proposed until modules 03 and 04 |
| [0005](0005-sidecars-per-editor.md) | Sidecar naming, rating, label and reject mapping and merge rules per editor | Proposed until module 07 |
| [0006](0006-identity.md) | Content fingerprints for files, frames and shoots | Accepted |
| [0007](0007-second-opinion-route.md) | Second opinions as chat-completions image parts on `publik-balanced`; the Gemini `publik-vision` route only if the probe says so | Proposed until module 09 |

Rules: a record is written before the code that depends on it; a Proposed record names the
module whose spike or measurement settles it; what was rejected is recorded with the deciding
factor, because that is the part people come back to.
