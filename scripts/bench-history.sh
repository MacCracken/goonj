#!/bin/bash
# Record goonj benchmark results to a history CSV.
#
# Runs `cyrius bench tests/<suite>.bcyr` over the .bcyr suites and appends one
# CSV row per benchmark, so a regression shows up as a diff instead of as a
# vague memory of what the number used to be. lib/hisab.cyr cites this file by
# name ("[measured: bench-history.csv ray_aabb_diag]") — those citations are
# only checkable if the CSV actually exists and keeps accumulating.
#
# The Rust-era version of this script ran `cargo bench --bench benchmarks`.
# There has been no root Cargo.toml since the Cyrius port (the Rust tree is
# frozen at rust-old/ as a parity oracle), so it exited 101 and — because it
# piped through `tee -a` — left a 0-byte bench-history.csv behind every time.
# This rewrite never creates the CSV until it has rows to put in it.
#
# ── Output format contract ────────────────────────────────────────────────
# lib/bench.cyr writes both lines this script parses, and its comments name
# this script as the consumer. Keep the two in sync.
#
#   lib/bench.cyr:bench_report()       ->
#     "  wav/export_48k_2s: 2.081ms avg (min=2.017ms max=2.564ms) [500 iters]"
#   lib/bench.cyr:bench_report_clock() ->
#     "  [timer floor 1.332us per clock read, measured; subtracted from every sample]"
#
# Result rows are selected with the glob `*": "*" avg"*` — the exact pattern
# lib/bench.cyr:456 promises this script uses. The floor line deliberately
# omits " avg" so it cannot land in the CSV as a fake benchmark; do not
# loosen the gate.
#
# Times are `major.fff<unit>` with the fraction zero-padded to 3 digits
# (lib/bench.cyr:_fmt_pad3 — dropping the pad silently turned 1050ns into
# "1.50us", a 45% error, issue PF-01). Every field is stored as integer
# nanoseconds so the CSV never has to be re-parsed with unit awareness.
#
# ── Why boot_id and cmp are columns ───────────────────────────────────────
# bench_report_clock() measures the clock-read floor per process and
# subtracts it from every sample. That floor is a property of the boot, not
# of the code: it ranges ~400ns to ~1700ns on this host, so a sub-10us figure
# recorded before a reboot is not comparable with one recorded after. Rather
# than leave that as a caveat someone has to remember, every row carries the
# floor that was subtracted from it, the boot it was measured under, and a
# `cmp` flag saying which comparisons are valid:
#
#   cmp=global  min >= 10us  — floor is noise; compare across boots freely
#   cmp=boot    min <  10us  — only compare against rows with the same boot_id
#
# Usage:
#   scripts/bench-history.sh                 # every tests/*.bcyr suite
#   scripts/bench-history.sh dwm ray         # named suites only
#   scripts/bench-history.sh -n              # parse + print, append nothing
#   scripts/bench-history.sh -o /tmp/x.csv   # alternate history file
#
# Env: BENCH_HISTORY_CSV overrides the default output path.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# A sample at or above this many ns is far enough above the per-boot clock
# floor (~400-1700ns) that the floor cannot meaningfully skew it.
FLOOR_SENSITIVE_NS=10000

CSV="${BENCH_HISTORY_CSV:-$ROOT/bench-history.csv}"
DRY_RUN=0

usage() {
    sed -n '2,/^set -euo/p' "$0" | sed 's/^# \{0,1\}//; $d'
    exit "${1:-0}"
}

SUITE_ARGS=()
while [ $# -gt 0 ]; do
    case "$1" in
        -h|--help)    usage 0 ;;
        -n|--dry-run) DRY_RUN=1; shift ;;
        -o|--output)  [ $# -ge 2 ] || { echo "error: $1 needs a path" >&2; exit 2; }
                      CSV="$2"; shift 2 ;;
        -*)           echo "error: unknown option '$1'" >&2; usage 2 >&2 ;;
        *)            SUITE_ARGS+=("$1"); shift ;;
    esac
done

command -v cyrius >/dev/null 2>&1 || {
    echo "error: 'cyrius' not on PATH — this script benches the Cyrius port," >&2
    echo "       not the frozen Rust tree at rust-old/." >&2
    exit 127
}

