# Keepers: Publik API integration

This file says how Keepers uses the Publik API. It builds on `docs/PUBLIK-CONTRACT.md` (the shared
contract, copied unchanged) and never contradicts it; where this file is silent, the contract rules.
Everything here is implemented in module 09 (`docs/modules/09-publik-second-opinion.md`).

## 1. Summary

- **One AI feature**: the second opinion (F15). For a burst of frames that already passed the local
  focus, exposure and eye checks, a vision model ranks 2 to 6 frames by expression and moment, with a
  one-line reason per frame. The answer is advice shown beside Keepers' own ranking; it never changes a
  decision.
- **Off by default.** No AI request of any kind is made until the user turns second opinions on and
  chooses a provider.
- **Products the app token needs: `ai.chat` only.** Images travel inside chat requests. No
  `ai.embeddings`, `ai.transcribe`, `ai.speech`, `ai.image` or `search`.
- **Without any provider** Keepers is complete: scanning, scores, faces and eyes, groups, the automatic
  cull with reasons, review, export, copy and move, and style learning all run locally. Only F15 needs a
  provider, and it can be a local vision model.

## 2. Setup

**Slug**: `keepers` (confirm it is free when the listing is made; if not, the maintainer picks one and
`APP_SLUG` changes with it).

**Listing rule**: Publik lists an app that needs a service it covers only if the app runs on publik
API. Keepers' one covered need is chat with images (`ai.chat`) for the second opinion, and it runs on
publik API: the first provider the enable dialog offers, and the only one that needs nothing set up.
The own key and the local model stay beside it as the alternatives.

**The maintainer's step** (the live Publik path needs the app token; until this step is done,
module 09 is completed and tested against the fake server). Once module 09 is done and a release build
works, the maintainer runs Publik's setup skill once, in an interactive run in the repository, with
`https://publikhq.com/submit` open: fetch `https://publikhq.com/skills/setup.md` with curl (raw) and
follow it for the publik slug `keepers`, products `ai.chat`.

1. It reads the repository, derives the listing (`skills/publish.md` steps 1 and 2, with the listing
   draft in `docs/PRODUCT.md` §9) and shows one checklist: the listing fields, the product Keepers
   needs (`ai.chat`), and the built-app test. Nothing changes before the maintainer says yes.
2. One browser approval covers the rest. It lists the app (installing from source is enough; new
   releases update the listing within a day).
3. It mints the app token for `keepers` (`skills/api.md` step 2: `POST /apps/keepers/token`) and writes
   `crates/keepers-publik/publik-app-token.txt` (next to the `Cargo.toml` of the crate that holds
   `publik.rs`; public by design, committed) without printing it. A `403 claim_first` means the
   listing must first be claimed on Publik ("Claim it with GitHub" on the listing page).
4. It builds the release artifact and runs it once with `PUBLIK_SELF_TEST=1` (the startup hook of
   module 09). The line `publik self-test: ok keepers <version>` is the proof Publik counts.

The module and the UI already exist, so the skill only adds what is missing. Nothing is listed or
published without the maintainer's yes.

After the setup skill, the maintainer runs `scripts/publik-proof.sh --vision` once (§6.4). It spends a
few cents of the maintainer's balance and prints the replies, the charges and the header dump; they are
then recorded in the module 09 Design notes and in ADR-0007.

**The module.** At the start of module 09, fetch `https://publikhq.com/sdk/publik.rs.md` with curl (raw)
and copy the Rust module **verbatim** to `crates/keepers-publik/src/publik.rs` (the contract's
`crates/<app>-publik` location, so the CLI and the Tauri host share it). Change only `APP_SLUG`
(`"keepers"`), `APP_VERSION` (`env!("CARGO_PKG_VERSION")`) and these adaptations, each recorded in
the module's Design notes:

1. **The key never reaches the webview.** Tauri commands return `AiStatus` only; no serialisable type
   in `keepers-publik` has a key field. A test serialises every command result and event and searches
   for the key.
2. **`device_name`**: the Rust module sends none; Keepers keeps it that way. `SECURITY.md` names the
   field and says it is not sent.
3. **Transport**: desktop only, so the reference's system `curl` stays. The key goes in a curl config
   read from stdin (`--config -`), never on the command line. A vision body (up to 3.9 MB) goes in a
   temporary file under `<cache>/keepers/tmp/` created with mode 0600 and deleted after the call,
   success or failure. If the reference already passes bodies another safe way, keep its way. With no
   `curl` on the system: "publik API needs the curl program, which was not found. Use your own key or
   a local model."
