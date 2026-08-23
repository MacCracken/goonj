#!/bin/bash
# Bump the project version.
#
# VERSION is the single source of truth; cyrius.cyml derives it via
# version = "${file:VERSION}", so this script only writes VERSION. (The
# Rust-era version of this script also sed'd Cargo.toml — that file is gone
# with the Cyrius port, and `set -e` made the whole script abort before it
# ever wrote VERSION.)
#
# Usage: scripts/version-bump.sh 2.0.1
set -euo pipefail

if [ $# -ne 1 ]; then
    echo "usage: $0 <version>   (e.g. $0 2.0.1)" >&2
    exit 2
fi

VERSION="$1"

# release.yml matches tags against '[0-9]+.[0-9]+.[0-9]+*' and then requires
# VERSION to equal the tag exactly — reject anything that cannot satisfy that.
if ! printf '%s' "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+'; then
    echo "error: '$VERSION' is not a MAJOR.MINOR.PATCH version" >&2
    exit 2
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# cyrius.cyml must keep deriving from VERSION, or the release gate compares
# a stale literal against the tag.
if ! grep -q '^version = "\${file:VERSION}"' "$ROOT/cyrius.cyml"; then
    echo "error: cyrius.cyml no longer derives version from \${file:VERSION};" >&2
    echo "       fix it before bumping, or the release version gate will fail." >&2
    exit 1
fi

printf '%s\n' "$VERSION" > "$ROOT/VERSION"
echo "VERSION -> $VERSION"

# The bundle stamps the version into its header, so it is stale until rebuilt.
echo "next: run 'cyrius distlib' to restamp dist/goonj.cyr, and add a"
echo "      [$VERSION] section to CHANGELOG.md (release.yml extracts it)."