# ── Resolve the suite list ────────────────────────────────────────────────
# Discovered, not hardcoded: a literal list of the current 16 suites would go
# stale the first time someone adds a .bcyr and forgets this file.
SUITES=()
if [ ${#SUITE_ARGS[@]} -gt 0 ]; then
    for a in "${SUITE_ARGS[@]}"; do
        a="${a#tests/}"; a="${a%.bcyr}"
        [ -f "$ROOT/tests/$a.bcyr" ] || { echo "error: no such suite: tests/$a.bcyr" >&2; exit 2; }
        SUITES+=("$a")
    done
else
    for f in "$ROOT"/tests/*.bcyr; do
        [ -e "$f" ] || { echo "error: no tests/*.bcyr suites found under $ROOT" >&2; exit 1; }
        f="${f##*/}"
        SUITES+=("${f%.bcyr}")
    done
fi

# ── Run provenance ────────────────────────────────────────────────────────
RUN_UTC="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
HOST="$(uname -n 2>/dev/null || echo unknown)"

# Per-boot identity, so `cmp=boot` rows can actually be grouped. Linux exposes
# a UUID directly; elsewhere fall back to a boot timestamp, then to unknown.
if [ -r /proc/sys/kernel/random/boot_id ]; then
    BOOT_ID="$(< /proc/sys/kernel/random/boot_id)"
elif BOOT_ID="$(sysctl -n kern.boottime 2>/dev/null)"; then
    BOOT_ID="$(printf '%s' "$BOOT_ID" | tr -cd '0-9')"
else
    BOOT_ID="unknown"
fi

CYRIUS_VER="$(cyrius --version 2>/dev/null | head -1 | tr -cd '0-9.' || true)"
[ -n "$CYRIUS_VER" ] || CYRIUS_VER="unknown"

if COMMIT="$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null)"; then
    # A dirty tree's numbers are not reproducible from the commit alone, so
    # say so in the row rather than implying the commit produced them.
    if [ -n "$(git -C "$ROOT" status --porcelain 2>/dev/null)" ]; then DIRTY=1; else DIRTY=0; fi
else
    COMMIT="unknown"; DIRTY=0
fi

# ── Helpers ───────────────────────────────────────────────────────────────

# "2.081ms" -> 2081000. Units are checked longest-suffix-first because ns, us
# and ms all end in "s". 10# forces base 10: the zero-padded fraction makes
# "052" octal (=42) and "1.098us" an outright syntax error without it.
to_ns() {
    local t="$1" major minor
    case "$t" in
        *ns) printf '%s' "$((10#${t%ns}))"; return ;;
        *us) t="${t%us}"; major=1000 ;;
        *ms) t="${t%ms}"; major=1000000 ;;
        *s)  t="${t%s}";  major=1000000000 ;;
        *)   return 1 ;;
    esac
    if [ "${t#*.}" = "$t" ]; then minor=0; else minor="${t#*.}"; t="${t%%.*}"; fi
    printf '%s' "$(( 10#$t * major + 10#$minor * (major / 1000) ))"
}