4. **One AI client** (`client.rs`), OpenAI-compatible chat completions, for publik, the own key and
   the local model. The Gemini route (`gemini.rs`) exists for publik only.
5. **Token file**: `crates/keepers-publik/publik-app-token.txt`, next to that crate's `Cargo.toml`,
   because the reference compiles it in with `include_str!("../publik-app-token.txt")` from `src/`
   (contract §2). No `build.rs` and no copy under `app/src-tauri/`; `PUBLIK_APP_TOKEN` at build time
   overrides it, as the reference does.
6. **Status file**: `<data>/keepers/publik-status.json`.
7. **Tier**: F15 always sends `publik-balanced`. The reference's `publik-smart` fallback helper is kept
   and tested (contract test 13) but Keepers does not request `publik-smart` in v1.

**Provider choice.** `settings.second_opinion.provider` is `"publik"`, `"own"` or `"local"`. Keepers
calls the reference's `resolve_provider(user_key)` with the own key when the provider is `own` and with
`None` when it is `publik` (so `PUBLIK_API_KEY` beats the credential file, and nothing set gives
`NeedsConsent`); `local` never calls `resolve_provider` and never reads the Publik credential.

**Self-test** (contract §3). The first statement of `main` in `app/src-tauri/src/main.rs`, before the
Tauri builder, logging or ONNX Runtime start: if `PUBLIK_SELF_TEST` is set, print the reference's
`self_test(env!("CARGO_PKG_VERSION"))` line (`publik self-test: ok keepers <version> (install <8
chars>)` or `publik self-test: failed: <message>`) and return. It provisions for real (the builder who
runs it is the consent) and never runs in normal use. Windows release builds have no console (the GUI
subsystem), so on Windows the hook first attaches to the console of the terminal that started it;
otherwise the line would be lost. The `keepers` CLI never provisions and ignores the variable.

## 3. Providers

| | publik API | Your own key | Local model |
|---|---|---|---|
| What it is | the per-install `pk_` key from `POST /installs` | any OpenAI-compatible base URL, key and vision model | Ollama at `http://localhost:11434/v1`, or any OpenAI-compatible server on localhost |
| Second opinion | yes, `publik-balanced` | yes, the user's model | yes, if a vision model accepts several images per request |
| Cost | Publik balance (§8) | billed by the user's provider | free |
| Retries | never | up to 2, after 2 s and 8 s, on timeout, 429 or 5xx | none |
| Timeout | 60 s | 60 s | 180 s |
| Leaves the machine | the prepared frames and the prompt | the same, to the user's provider | nothing |

**Local discovery**: `GET http://localhost:11434/api/tags`, then `POST /api/show` `{"model": name}`
for each model; a model is a vision model when its `capabilities` array contains `"vision"`; Keepers
picks the largest by `size`. If no model reports `capabilities` (an older server), Settings asks for
the model name: "This local server does not say which models can see images. Type the name of a vision
model." Then the multi-image probe (§6.3) runs once per model; a model that fails it cannot be used for
second opinions: "This local model does not accept several images at once. Choose another vision
model." Field names and behaviour are verified against the server's documentation at module 09.

**What degrades locally**: slower (tens of seconds per group on a CPU), and small vision models judge
expressions less reliably. The UI says "Local model" beside every local answer.

## 4. First run and consent

**When.** The first click on **Second opinion…** (Compare view, or the group menu), or turning on
Settings > Second opinion. Never at launch, never when opening a shoot.

**The enable dialog** (exact strings):

- Title: **Turn on second opinions**
- Intro: "A second opinion asks a vision model to rank 2 to 6 similar frames by expression and moment.
  It is advice shown beside Keepers' own ranking; it never changes your cull."
- Warning, emphasised: "Photos of people are personal data. Send a client's photos to an outside
  service only if you have their permission. You are responsible for that."
- Three cards of equal size, side by side:
  - **publik API**: the data-path sentence, then `DISCLOSURE` verbatim, then **Continue with publik API**.
    Data-path sentence: "Only reduced copies of the frames you ask about (at most 1568 pixels on the
    long side, all metadata removed) go through publik's servers. File names, locations, camera details
    and everything else stay on this computer."
  - **Your own key**: "Photos go to the vision service you choose, under its terms. You pay it
    directly." Fields: base URL, key (password field), model. Button **Use my own key**.
  - **Local model**: "Photos stay on this computer. Needs Ollama, or another OpenAI-compatible server
    on this computer, with a vision model. Slower, and free." Shows the detected model or "No local
    vision model found at http://localhost:11434." Button **Use a local model**.
