# ADR-0004: YuNet, MediaPipe face landmarks with the eye aspect ratio, CLIP ViT-B/32; no aesthetic model in v1

- **Status:** Proposed until modules 03 and 04 (the face-landmark conversion's licence and the
  eye-state gate are settled at 03; the CLIP export at 04)
- **Date:** 2026-10-27

## Context

Keepers needs three learned pieces: finding faces, judging whether eyes are open, and measuring
visual similarity between frames for near-duplicate grouping (`docs/PRODUCT.md` §4, F4 and F6).
Everything runs on the CPU of a laptop at hundreds of frames a minute, so the models must be
small. ADR-0002 forbids non-commercial or research-only weights, which rules out several popular
face stacks. The eye-state gate is numeric: at least 90% of judged eyes correct on the labelled
CC0 set (`docs/TESTING.md` §6).

## Decision

- **Faces**: YuNet from the OpenCV Zoo (the model folder's README states MIT), decoded as
  OpenCV's `FaceDetectorYN` does (Apache-2.0, ported with attribution), run through ONNX Runtime
  on the 1024 px working image.
- **Eyes**: the MediaPipe face-landmark model (Apache-2.0 upstream) as ONNX, and the eye aspect
  ratio of Soukupová and Čech (2016) on the landmarks, with thresholds for open, closed and
  uncertain; faces too small to judge are reported as such.
- **Similarity**: the CLIP ViT-B/32 image encoder (MIT upstream) as ONNX, L2-normalised
  embeddings, beside dHash and capture-time gaps.
- **No aesthetic model in v1**: no permissively licensed one of adequate quality exists, and a
  score nobody can explain contradicts the "reasons on every decision" promise.

Each model is a row in `THIRD_PARTY.md` and an entry in the registry (`models.toml`) with URL,
SHA-256 and size; the app downloads on the user's click and never ships weights. Without the
face models everything else still works and the app says faces were not checked.

## Consequences

- Three small models, about 100 MB in total, downloaded once.
- The landmark conversion's licence must be recorded at module 03 before it is used; if no
  permissively licensed conversion exists, the eye check needs another route (below).
- The eye aspect ratio is a geometric rule, so it is explainable and cheap, but it is weaker
  than a trained eye classifier on squinting faces and extreme angles. If it fails the 90% gate,
  the maintainer decides between accepting a weaker check (more "uncertain" answers) and funding
  a small classifier trained on a licensed set; that decision supersedes this record.
- CLIP embeddings are stored per frame (512 floats); they describe content, not identity, and
  the schema has no face vector.

## Alternatives considered

- **InsightFace (RetinaFace, ArcFace)**: excellent accuracy. Rejected because its weights are
  non-commercial.
- **A trained eye-state classifier**: more robust than the aspect ratio. Deferred because it needs
  a licensed training set and a labelling effort; kept as the fallback the maintainer can fund.
- **An aesthetic model (NIMA and successors)**: would rank "nice" frames. Rejected for v1 on
  licence availability and explainability; listed under "Later" in `docs/PRODUCT.md`.
- **dHash only for similarity**: no model download. Rejected because a perceptual hash merges
  frames that merely share a layout and misses the same moment under a different exposure;
  dHash stays as the cheap first pass.
