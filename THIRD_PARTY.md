# Third-party software, models and datasets

Every direct dependency, model and dataset with its licence and source. Transitive dependencies
are checked by the licence gate (`deny.toml` and the npm check in `app/package.json`) on every
push; `Cargo.lock` and `app/package-lock.json` list them with their versions.

## Rust crates

| Crate | Used for | Licence | Source |
|---|---|---|---|
| clap 4 | command-line parsing for `keepers` and `keepers-fixtures` | MIT OR Apache-2.0 | https://github.com/clap-rs/clap |
| serde 1 | the documents the CLI and the app exchange | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| serde_json 1 | JSON output and golden comparison | MIT OR Apache-2.0 | https://github.com/serde-rs/json |
| image 0.25 (`jpeg` feature only) | JPEG encoding of generated fixtures; decoding from module 01 | MIT OR Apache-2.0 | https://github.com/image-rs/image |
| libm 0.2 | the Gaussian taps, the sRGB curve and the exposure factor of the fixture generator, identical on every platform | MIT | https://github.com/rust-lang/compiler-builtins |
| sha2 0.10 | the fixture manifest; identities and cache keys from module 01 | MIT OR Apache-2.0 | https://github.com/RustCrypto/hashes |
| tauri 2 | the desktop host | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri |
| tauri-build 2 | build script of the desktop host | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri |

Test-only crates:

| Crate | Used for | Licence | Source |
|---|---|---|---|
| regex 1 | the source-hygiene test and the golden harness's `[match]` rules | MIT OR Apache-2.0 | https://github.com/rust-lang/regex |
| toml 1 | reading `case.toml` and `deny.toml` in tests | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| tempfile 3 | a fresh data directory per golden case | MIT OR Apache-2.0 | https://github.com/Stebalien/tempfile |
| ignore 0.4 | the hygiene test's walk when git is unavailable | Unlicense OR MIT | https://github.com/BurntSushi/ripgrep |

Tools (not linked into any binary): cargo-deny (MIT OR Apache-2.0,
https://github.com/EmbarkStudios/cargo-deny), rustfmt and clippy (part of the Rust toolchain).

## npm packages

Shipped in the window:

| Package | Used for | Licence | Source |
|---|---|---|---|
| react 19 | the window | MIT | https://github.com/facebook/react |
| react-dom 19 | the window | MIT | https://github.com/facebook/react |
| @tauri-apps/api 2 | calling the host's commands | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri |

Development only (never distributed):

| Package | Used for | Licence | Source |
|---|---|---|---|
| vite 8 | building and serving the window | MIT | https://github.com/vitejs/vite |
| @vitejs/plugin-react 6 | React support in Vite | MIT | https://github.com/vitejs/vite-plugin-react |
| typescript 5 | type checking | Apache-2.0 | https://github.com/microsoft/TypeScript |
| @types/react 19, @types/react-dom 19 | type definitions | MIT | https://github.com/DefinitelyTyped/DefinitelyTyped |
| eslint 10, @eslint/js 10 | linting | MIT | https://github.com/eslint/eslint |
| typescript-eslint 8 | TypeScript rules for ESLint | MIT | https://github.com/typescript-eslint/typescript-eslint |
| eslint-plugin-react-hooks 7 | the rules of hooks | MIT | https://github.com/facebook/react |
| prettier 3 | formatting | MIT | https://github.com/prettier/prettier |
| vitest 5 | unit tests of the window's logic | MIT | https://github.com/vitest-dev/vitest |
| jsdom 30 | a DOM for Vitest | MIT | https://github.com/jsdom/jsdom |
| @testing-library/react 16, @testing-library/dom 10 | rendering components in tests | MIT | https://github.com/testing-library |
| license-checker-rseidelsohn 5 | the npm licence check | BSD-3-Clause | https://github.com/RSeidelsohn/license-checker-rseidelsohn |
| @tauri-apps/cli 2 | `npm run tauri dev` | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri |

## Models

None yet. The face detector, the face-landmark model and the image encoder arrive with modules
03 and 04 (`docs/adr/0004-model-set.md`); each will have a row here with its licence, source and
checksum, and is downloaded by the app on the user's click, never committed.

## Datasets

None yet. The CC0 photos for the eye-state test arrive with module 03, each with its source URL
and licence; real RAW samples are downloaded by checksum in continuous integration and never
committed.

## Generated fixtures

`tests/fixtures/gen/` is written by `crates/keepers-fixtures` from a seed; nothing in it comes from
a real photograph. The placeholder window icons in `app/src-tauri/icons/` are drawn by the project
and carry no third-party content.