- Checkbox under the publik and own-key cards: "I have permission to send these photos for a second
  opinion." Unticked by default; both buttons stay disabled until it is ticked. Calling `ai_enable`
  without it returns `permission_required`: "Tick the permission box first."
- Footer: **Not now** (closes; nothing is saved or sent).

**After Continue with publik API**: `provision()` (§6.4). On success the dialog becomes the first-run
card:

- Heading: "Second opinions are on"
- `balance_line(...)`, for example "$0.00 · link this computer for $0.05 of free use"
- `cost_sentence` verbatim
- "A group of four frames costs about $0.03 (estimate). The real cost is shown after each call."
- Three buttons side by side: **Sign in with publik** (primary, opens `claim_url`),
  **Use my own key**, **Use a local model**. Once `claim_state` is `claimed` the primary reads
  **Add a plan or pack** and opens `add_credit_url`. **Done** closes the card.

Provisioning failures keep the dialog open with the other two cards usable: 429 shows `error.message`;
no app token: "This build has no publik API app token. Use your own key for now."; other codes:
"publik API could not set up this computer ({code}). Use your own key for now."; unreachable:
"publik API is unreachable right now. Use your own key for now."

**The $0.00 start.** A fresh install provisions with `balance_micros` 0, so the first ask would return
402. Before sending, if the status says `balance_micros` is 0, Keepers refreshes `GET /wallet` (free).
If it is still 0, no chat request is made and the needs-credit panel shows: "Your publik balance is
$0.00." with exactly one link (**Sign in with publik** to `claim_url` while anonymous,
**Add a plan or pack** to `add_credit_url` once claimed) and "or use your own key or a local model in
Settings" with the in-app buttons **Use my own key** and **Use a local model** (no URL). Module 09
verifies that `/wallet`'s `balance_micros` includes the starter amount; if it does not, the pre-check
is removed and the 402 path alone applies.

## 5. Feature map

| Feature | Trigger | Endpoint | Tier | What is sent | What comes back | Cache key | Estimated cost |
|---|---|---|---|---|---|---|---|
| F15 second opinion | **Ask** in Compare or on a group; **Ask about close calls** (batch) | `POST /chat/completions` | `publik-balanced` | prompt `second-opinion.v1` + 2–6 JPEGs ≤ 1568 px, no metadata | JSON ranking, reasons, confidence, note | SHA-256 of provider, model, route, prompt id+version, frame hashes | $0.014 (2 frames), $0.027 (4), $0.041 (6) |
| F15 fallback route | the same, when ADR-0007 selects it | `POST {base_url}/gemini/v1beta/models/publik-vision:generateContent` | `publik-vision` | the same, as Gemini parts | the same JSON as text | same, with route `gemini` | assumed as above; measured |
| F15 local probe | choosing a local model | local `POST /v1/chat/completions` only | the local model | prompt `local-vision-probe.v1` + two 64 × 64 JPEGs | `{"colours": [...]}` | not cached | free |

## 6. Calls in detail

### 6.1 Second opinion, chat completions route (publik, own key, local)

Headers: `Authorization: Bearer <key>`, `Content-Type: application/json`,
`x-publik-idempotency-key` (publik only; see "Idempotency key" below). Not streamed. `max_tokens`
700. Timeout 60 s (local 180 s). `response_format` is sent only if the §6.4 probe shows the tier
honours it; the default is not to send it.

Complete request body for four frames (base64 shortened):

