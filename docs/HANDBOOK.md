# Keepers handbook

The user guide. It grows one module at a time, and every example in it has been run.

## Where things stand

Keepers is pre-alpha. The build, the tests and the command line exist; nothing culls yet. The
chapters below are added as the roadmap (`ROADMAP.md`) progresses: opening a shoot (01),
technical scores (02), faces and eyes (03), bursts and near-duplicates (04), the automatic cull
and its reasons (05), the review window (06), export to your editor (07), speed and hardware
(08), the optional second opinion (09), learning your style (10), and installing (11).

## The command line today

`keepers` prints its version as a JSON document:

```sh
keepers --version --json
```

```json
{"schema":"keepers.version/1","name":"keepers","version":"0.1.0"}
```

Every document Keepers prints starts with a `schema` field, so a script can tell what it is
reading. A command Keepers does not know is a usage error, exit code 1:

```sh
keepers frobnicate --json
```

```json
{"schema":"keepers.error/1","code":"usage","message":"unknown command 'frobnicate'. Run keepers --help."}
```

## The window today

The desktop app opens a window titled "Keepers" showing "Keepers 0.1.0". Opening a folder comes
with module 01 and the review window with module 06.
