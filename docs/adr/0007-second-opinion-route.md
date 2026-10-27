# ADR-0007: Second opinions as chat-completions image parts on `publik-balanced`; the Gemini `publik-vision` route only if the probe says so

- **Status:** Proposed until module 09 (settled by the proof script's probe and the measured
  charges)
- **Date:** 2026-10-27

## Context

The one AI feature, the optional second opinion (`docs/PUBLIK-INTEGRATION.md`), sends two to six
reduced frames of a burst to a vision model and asks for a ranking by expression and moment. The
shared contract (`docs/PUBLIK-CONTRACT.md` §6) names the primary route: OpenAI content parts with
`image_url` data URLs on `/chat/completions`, with a tier name as the model, and records that a
production app fell back to the Gemini `generateContent` dialect at `publik-vision` when a tier
refused image parts. Publik settles usage even when the client gives up, so a retry on another
route can charge twice. Image tokens, the uncertain part of the cost, can only be measured with a
real call.

## Decision

Keepers sends second opinions as chat-completions image parts on `publik-balanced`: frames
resized to at most 1568 px on the long side, re-encoded as baseline JPEG with every metadata
segment removed, the request under 3.9 MB. The route is a build constant, never switched at run
time. The Gemini `publik-vision` route exists in the code and its tests, and becomes the build
constant only if the probe in `scripts/publik-proof.sh --vision` (run once by the maintainer at
module 09) shows that `publik-balanced` refuses image parts. The probe also records whether the
tier honours `response_format` (not sent by default) and the measured charge per frame count,
which replaces the estimates in `docs/PUBLIK-INTEGRATION.md` §8. `publik-smart` is never
requested in v1.

## Consequences

- One request per ask, one charge, never a silent second attempt; "Try again" is the user's
  click and says it may cost another call.
- The own-key and local providers use the same chat-completions body with only the model
  changed, so one client serves all three.
- Until the probe runs, the cost figures are estimates and are labelled as such in the UI.
- If the probe selects the Gemini route, the system prompt travels as the first text part (the
  dialect refuses `systemInstruction`), and the answer is read from the candidates' text parts;
  validation from step 2 on is shared.

## Alternatives considered

- **Automatic dialect switching at run time** (try chat completions, fall back to Gemini on
  refusal): robust to Publik changes. Rejected because a failed first attempt may still be
  charged, and a photographer asking about sixty groups would pay twice without knowing.
- **Sending 512 px frames on the Gemini route** (the earlier plan): cheaper. Rejected because
  expressions and half-closed eyes are what the opinion is for, and they are lost at 512 px; the
  contract's primary route and size are the default.
- **Face crops instead of whole frames**: fewer tokens. Deferred to "Later" in `docs/PRODUCT.md`;
  the moment of a frame is more than its faces.