```json
{
  "model": "publik-balanced",
  "stream": false,
  "max_tokens": 700,
  "messages": [
    {"role": "system", "content": "You help a professional photographer choose between similar frames of one moment, taken seconds apart. Every frame you see has already passed automatic checks for focus, exposure and open eyes. Judge only what those checks cannot: facial expressions (natural smiles, relaxed faces, no half-closed eyes, no mouths caught mid-word), the moment itself (peak action, a gesture, eye contact, people interacting) and body language. Mention technical quality only if one frame is clearly worse than the others.\nNever identify anyone. Never guess names, age, ethnicity, health or any other personal attribute. Refer to people only by their place in the frame, for example \"the person on the left\".\nAnswer with one JSON object and nothing else: no prose before or after it, no code fence."},
    {"role": "user", "content": [
      {"type": "text", "text": "These are 4 frames from one burst, labelled A, B, C and D. Each image follows its label.\nRank all 4 frames from best to worst for a photographer who will keep one.\nFor each frame give one reason of at most 20 words about expression or moment.\nReply with exactly this shape:\n{\"ranking\": [{\"label\": \"A\", \"reason\": \"...\"}], \"confidence\": \"high\", \"note\": \"\"}\nRules: \"ranking\" lists every label in A, B, C and D exactly once, best first. \"confidence\" is \"high\", \"medium\" or \"low\"; use \"low\" when the frames are nearly the same. \"note\" is at most 30 words, or an empty string."},
      {"type": "text", "text": "Frame A:"},
      {"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0a…"}},
      {"type": "text", "text": "Frame B:"},
      {"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0a…"}},
      {"type": "text", "text": "Frame C:"},
      {"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0a…"}},
      {"type": "text", "text": "Frame D:"},
      {"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0a…"}}
    ]}
  ]
}
```

