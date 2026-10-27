# ADR-0001: Tauri 2 desktop shell, Rust core shared with a CLI, React window

- **Status:** Accepted
- **Date:** 2026-10-27

## Context

A wedding is 3,000 to 6,000 frames. The review window has to hold thousands of thumbnails,
scroll without stutter and stay under 2 GB of memory while analysis runs in the background
(`docs/PRODUCT.md` §7). The same analysis must run from a terminal (`keepers analyse`) and from
the window with identical results, because the golden tests drive the CLI and the window only
renders what the core returns (`docs/ARCHITECTURE.md` §7, invariant 5). Windows is the first
platform; macOS and Linux follow from continuous integration.

## Decision

A Rust core library (`crates/keepers-core`) decides everything. Two thin front ends call it: a
Tauri 2 desktop app whose host (`app/src-tauri`) only marshals commands into the core, with a
React 19, TypeScript and Vite window; and a `keepers` command (`crates/keepers-cli`) that prints
JSON documents. Image decoding, scoring, grouping, ranking, the journal and export all live in the
core and are tested there.

## Consequences

- One implementation for the app, the CLI and the tests; a golden case for the CLI proves the
  window's numbers too.
- Tauri uses the system webview, so the installer is small and memory goes to thumbnails, not to
  a bundled browser. The cost is a per-platform webview (WebView2 on Windows, WebKit on macOS and
  Linux) whose rendering differs in details; the window's logic is tested in Vitest and the
  rendering checked by one launch smoke test.
- The window cannot compute a score, so any logic that creeps into TypeScript is a bug; the
  boundary is one file (`app/src/api.ts`).
- The Rust toolchain, the Visual Studio C++ Build Tools on Windows and the GTK and WebKit
  development packages on Linux are build prerequisites.

## Alternatives considered

- **Electron**: a bundled Chromium costs hundreds of megabytes of memory before the first
  thumbnail, and two runtimes (Node and a native addon or a sidecar) to share the core with a CLI.
  Rejected on memory with thousands of thumbnails.
- **A Python sidecar for analysis**: fast to prototype with OpenCV and ONNX Runtime, but the
  install grows past 300 MB, start-up takes seconds, and results would depend on the Python
  environment. Rejected on install size and start-up.
- **A native toolkit (egui, iced, Qt)**: no webview differences, but a virtualised grid, loupe
  and compare views with synced zoom are mature on the web and would be months of work natively.
  Rejected on effort.
