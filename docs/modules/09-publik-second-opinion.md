# 09: Publik second opinion

## Goal

On a burst of frames that already passed the local checks, a photographer can ask a vision model which
frame has the best expression or moment and see its ranking, with a one-line reason per frame, beside
Keepers' own ranking. It is optional and off by default; it works through publik API, the user's own
key or a local vision model, offered side by side; it never changes a decision by itself. Everything
this module does is specified in `docs/PUBLIK-INTEGRATION.md`; this spec lists what will exist and how
it is tested.

## Depends on

04, 05, 06.

## Scope

Section signs (§) refer to `docs/PUBLIK-INTEGRATION.md` unless they say "contract".

1. **The Publik contract in `docs/PUBLIK-CONTRACT.md`, in full.**
2. Crate `crates/keepers-publik` with `src/publik.rs` copied verbatim from `publik.rs.md` and the seven
   adaptations of PUBLIK-INTEGRATION §2; the token compiled in from
   `crates/keepers-publik/publik-app-token.txt` by the reference's `include_str!`.
3. Provider choice (`publik` | `own` | `local`) mapped onto the reference's `resolve_provider` as §2
   says; the own key in `data/secrets.json` (mode 0600), never returned to the UI.
4. `settings.second_opinion.enabled` false by default; no AI client is constructed until it is true and
   a provider is chosen.
5. The enable dialog, the first-run card, the $0.00 pre-check and needs-credit panel, with the exact
   strings of PUBLIK-INTEGRATION §4, including the personal-data warning and the permission box.
6. Frame selection and preparation (§7): 2 to 6 candidates, resize to ≤ 1568 px, metadata stripped,
   body ≤ 3,900,000 bytes with the quality and size steps.
7. Prompt `prompts/second-opinion.v1.md` and schema `prompts/second-opinion.v1.schema.json` (§6.1),
   loaded with `include_str!`; the renderer; validation steps 1 to 5 with the documented failure.
8. The chat completions route for all three providers (§6.1) and the Gemini `publik-vision` route
   (§6.2) behind the build constant `VISION_ROUTE`, default `ChatCompletions`.
9. Response caches per provider (`data/ai-cache/*.sqlite`), the cache key and the idempotency header.
10. The status object from every `x-publik-*` header (contract §7), `publik-status.json`, `GET /wallet`
    refreshes at the three moments of §6.4 (at app start only when second opinions are on and the
    provider is `publik`); the balance line in the three places of §9.