For the own key and the local model only `model` changes (the user's or the detected model name).

**Prompt file** `prompts/second-opinion.v1.md` (loaded with `include_str!`, never edited in place; a
change is `v2`). Placeholders: `{frame_count}` (2–6), `{label_list}` ("A and B", "A, B and C", …,
"A, B, C, D, E and F"), `{label}`.

````md
---
id: second-opinion
version: 1
tier: publik-balanced
max_tokens: 700
timeout_s: 60
schema: second-opinion.v1.schema.json
---
## system
You help a professional photographer choose between similar frames of one moment, taken seconds apart. Every frame you see has already passed automatic checks for focus, exposure and open eyes. Judge only what those checks cannot: facial expressions (natural smiles, relaxed faces, no half-closed eyes, no mouths caught mid-word), the moment itself (peak action, a gesture, eye contact, people interacting) and body language. Mention technical quality only if one frame is clearly worse than the others.
Never identify anyone. Never guess names, age, ethnicity, health or any other personal attribute. Refer to people only by their place in the frame, for example "the person on the left".
Answer with one JSON object and nothing else: no prose before or after it, no code fence.
## user
These are {frame_count} frames from one burst, labelled {label_list}. Each image follows its label.
Rank all {frame_count} frames from best to worst for a photographer who will keep one.
For each frame give one reason of at most 20 words about expression or moment.
Reply with exactly this shape:
{"ranking": [{"label": "A", "reason": "..."}], "confidence": "high", "note": ""}
Rules: "ranking" lists every label in {label_list} exactly once, best first. "confidence" is "high", "medium" or "low"; use "low" when the frames are nearly the same. "note" is at most 30 words, or an empty string.
## frame
Frame {label}:
````

Sections are split on lines starting `## `; the text of each section is taken without its trailing
newline. Only the three named placeholders are substituted; the literal braces of the JSON example
are left alone (the renderer replaces `{name}` only for the three known names).

**Response schema** `prompts/second-opinion.v1.schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "keepers/second-opinion.v1.schema.json",
  "type": "object",
  "additionalProperties": false,
  "required": ["ranking", "confidence", "note"],
  "properties": {
    "ranking": {
      "type": "array", "minItems": 2, "maxItems": 6,
      "items": {
        "type": "object", "additionalProperties": false, "required": ["label", "reason"],
        "properties": {
          "label": {"type": "string", "enum": ["A", "B", "C", "D", "E", "F"]},
          "reason": {"type": "string", "minLength": 1, "maxLength": 160}
        }
      }
    },
    "confidence": {"type": "string", "enum": ["high", "medium", "low"]},
    "note": {"type": "string", "maxLength": 240}
  }
}
```

Example answer (`choices[0].message.content`):
`{"ranking":[{"label":"C","reason":"Both people laugh naturally and look at each other; the moment peaks here."},{"label":"A","reason":"Good smiles, but the person on the right is caught mid-word."},{"label":"D","reason":"The person on the left looks down; the moment has passed."},{"label":"B","reason":"The person on the right half-blinks."}],"confidence":"medium","note":"A and C are close; C has the stronger interaction."}`

**Validation**, in order; the first failure discards the answer:

1. `finish_reason` is `stop` (a `length` finish is a failure: the answer was cut off).
2. Trim whitespace. If the text starts with a line of three backticks (optionally followed by `json`)
   and ends with three backticks, remove those two fence lines; nothing else is stripped.
3. It parses as one JSON object and validates against the schema.
4. The ranking's labels are exactly the labels sent, each once (the schema cannot say this).
5. Each reason and the note are trimmed and have control characters removed; a reason that is then
   empty is a failure.

On failure: nothing is cached or shown; the panel says "The AI answer could not be used." with
**Try again**, and for publik "Trying again sends the frames again and may cost another call." The log
records `invalid_ai_answer` and the rule number, never the text. On success the stored result maps
labels back to frame ids: `{"ranking":[{"frame_id","label","reason"}],"confidence","note","provider",
"model","charge_micros"}`.

**Cache key**: SHA-256 (64 hex) of the canonical JSON (sorted keys, no whitespace) of
`{"frames":["<sha256 of JPEG A>",…],"model":"publik-balanced","prompt":"second-opinion",
"prompt_version":1,"provider":"publik","route":"chat_completions"}`. For the own key, `provider` is
`"own:<base_url>"`; for local, `"local"`. Caches are separate files per provider
(`ai-cache/publik.sqlite`, `own.sqlite`, `local.sqlite`), table `responses(key TEXT PRIMARY KEY,
created_at TEXT NOT NULL, prompt TEXT NOT NULL, prompt_version INTEGER NOT NULL, answer TEXT NOT NULL,
charge_micros INTEGER)`. The same frames in a different order are a different question and miss.

**Idempotency key** (contract §6): for the first attempt, the first 32 hex of the cache key. A **Try
again** after an answer failed validation is attempt n ≥ 2: the key is the first 32 hex of the SHA-256
of the same canonical JSON with `"attempt": n` added, so it is a new request, not a replay of the
unusable answer. The cache key itself never includes the attempt, so a valid answer on attempt n is
cached under the normal key. Failed attempts are counted per cache key in the provider's cache file
(`failed_attempts(key TEXT PRIMARY KEY, attempts INTEGER NOT NULL)`), so the count survives a restart.
A **Try again** after a 503 or a timeout keeps the previous key unchanged.

### 6.2 Fallback: the Gemini dialect (publik only)

Used only if ADR-0007, after the §6.4 probe, finds that `publik-balanced` refuses image parts on
`/chat/completions`. The route is a build constant (`VisionRoute::ChatCompletions` or
`VisionRoute::GeminiPublikVision`), never switched at run time, because a second attempt could charge
twice. `POST {base_url}/gemini/v1beta/models/publik-vision:generateContent`, headers
`x-goog-api-key: <key>`, `Content-Type: application/json`, `x-publik-idempotency-key`. The dialect
refuses `systemInstruction`, `tools`, `toolConfig`, `cachedContent` and `safetySettings`, so the
system text becomes the first text part:

```json
{
  "contents": [{"role": "user", "parts": [
    {"text": "<system section>\n\n<user section, rendered as in 6.1>"},
    {"text": "Frame A:"},
    {"inlineData": {"mimeType": "image/jpeg", "data": "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgH…"}},
    {"text": "Frame B:"},
    {"inlineData": {"mimeType": "image/jpeg", "data": "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgH…"}}
  ]}],
  "generationConfig": {"maxOutputTokens": 700}
}
```

The answer text is the concatenation of `candidates[0].content.parts[*].text`; `finishReason` must be
`STOP`; then validation steps 2 to 5 apply. The probe confirms that `generationConfig` is accepted; if
it is not, it is dropped and `max_tokens` is recorded as not enforceable on this route.

### 6.3 Local multi-image probe (local only, never sent to publik)

`prompts/local-vision-probe.v1.md`, one user message, no system message:

```md
---
id: local-vision-probe
version: 1
max_tokens: 50
timeout_s: 180
---
## user
You see two small images. Reply with JSON only, in this shape: {"colours": ["<colour of the first image>", "<colour of the second image>"]}. Use plain colour names.
```

Sent to the local model with two 64 × 64 JPEGs, pure red then pure blue, as `image_url` parts. It
passes when the answer parses, `colours` has two strings, the first contains "red" and the second
"blue" (case-insensitive). The result is stored in `settings.second_opinion.local.multi_image`.

### 6.4 Account calls and the proof script

`POST /installs`, `GET /wallet`, `POST {base_url}/installs/revoke` are made exactly as the reference
module makes them (contract §5): `provision()` on **Continue with publik API** or **Reconnect**;
`GET /wallet` at app start **only when second opinions are on and the chosen provider is publik**
(contract §7: only once the user has chosen publik API; with the feature off, or with the own key or
the local model, start-up makes no request to publikhq.com), when Settings > Second opinion opens
with publik chosen, and in the $0.00 pre-check; revoke on **Disconnect** (best effort; keep `install_id`).

`scripts/publik-proof.sh --vision` (maintainer only; never in CI) uses the credential file or
`PUBLIK_API_KEY` and makes, in order: `GET /models` (to record whether `publik-vision` is listed); one
chat call with a single 64 × 64 grey JPEG and the text "Reply with the word ok." to `publik-balanced`;
the same to `publik-fast`; the same through the Gemini route; one real second opinion on the generated
four-frame fixture group; and that second opinion again with `"response_format": {"type":
"json_object"}`. It prints each status, reply, `x-publik-charge-micros` and the full `x-publik-*`
header dump. Results go into ADR-0007 and replace the estimates in §8.

## 7. Images

- **Which frames**: the group's members that no local rule rejected (out of focus, motion blur, face
  out of focus, eyes closed, over- or underexposed) and the user did not reject, ordered by composite
  (then index), first six; or the 2 to 4 frames of the Compare view; or a selection of 2 to 6. Fewer
  than two: "A second opinion needs at least two sharp frames with open eyes."
