#!/bin/sh
# check-namespace.sh -- fail if a top-level goonj name is also defined in the stdlib.
#
# A bundle shares one flat namespace with everything in the consumer's unit, and a
# redefined enum member takes the last definition at every use, including uses compiled
# before it. The compiler only warns, and only in the consumer's build. Through 2.3.1
# goonj's DH_SIZE met yukti's and its LOG_* met stdlib log's, and both changed what goonj
# did in a consumer's unit (tests/namespace.tcyr).
#
# This reads every top-level name in dist/goonj.cyr (fns, vars, consts, structs, enums
# and enum members) and every top-level name in each stdlib module of the pinned
# toolchain, and fails on any name in both. Extra directories of .cyr files, such as a
# consumer's lib/, can be passed to check those too.
#
# Usage: scripts/check-namespace.sh [dir-of-cyr-files ...]
# Env:   GOONJ_BUNDLE (default dist/goonj.cyr), CYRIUS_STDLIB (default: the pin's lib/).
set -eu
export LC_ALL=C   # sort and comm must agree on the order of `_`

cd "$(dirname "$0")/.."
bundle="${GOONJ_BUNDLE:-dist/goonj.cyr}"
pin="$(grep '^cyrius = ' cyrius.cyml | head -1 | sed 's/cyrius = "\(.*\)"/\1/')"

stdlib="${CYRIUS_STDLIB:-}"
if [ -z "$stdlib" ]; then
    if [ -d "$HOME/.cyrius/versions/$pin/lib" ]; then
        stdlib="$HOME/.cyrius/versions/$pin/lib"
    else
        stdlib="$HOME/.cyrius/lib"
    fi
fi
if [ ! -d "$stdlib" ]; then
    echo "check-namespace: no stdlib at $stdlib (pin $pin)" >&2
    exit 2
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# One name per line. Comments are stripped; an enum's members are read until its `}`,
# separated by `,` or `;`.
cat > "$tmp/names.awk" <<'EOF'
function members(s,    n, i, p, parts) {
    n = split(s, parts, /[,;]/)
    for (i = 1; i <= n; i++) {
        p = parts[i]
        gsub(/^[ \t]+/, "", p)
        if (match(p, /^[A-Za-z_][A-Za-z_0-9]*/)) print substr(p, RSTART, RLENGTH)
    }
}
{
    line = $0
    sub(/#.*/, "", line)
    if (inenum) {
        if (line ~ /}/) { sub(/}.*/, "", line); members(line); inenum = 0 }
        else members(line)
        next
    }
    if (line ~ /^(pub |public )?enum [A-Za-z_]/) {
        t = line
        sub(/^(pub |public )?enum /, "", t)
        match(t, /^[A-Za-z_][A-Za-z_0-9]*/)
        print substr(t, RSTART, RLENGTH)
        if (line ~ /{/) {
            body = line
            sub(/^[^{]*{/, "", body)
            if (body ~ /}/) { sub(/}.*/, "", body); members(body) }
            else { members(body); inenum = 1 }
        }
        next
    }
    if (line ~ /^(pub |public )?(fn|var|const|struct) [A-Za-z_]/) {
        t = line
        sub(/^(pub |public )?(fn|var|const|struct) /, "", t)
        match(t, /^[A-Za-z_][A-Za-z_0-9]*/)
        print substr(t, RSTART, RLENGTH)
    }
}
EOF

awk -f "$tmp/names.awk" "$bundle" | sort -u > "$tmp/goonj"
count="$(wc -l < "$tmp/goonj" | tr -d ' ')"
if [ "$count" -lt 500 ]; then
    # 2.3.2 reads 853. A parser that stops seeing declarations would pass everything.
    echo "check-namespace: read only $count names from $bundle; the parser is broken" >&2
    exit 2
fi

: > "$tmp/hits"
for dir in "$stdlib" "$@"; do
    for f in "$dir"/*.cyr; do
        [ -f "$f" ] || continue
        case "$(basename "$f")" in goonj.cyr) continue ;; esac
        awk -f "$tmp/names.awk" "$f" | sort -u | comm -12 "$tmp/goonj" - |
            sed "s|^|$f: |" >> "$tmp/hits"
    done
done

if [ -s "$tmp/hits" ]; then
    echo "check-namespace: goonj names also defined elsewhere (last definition wins):" >&2
    cat "$tmp/hits" >&2
    exit 1
fi
echo "check-namespace: $count goonj names, none defined in $stdlib${*:+ or $*}"