11. The results panel in Compare: Keepers' ranking and the AI's ranking side by side, reasons,
    confidence, provider and model, "Last call $0.027 · $3.12 of publik balance"; **Pick this frame**
    appends exactly one journal line, `{"type":"flag","frames":[<the chosen frame_id>],"value":"pick",
    "via":"second_opinion"}`; the other frames of the group keep their decisions (the automatic cull
    or the user's own). Nothing else ever writes the journal from this module.
12. **Ask about close calls**: groups with at least two candidates whose top two composites differ by
    less than 0.05; the batch confirmation, sequential calls, measured cost line and stop rules of §8–§9.
13. Error handling for every row of §9, own-key retries (2, after 2 s and 8 s), no Publik retries.
14. Local provider: discovery by `/api/tags` and `/api/show`, the multi-image probe (§6.3) with
    `prompts/local-vision-probe.v1.md`, and the messages of §3.
15. Tauri commands `ai_status`, `ai_enable`, `ai_disable`, `ai_set_own_key`, `ai_local_models`,
    `ai_set_local_model`, `ai_publik_reconnect`, `ai_publik_disconnect`, `ai_clear_answers`,
    `second_opinion_preview`, `second_opinion_run` (with `confirmed` and `retry_after_invalid`),
    `second_opinion_get`, `open_link`, with the arguments, results and `AiStatus` fields of
    ARCHITECTURE §5; Settings > Second opinion (three providers, balance line,
    "Clear second-opinion answers", Disconnect).
16. CLI `keepers second-opinion <dir> (--group <g> | --frames <i,j,…> | --close-calls --confirm)
    [--provider publik|own|local] [--dry-run] --json`, printing `keepers.second_opinion.plan/1` or
    `keepers.second_opinion/1`; without a usable provider it exits 3 with "No provider is set up.
    Turn on second opinions in the app, or set PUBLIK_API_KEY." The CLI never provisions.
17. `scripts/publik-proof.sh --vision` (§6.4), maintainer only, never run in CI or during implementation.
18. `SECURITY.md` updated with every request of this module and the `device_name` note.
19. The built-app self-test (contract §3, PUBLIK-INTEGRATION §2): the first statement of `main` in
    `app/src-tauri/src/main.rs`, before the Tauri builder, logging or ONNX Runtime start, prints the
    reference's `self_test(...)` line and returns when `PUBLIK_SELF_TEST` is set; on Windows it first
    attaches to the parent console. It never runs in normal use; the CLI ignores the variable.

Feature-map rows (PUBLIK-INTEGRATION §5): F15 second opinion (chat completions, `publik-balanced`),
F15 fallback route (Gemini, `publik-vision`), F15 local probe (local only).

## Files

- `crates/keepers-publik/{Cargo.toml,publik-app-token.txt}` (the token written by the maintainer's step), `src/{lib.rs,publik.rs,provider.rs,client.rs,gemini.rs,prepare.rs,prompt.rs,validate.rs,second_opinion.rs,cache.rs,status.rs,local.rs}`
- `prompts/{second-opinion.v1.md,second-opinion.v1.schema.json,local-vision-probe.v1.md}`
- `app/src-tauri/src/commands_ai.rs`, `app/src-tauri/src/main.rs` (the self-test hook, the first lines of `main`)
- `app/src/views/{SecondOpinion.tsx,EnableSecondOpinion.tsx,SettingsSecondOpinion.tsx}`, `app/src/state/secondOpinion.ts`
- `crates/keepers-cli/src/commands/second_opinion.rs`, `scripts/publik-proof.sh`, `SECURITY.md`, `CONTRIBUTING.md` (the §12 block)
- `crates/keepers-publik/tests/09_publik_second_opinion.rs`, `app/src-tauri/tests/09_publik_self_test.rs`, `app/src/state/secondOpinion.test.ts`, `tests/cases/09-publik-second-opinion/`

## Interfaces

```rust
pub enum Provider { Publik(PublikCredential), Own { base_url: String, model: String }, Local { base_url: String, model: String } }
pub fn choose_provider(settings: &SecondOpinionSettings, secrets: &Secrets, env: &EnvVars) -> ProviderChoice; // Ready(Provider) | NeedsConsent | NeedsSetup
pub fn candidates(group: &GroupDetail, decisions: &[FrameDecision]) -> Result<Vec<FrameId>, SoError>; // TooFewFrames
pub fn prepare(frames: &[FrameId], cache: &ShootCache) -> Result<Prepared, SoError>;               // FramesTooLarge
pub fn render(prompt: &PromptFile, n: usize) -> RenderedPrompt;
pub fn ask(p: &Provider, prepared: &Prepared, route: VisionRoute, http: &dyn Transport,
    cache: &AiCache, status: &mut Status) -> Result<SecondOpinion, SoError>;
pub fn validate(text: &str, finish: &str, labels: &[char]) -> Result<Answer, InvalidAnswer>;   // rule number
pub fn estimate_micros(n: usize, measured: Option<(u64, usize)>) -> u64;                        // 4 × (330 + 1,635 n)
pub enum SoError { TooFewFrames, FramesTooLarge, NeedsConsent, NeedsCredit { message: String, url: String },
    Disconnected, RateLimited(String), Busy, Timeout, Offline, Http { code: u16, message: String }, InvalidAnswer(u8) }
```

The startup hook, the first lines of `main` in `app/src-tauri/src/main.rs` (the reference's own hook,
plus the Windows console attach of Design notes):

```rust
fn main() {
    if std::env::var_os("PUBLIK_SELF_TEST").is_some() {
        #[cfg(windows)] attach_parent_console(); // AttachConsole(ATTACH_PARENT_PROCESS); errors ignored
        println!("{}", keepers_publik::publik::self_test(env!("CARGO_PKG_VERSION")));
        return;
    }
    // … then logging, settings and the Tauri builder, as before
}
```

CLI documents. The golden case compares only the values the spec fixes (schema, group, provider,
model, route, prompt, version, labels, indices, `cached`, `estimate_micros`); `width`, `height`,
`quality`, `jpeg_bytes`, `body_bytes` and `cache_key`, which only the encoder can produce, are under
`[ignore]` and checked by rule in test 27 (the numbers below are illustrative):

```jsonl
{"schema":"keepers.second_opinion.plan/1","group":"g0001","provider":"publik","model":"publik-balanced",
 "route":"chat_completions","prompt":"second-opinion","prompt_version":1,
 "frames":[{"label":"A","index":0,"width":1536,"height":1024,"quality":85,"jpeg_bytes":402113}],
 "body_bytes":2210345,"cache_key":"<64 hex>","cached":false,"estimate_micros":34020}
{"schema":"keepers.second_opinion/1","group":"g0001","provider":"publik","model":"publik-balanced",
 "ranking":[{"label":"C","index":2,"frame_id":"<32 hex>","reason":"…"}],"confidence":"medium","note":"…",
 "local_best":0,"cached":false,"charge_micros":27480,"balance_line":"$3.12 of publik balance"}
```

## Out of scope

- Any automatic use of the answer (never); `publik-smart`; face crops as input (PRODUCT §8);
  products other than `ai.chat`; publishing the listing (11 prepares it, the maintainer publishes).

## Design notes

- At the start of this module: fetch `https://publikhq.com/developers`, `/pricing`, `/llms.txt`,
  `/skills/setup.md`, `/skills/api.md` and `/sdk/publik.rs.md` raw with curl and record here anything
  that changed since 2026-10-16.
- The live Publik path needs the app token, which the maintainer's setup step writes (PUBLIK-INTEGRATION
  §2: once this module is done and a release build works, Publik's setup skill lists the app, mints the
  token and runs the built app with `PUBLIK_SELF_TEST=1`); until then the module is completed and
  tested against the fake server.
- Self-test on Windows: a Tauri release build uses the GUI subsystem
  (`windows_subsystem = "windows"`), which has no console, so a plain `println!` from a terminal start
  is lost and the setup skill would see no line. The hook calls `AttachConsole(ATTACH_PARENT_PROCESS)`
  first (`windows-sys`, `Win32_System_Console`), ignoring a failure, which keeps a redirected stdout
  working too. This is the one addition to the reference's hook; test 30 reads stdout through a pipe,
  and the terminal case is checked by hand once on a Windows release build.
- Once the proof script has run, its output is recorded here and in ADR-0007 (route,
  whether `publik-balanced` and `publik-fast` accept images, whether `response_format` is honoured, the
  measured charge for four frames), `VISION_ROUTE` is set, the §8 estimate is replaced with the
  measurement, and ADR-0007 is set to Accepted.
- Further decisions are filled in here during implementation.

## Acceptance tests

Code tests in `crates/keepers-publik/tests/09_publik_second_opinion.rs`, UI logic in
`app/src/state/secondOpinion.test.ts`, golden cases in `tests/cases/09-publik-second-opinion/`. All HTTP
is a localhost fake; no test reaches publikhq.com. References: `docs/PUBLIK-CONTRACT.md`, the strings
and formulas of `docs/PUBLIK-INTEGRATION.md`, the JPEG marker definitions of T.81, and the fixtures.

Contract tests (`docs/PUBLIK-CONTRACT.md` §11), each one named test:

1. Resolution order: own key beats env beats credential file; nothing set gives `NeedsConsent`.
2. `provision()` sends the documented body with no hostname; 201 writes the credential with mode 0600
   before returning; 200 with `key: null` keeps the key on disk; 200 with no key on disk mints once more
   under a fresh id; `install_revoked` mints once more; a second failure surfaces the message.
3. The app token comes from the file, overridden by `PUBLIK_APP_TOKEN`; a build without a `pat_` token
   gives "This build has no publik API app token. Use your own key for now." (the real-token test skips
   with a message until the maintainer's step is done).
4. Header transport; tier model names only on chat (`publik-balanced`), the Gemini fallback's
   `publik-vision` the only other model value; the idempotency key is 32 lowercase hex.
5. Status parsed from every header in contract §7; `GET /wallet` refresh.
6. 402 (anonymous and claimed): the message, exactly one URL, one call, needs-credit set, nothing
   cached, the batch stopped, the app still usable.
7. A success after 402 clears needs-credit.
8. 401 and 403 `install_revoked` mark disconnected and offer **Reconnect**.
9. 429, 503 and other errors: one attempt, the documented message.
10. A Publik timeout is not retried; the own-key path retries with backoff.
11. Caches are per provider; identical input never spends twice; a prompt version bump misses.
12. The local provider never calls publikhq.com, even with a Publik credential on disk.
13. `publik-smart` falls back to `publik-balanced` while `claim_state` is `anonymous` (the reference helper).
14. Every response is validated against its schema; an invalid one is discarded with "The AI answer
    could not be used."
15. Every request is under 4 MB; the images are resized to at most 1568 px and carry no EXIF, XMP or IPTC.
16. The key never appears in logs, errors, the status object or anything the UI receives.
17. The copy rule, over UI strings, `docs/` and `README.md` (exempting `docs/PUBLIK-CONTRACT.md` and
    the test itself, which quote the phrases).

Keepers tests K1 to K26 of `docs/PUBLIK-INTEGRATION.md` §11, each one named test, with the fake
responses and expected results listed there, plus:

18. Candidate selection: a group of 9 with 2 rule-rejected members sends the top 6 of the other 7 by
    composite, labelled A–F in that order; a user-rejected frame is never sent.
19. Close calls: groups with top-two composite gaps 0.04 and 0.06 → only the first is a close call.
20. Enable dialog (Vitest): three cards of equal width in one row; **Continue with publik API** and
    **Use my own key** disabled until the permission box is ticked; **Use a local model** never needs it;
    **Not now** sends nothing.
21. First-run card at `balance_micros: 0`, anonymous: "$0.00 · link this computer for $0.05 of free use",
    `cost_sentence` verbatim, and the three buttons side by side, the primary **Sign in with publik**;
    once claimed the primary reads **Add a plan or pack**.
22. `open_link` accepts only `claim_url`, `add_credit_url` and `top_up_url` from the current status;
    any other URL gives `link_not_allowed`.
23. The own key round trip: `ai_set_own_key` stores it in `secrets.json` with mode 0600 (POSIX runners)
    and `ai_status` returns `own.configured: true` with no key field.
24. `originals_untouched.rs` passes after a dry run and a faked ask.
25. `scripts/publik-proof.sh --vision` without a credential or `PUBLIK_API_KEY` exits 1 with "publik-proof
    needs a Publik key (credential file or PUBLIK_API_KEY)."; with `CI=true` it exits 1 with
    "publik-proof is for the maintainer only; it never runs in CI." `SECURITY.md` names `/installs`,
    `/installs/revoke`, `/wallet`, `/chat/completions`, the Gemini route and the localhost endpoints.
26. Results panel (Vitest): a stored answer shows Keepers' order and the AI's order side by side with
    each reason, "Confidence: medium", "publik-balanced", and the footer line from the status; the
    answer from a local model is labelled "Local model".
27. Plan bounds (code test on the `gen/bursts` group `g0001`, five frames, and on the six
    `second-opinion/` noise frames): every `width` and `height` keeps the source aspect ratio with the
    long side ≤ 1568 and never above the source; `quality` is 85, 75 or 65; each `jpeg_bytes` ≤
    `B(n) = floor((3,900,000 − 16,384) × 3 / 4 / n)`; `body_bytes` ≤ 3,900,000 and equals the length of
    the serialised body; `cache_key` matches `^[0-9a-f]{64}$`; `estimate_micros` = 4 × (330 + 1,635 n)
    (34,020 for five frames).
28. Wallet at start: with second opinions off, or on with provider `own` or `local`, app start makes no
    request to publikhq.com; on with provider `publik`, exactly one `GET /wallet` and no other request.
29. Command errors spelled out: `own_key_missing` "Enter a base URL, a key and a model first.";
    `local_no_vision_model` "No local vision model found at {base_url}."; `local_probe_failed` "This
    local model does not accept several images at once. Choose another vision model."; `no_app_token`
    the contract's sentence; `confirmation_required` "Confirm the number of calls first."
30. Self-test (contract test 18), in `app/src-tauri/tests/09_publik_self_test.rs`, spawning the app
    binary with `KEEPERS_DATA_DIR` set to an empty temporary directory, `HOME`, `XDG_CONFIG_HOME` and
    `LOCALAPPDATA` set to another, and the fake server reached as in test 2: with `PUBLIK_SELF_TEST=1`
    and a fake `/installs` answering 201, stdout is exactly one line, `publik self-test: ok keepers
    <version> (install <first 8 characters of the install_id>)`, and the process exits with code 0
    within 10 seconds, having opened no window (on Linux it runs with no display, so a window would
    fail the run) and leaving `KEEPERS_DATA_DIR` empty; with `/installs` answering 500 it prints
    `publik self-test: failed: <message>` and exits the same way. Without the variable it never runs:
    `PUBLIK_SELF_TEST` is read only by that hook (a source search over `app/src-tauri/src/` and
    `crates/*/src/`), and K18 and K26 make no `/installs` request.

## Status

Planned