- **Source**: the frame's largest embedded preview (the file itself for a JPEG frame), orientation applied.
- **Resize**: long side to 1568 px with the `image` crate's Lanczos3 filter if it is larger; never upscaled.
- **Encode**: baseline JPEG, quality 85, 4:2:0. The output holds only SOI, APP0 (JFIF), DQT, SOF0,
  DHT, SOS and EOI: no APP1 (EXIF, XMP), APP2 (ICC), APP13 (IPTC) or COM segment. Pixels are treated as
  sRGB.
- **Size**: the serialised body stays at or under 3,900,000 bytes (the contract's 4 MB, base64
  included, with margin). Per-frame budget `B = floor((3,900,000 − 16,384) × 3 / 4 / n)` bytes of JPEG
  (485,452 for six frames). A frame over budget is re-encoded at quality 75, then 65, then at 1280 px
  and quality 75, then 1024 px and quality 75; still over: "These frames are too large to send, even
  reduced." (`frames_too_large`).
- **On disk**: prepared JPEGs live in memory; only the transient curl body file touches disk (§2).

## 8. Cost model

Published price: a typical use (about 1,500 words in and 375 out, roughly 2,000 + 500 = 2,500 tokens at
0.75 words a token) costs $0.01 on `publik-balanced`; smaller calls cost proportionally less. Keepers
therefore estimates **4 micros per token** ($0.01 / 2,500 tokens).

- Text: the prompt is 124 system words + 94 user words + 2 words per frame label, about 300 tokens.
- Images: published formulas of the major vision models put a 1568 × 1045 image at roughly 1,100 to
  1,600 input tokens (some scale large images down first). **Estimate: 1,600 tokens a frame.**
  This is the uncertain part: **measured with `x-publik-charge-micros` in module 09** (§6.4).
- Output: about 30 + 35 tokens a frame (a 20-word reason plus JSON).
- Formula: `estimate_micros(n) = 4 × (330 + 1,635 × n)`.

| Frames | Input tokens | Output tokens | Typical uses | Estimate |
|---|---|---|---|---|
| 2 | 296 + 3,200 = 3,496 | 100 | 3,596 / 2,500 = 1.44 | $0.014 |
| 4 | 301 + 6,400 = 6,701 | 170 | 6,871 / 2,500 = 2.75 | $0.027 |
| 6 | 307 + 9,600 = 9,907 | 240 | 10,147 / 2,500 = 4.06 | $0.041 |

- **Per session** (one wedding, 15 close-call groups of about four frames): 15 × $0.027 = $0.41.
- **Per active user per month** who uses the feature (4 shoots × 15 groups): 60 × $0.027 = **$1.65**.
  Users who never turn it on: $0. The $0.05 from linking covers about 1.8 groups of four; Basic ($8,
  about 800 typical uses) covers about 290.
- After the first call, estimates use the measured charge scaled by frame count:
  `measured × (330 + 1,635 n) / (330 + 1,635 n_measured)`.
