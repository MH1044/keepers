# Security

## Threat model

Keepers runs on a photographer's computer over photos that are often of other people: clients,
guests, children. The things that must not go wrong, in order:

1. **Originals are never written.** Keepers opens original files read-only, writes sidecars only
   on an explicit Export, and refuses any target path equal to an original's path. A test hashes
   every fixture original before and after every operation.
2. **Nothing leaves the machine without the user asking.** Keepers is local-first. The network
   requests it can make are listed below, and there are no others. No telemetry, no analytics, no
   crash upload, no update check in development builds.
3. **No one is identified.** Keepers finds faces and judges eye state; it never computes or stores
   a face vector, never recognises or names anyone, and its database schema has no column for it.
4. **Decisions are never lost.** The decision journal is append-only; a cache bug cannot delete a
   pick or a reject.
5. **Logs carry no content.** Log lines record stage timings, counts, error codes and frame
   indices: never paths, file names, EXIF values, pixels, keys or balances.

## Every network request the app can make

1. **Model downloads** from the URLs pinned in the model registry, on the user's click, with the
   size shown first, verified by SHA-256 (module 03).
2. **The optional update check** against this repository's public releases API, carrying nothing
   but the request; off in development builds; switchable off in Settings (module 11).
3. **Second opinions**, only after the user has turned them on, read the disclosure and chosen a
   provider (module 09):
   - publik API at `publikhq.com`: `POST /installs` (once, to set up this computer),
     `POST /installs/revoke` (on Disconnect), `GET /wallet` (at start only when publik API is the
     chosen provider, when the Settings page opens, and before a call when the balance reads
     zero), and `POST /chat/completions` or the Gemini route for the opinion itself;
   - the user's own OpenAI-compatible base URL, with the user's own key;
   - `localhost` only, for a local model: `GET /api/tags`, `POST /api/show`,
     `POST /v1/chat/completions`.
4. **The builder's self-test**: one `POST /installs` to `publikhq.com`, only when the app is
   started with `PUBLIK_SELF_TEST` set, after which it exits (module 09). Never in normal use.

What a second opinion sends: reduced copies of the frames asked about (at most 1568 pixels on the
long side, every metadata segment removed) and a fixed prompt. Nothing else: no file names, folder
names, capture times, camera or lens details, serial numbers, GPS, face boxes, scores or
decisions. The `device_name` field of the Publik setup request is not sent.

## Keys and credentials

- The Publik per-install key is written by the core to the Publik credential path with mode 0600
  and is never returned to the window: no command result, event or log contains it, and a test
  searches every serialised result for it.
- The user's own key is stored in `secrets.json` (mode 0600) in the app's data directory and is
  never printed, logged or returned.
- Keys travel only in request headers, never in a URL or on a command line.
- The repository holds no credential. The one deliberate exception, once module 09 is listed, is
  the Publik app token in `publik-app-token.txt`, which is public by design and can only mint
  keys attributed to this app.

## Reporting a vulnerability

Open a private security advisory on this repository's GitHub page ("Security" tab, "Report a
vulnerability"). Please do not open a public issue for a vulnerability. Keepers is pre-alpha and
has no release yet; fixes land on `main` and in the next release.