# Bench names are [A-Za-z0-9_/.-] today; quote defensively so a future name
# with a comma cannot silently shift every column to its right.
csv_field() {
    case "$1" in
        *[,\"$'\n']*) printf '"%s"' "${1//\"/\"\"}" ;;
        *)            printf '%s' "$1" ;;
    esac
}

TMPDIR_RUN="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_RUN"' EXIT
ROWS="$TMPDIR_RUN/rows.csv"
: > "$ROWS"

TIME_RE='[0-9]+\.?[0-9]*(ns|us|ms|s)'
ROW_RE="^[[:space:]]*(.+): ($TIME_RE) avg \(min=($TIME_RE) max=($TIME_RE)\) \[([0-9]+) iters\][[:space:]]*$"
FLOOR_RE="\[timer floor ($TIME_RE) per clock read"

failed_suites=()
warned=0
n_rows=0
floor_min=""
floor_max=""

# ── Run the suites ────────────────────────────────────────────────────────
echo "bench-history: ${#SUITES[@]} suite(s), cyrius $CYRIUS_VER, commit $COMMIT$([ "$DIRTY" = 1 ] && echo ' (dirty)')"

for suite in "${SUITES[@]}"; do
    log="$TMPDIR_RUN/$suite.log"
    rc=0
    # bench_report writes to fd 1; the toolchain's notes go to fd 2. Merge
    # both so a build failure is captured in the same place we parse from.
    cyrius bench "tests/$suite.bcyr" > "$log" 2>&1 || rc=$?

    # The floor is printed once per process, and each suite is its own
    # process — so this is the floor actually subtracted from this suite's
    # samples, not a run-wide constant.
    floor_ns=""
    if [[ "$(grep -m1 -F '[timer floor ' "$log" || true)" =~ $FLOOR_RE ]]; then
        floor_ns="$(to_ns "${BASH_REMATCH[1]}")"
        [ -n "$floor_min" ] && [ "$floor_min" -le "$floor_ns" ] || floor_min="$floor_ns"
        [ -n "$floor_max" ] && [ "$floor_max" -ge "$floor_ns" ] || floor_max="$floor_ns"
    fi

    suite_rows=0
    while IFS= read -r line; do
        # The gate lib/bench.cyr:456 promises we use. Anything without " avg"
        # — the floor line, toolchain notes, the pass/fail banner — is out.
        case "$line" in *": "*" avg"*) ;; *) continue ;; esac

        if [[ ! "$line" =~ $ROW_RE ]]; then
            # Matched the contract's gate but not its shape: lib/bench.cyr
            # changed format. Loud, because silently dropping the row would
            # look identical to the benchmark having been deleted.
            echo "warning: $suite: unparseable result row (bench_report format drift?):" >&2
            echo "         $line" >&2
            warned=1
            continue
        fi

        name="${BASH_REMATCH[1]}"
        avg_ns="$(to_ns "${BASH_REMATCH[2]}")"
        min_ns="$(to_ns "${BASH_REMATCH[4]}")"
        max_ns="$(to_ns "${BASH_REMATCH[6]}")"
        iters="${BASH_REMATCH[8]}"

        if [ "$min_ns" -lt "$FLOOR_SENSITIVE_NS" ]; then cmp=boot; else cmp=global; fi

        printf '%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
            "$RUN_UTC" "$(csv_field "$HOST")" "$BOOT_ID" "$CYRIUS_VER" \
            "$COMMIT" "$DIRTY" "$(csv_field "$suite")" "$(csv_field "$name")" \
            "$avg_ns" "$min_ns" "$max_ns" "$iters" "${floor_ns:-}" "$cmp" >> "$ROWS"

        suite_rows=$((suite_rows + 1))
    done < "$log"

    n_rows=$((n_rows + suite_rows))

    if [ "$rc" -ne 0 ] || [ "$suite_rows" -eq 0 ]; then
        failed_suites+=("$suite")
        echo "  $suite: FAILED (exit $rc, $suite_rows rows)" >&2
        sed 's/^/    | /' "$log" | tail -15 >&2
    else
        printf '  %-16s %2d rows  floor %sns\n' "$suite" "$suite_rows" "${floor_ns:-?}"
    fi
done

# ── Emit ──────────────────────────────────────────────────────────────────
HEADER='run_utc,host,boot_id,cyrius,commit,dirty,suite,benchmark,avg_ns,min_ns,max_ns,iters,floor_ns,cmp'

if [ "$n_rows" -eq 0 ]; then
    # Deliberately do not touch $CSV. The old script's `tee -a` created an
    # empty file on every failed run; an empty history is worse than none,
    # because it looks like a run that legitimately measured nothing.
    echo "error: no benchmark rows parsed — nothing written to $CSV" >&2
    exit 1
fi

if [ "$DRY_RUN" = 1 ]; then
    echo "--- dry run: would append $n_rows row(s) to $CSV ---"
    echo "$HEADER"
    cat "$ROWS"
else
    if [ ! -s "$CSV" ]; then
        mkdir -p "$(dirname "$CSV")"
        printf '%s\n' "$HEADER" > "$CSV"
    elif [ "$(head -1 "$CSV")" != "$HEADER" ]; then
        echo "error: $CSV has a different header — refusing to append mismatched columns." >&2
        echo "       expected: $HEADER" >&2
        echo "       found:    $(head -1 "$CSV")" >&2
        exit 1
    fi
    cat "$ROWS" >> "$CSV"
    echo "appended $n_rows row(s) -> $CSV"
fi

if [ -n "$floor_min" ]; then
    echo "timer floor this run: ${floor_min}ns..${floor_max}ns (per-boot; boot_id $BOOT_ID)"
    echo "rows with min < $((FLOOR_SENSITIVE_NS / 1000))us are marked cmp=boot — compare those only within this boot_id."
fi

if [ ${#failed_suites[@]} -gt 0 ]; then
    echo "error: ${#failed_suites[@]} suite(s) produced no results: ${failed_suites[*]}" >&2
    exit 1
fi
[ "$warned" = 0 ] || exit 1
