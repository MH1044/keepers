#!/bin/sh
# The first step of every release: refuses to go on without the release notes, so a tag can
# never publish a release that has nothing to say.
#
#   scripts/release-check.sh <version>      exits 0 when docs/releases/<version>.md exists
set -eu

version="${1:-}"
if [ -z "$version" ]; then
  echo "usage: scripts/release-check.sh <version>" >&2
  exit 2
fi

root="$(cd "$(dirname "$0")/.." && pwd)"
notes="docs/releases/$version.md"
if [ ! -s "$root/$notes" ]; then
  echo "$notes is missing; write the release notes first." >&2
  exit 1
fi
echo "release notes found: $notes"