- **Batch rule** (stricter than the contract's "more than 10"): every request for more than one group
  shows, before anything is sent: "Ask for {n} second opinions? {n} calls to publik-balanced, about
  {each} each (estimate), about {total} in total. Groups already asked are free." Buttons **Ask** and
  **Cancel**. During the batch: "{done} of {n} · last call {charge} · {spent} so far". Calls run one at
  a time.
- **Developer share**: F15 calls are chat calls (`ai.chat`) and earn half of Publik's markup. Whether
  the Gemini route is billed the same way is checked at module 09 from the charge header.
- Dollar formatting: under $0.10 three decimals ("$0.027"), otherwise two ("$3.12").

## 9. Status and errors

**Balance line** (from `x-publik-balance`, "$3.12 of publik balance"): Settings > Second opinion, the
first-run card, and the footer of the second-opinion panel as "Last call $0.027 · $3.12 of publik
balance". Nowhere else; with second opinions off it is never shown.

| Response | Single ask | Batch of close calls |
|---|---|---|
| 402 | Panel: `error.message`, exactly one link (`error.top_up_url`, labelled **Sign in with publik** while anonymous, **Add a plan or pack** once claimed), "or use your own key or a local model in Settings" with **Use my own key** and **Use a local model** (in-app). Nothing cached; needs-credit set | "Stopped after {done} of {n}: {error.message}" with the same link and buttons; finished results stay; the rest are listed as not asked |
| 401, 403 `install_revoked` | "publik API no longer accepts this computer's key." with **Reconnect**; marked disconnected | the same; batch stopped |
| 429 | `error.message`; no retry | the same; batch stopped |
| 503 | "publik API is busy; try again in a moment" with **Try again** (same idempotency key) | the same; batch stopped |
| other 4xx/5xx | "publik API {code}: {error.message}" | the same; batch stopped |
| timeout (60 s) | "publik API did not answer within 60 seconds. Trying again may cost another call." with **Try again** | the same; batch stopped |
| offline | "A second opinion needs a connection or a local model." with **Use a local model** | nothing starts |
| invalid answer | "The AI answer could not be used." (§6.1) | that group marked failed; the batch continues, because the call itself succeeded |

A success after a 402 clears needs-credit. Nothing ever queues silently. The rest of the app keeps
working in every row. Own-key errors use the same table with "your provider" in place of "publik API"
and retry per §3; local errors say "The local model did not answer: {message}".

## 10. Privacy

- **Leaves the machine** (publik or own key): the prepared JPEGs of the frames asked about and the fixed
  prompt text. Nothing else: no file names, folder names, capture times, camera or lens details, serial
  numbers, GPS, face boxes, scores, decisions or other frames. The prompt text contains no user data.
- **Local model**: nothing leaves the machine (the probe test proves no request goes to any host but
  localhost).
- **Stored**: answers in `ai-cache/` on disk (derived, text only, deletable in Settings: "Clear
  second-opinion answers"); the prepared JPEGs are never stored or cached;
  the own key in `secrets.json` (mode 0600); the Publik credential at the contract's path. Logs hold
  codes and counts only.
- **Personal data**: the enable dialog's warning and permission box (§4). Publik keeps no request
  bodies beyond a 24-hour idempotency fingerprint and does not train on them; the model provider's
  abuse-monitoring retention applies. `SECURITY.md` repeats this.

## 11. Tests

All HTTP is faked with a localhost server (`tiny_http`); no test reaches publikhq.com. Code tests in
`crates/keepers-publik/tests/09_publik_second_opinion.rs`, UI logic in
`app/src/state/secondOpinion.test.ts`, golden cases in `tests/cases/09-publik-second-opinion/`.

Contract tests 1 to 18 (`docs/PUBLIK-CONTRACT.md` §11), each a named test. Keepers' specifics:
test 4 checks `publik-balanced` is the only chat model value and `publik-vision` (the Gemini
fallback) the only other one; test 6 runs the
batch variant; test 12 runs a full ask on the local provider with a credential on disk and a fake
publikhq.com that fails the test if contacted; test 15 is K1 and K2 below; test 18 (the self-test)
runs the app binary itself, in `app/src-tauri/tests/09_publik_self_test.rs` (module 09, test 30).

| # | Test | Fake response or input | Expected |
|---|---|---|---|
| K1 | prepare strips and resizes | 6000 × 4000 fixture preview with EXIF, XMP, ICC and IPTC segments | 1568 × 1045; markers exactly SOI, APP0, DQT, SOF0, DHT, SOS, EOI |
| K2 | size budget | six 6000 × 4000 seeded-noise frames | body ≤ 3,900,000 bytes; quality steps as §7 |
| K3 | never upscale | 1200 × 800 preview | 1200 × 800 |
| K4 | prompt render | four frames | text equals `tests/cases/09-publik-second-opinion/prompt-4/expected.txt` |
| K5 | chat body | four frames, publik | keys `model, stream, max_tokens, messages` only; 9 user parts; data URLs `data:image/jpeg;base64,` |
| K6 | Gemini body | route constant set to Gemini | none of the five refused keys; header `x-goog-api-key`; path ends `publik-vision:generateContent` |
| K7 | validation | valid; missing label; duplicate label; extra key; reason 161 chars; prose before JSON; empty reason after trimming | accept only the first |
| K8 | fence | the valid answer inside a `json` code fence | accepted |
| K9 | cut off | `finish_reason: "length"` | "The AI answer could not be used." |
| K10 | cache | the same ask twice; then reversed order | one HTTP call, then a second call |
| K11 | idempotency | an ask; then an invalid answer and **Try again**, twice; then a 503 and **Try again** | first header equals the first 32 hex of the cache key; the retries after invalid answers send the first 32 hex of SHA-256 of the input with `"attempt": 2`, then `3` (computed in the test); the valid answer is cached under the unchanged cache key; the retry after 503 repeats the previous header |
| K12 | $0.00 pre-check | status 0; `/wallet` returns 0 | no `/chat/completions` request; panel text as §4; one URL |
| K13 | 402 mid-batch | five groups; 200, 200, 402 | two results kept and cached; three not asked; one link |
| K14 | confirmation | `second_opinion_run` for 2 groups with `confirmed: false` | `confirmation_required`; no request |
| K15 | measured cost | charge header 27480 | "last call $0.027"; next estimate scaled from 27,480 |
| K16 | advice only | any successful ask, then **Pick this frame** on label C | the ask leaves the journal bytes unchanged; the click appends exactly one line, `flag` with `value: "pick"`, `frames: [C's frame_id]`, `via: "second_opinion"`; the other members' effective decisions are unchanged |
| K17 | consent | `ai_enable` publik without permission | `permission_required`; no `/installs` request |
| K18 | off by default | fresh data directory, full analyse and export | no AI client constructed; panicking transport never called |
| K19 | local discovery | tags with three models, two with `vision` | the larger vision model chosen; missing `capabilities` gives the §3 message |
| K20 | local probe | answers `{"colours":["red","blue"]}` and `{"colours":["red"]}` | pass; fail with the §3 message |
| K21 | own key retries | 503, 503, 200 | three requests, sleeps 2 s and 8 s (injected); success |
| K22 | too few frames | a group with one candidate | "A second opinion needs at least two sharp frames with open eyes." |
| K23 | CLI dry run | `keepers second-opinion <gen/bursts> --group g0001 --dry-run --json` | the fixed fields equal the golden; `width`, `height`, `quality`, `jpeg_bytes`, `body_bytes` and `cache_key` are ignored there and checked by bound (long side ≤ 1568 and ≤ source, quality 85/75/65, each JPEG ≤ B(n), body ≤ 3,900,000 bytes, key 64 hex); no request |
| K24 | offline and timeout | connection refused; no answer in 60 s | the §9 strings; no second attempt |
| K25 | copy rule | every UI string file, `docs/` and `README.md`, except `docs/PUBLIK-CONTRACT.md` and the test itself | none of the contract's forbidden phrases |
| K26 | wallet at start | start-up with the feature off; on with `own`; on with `local`; on with `publik` | no request to publikhq.com in the first three; exactly one `GET /wallet` in the last |

## 12. CONTRIBUTING.md block

```md
## publik API
- Base URL: https://publikhq.com/api/v1 (OpenAI Chat Completions, Responses, Anthropic Messages)
- Models: publik-fast, publik-balanced, publik-smart only; never a vendor model name
- Key: per-install `pk_` from POST /installs, stored at the publik credential path; env PUBLIK_API_KEY / PUBLIK_API_BASE_URL override
- App token: publik-app-token.txt (public by design, committed) or PUBLIK_APP_TOKEN
- 402: show error.message plus error.top_up_url only
- Never print or log a key
- Self-test: PUBLIK_SELF_TEST=1 makes the built app print one `publik self-test:` line and exit
- Docs: https://publikhq.com/llms.txt, https://publikhq.com/developers, https://publikhq.com/skills/setup.md
- Keepers: the only AI feature is the second opinion (F15), off by default, product ai.chat only.
  Tier publik-balanced on /chat/completions with image parts; the Gemini publik-vision route only if
  ADR-0007 says so. Never publik-smart in v1. No retries on publik calls; own key retries twice.
  A second opinion never writes the decision journal.
```
