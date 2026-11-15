#!/bin/sh
# run.sh -- check the grid solvers' bits against the Rust original (ADR-0010).
#
# Extracts rust-old at tag 2.1.3 into build/rust-parity, generates the same cases for Rust
# and for dist/goonj.cyr from each seed, runs both, and compares them value by value. Each
# Cyrius run is repeated on the 128-bit path and on the 256-bit path from 8 elements. Then
# it runs tests/hardening.tcyr's golden sweeps through Rust, whose hashes that test pins.
#
# Needs cargo, and network access the first time for Rust's crates.
# Usage: scripts/rust-parity/run.sh [seed ...]     (no seed: the fixed one, 20261115)
set -eu
cd "$(dirname "$0")/../.."
here=scripts/rust-parity
work=build/rust-parity
mkdir -p "$work/cy/dist"
[ -d "$work/rust-old" ] || git archive 2.1.3 rust-old | tar -x -C "$work"
cp "$here/f32dump.rs" "$here/hsweeps.rs" "$work/rust-old/examples/"
rm -rf "$work/cy/lib"
cp -r lib "$work/cy/lib"
cp dist/goonj.cyr "$work/cy/dist/"
cp "$here/harness_tail.cyr.in" "$work/tail_default.cyr"
sed 's/^run_cases();/_row_wide = 0;\nrun_cases();/' "$here/harness_tail.cyr.in" > "$work/tail_narrow.cyr"
sed 's/^run_cases();/_row_wide = simd_has_avx2(); ROW_WIDE_MIN = 1;\nrun_cases();/' \
    "$here/harness_tail.cyr.in" > "$work/tail_wide.cyr"

cargo_q() {
    (cd "$work/rust-old" && cargo "$@" > ../cargo.log 2>&1) || { cat "$work/cargo.log"; exit 1; }
}

for seed in ${*:-20261115}; do
    python3 -I "$here/gen.py" "$work/rust-old/examples/cases_gen.rs" "$work/cases_$seed.cyr" "$seed" > /dev/null
    cargo_q build --release --example f32dump
    "$work/rust-old/target/release/examples/f32dump" > "$work/rust_$seed.out"
    for mode in default narrow wide; do
        cat "$here/harness_head.cyr.in" "$work/cases_$seed.cyr" "$work/tail_$mode.cyr" > "$work/cy/dump_$mode.cyr"
        (cd "$work/cy" && cyrius build --no-deps "dump_$mode.cyr" "dump_$mode" > ../cyrius.log 2>&1) \
            || { cat "$work/cyrius.log"; exit 1; }
        (cd "$work/cy" && "./dump_$mode" > "../cy_${seed}_$mode.out" 2> /dev/null)
        echo "seed $seed $mode:"
        python3 -I "$here/compare.py" "$work/rust_$seed.out" "$work/cy_${seed}_$mode.out" "$work/cases_$seed.cyr" \
            | sed 's/^/  /'
    done
done

echo "golden sweeps through Rust 2.1.3 (tests/hardening.tcyr pins these, as signed i64):"
cargo_q build --release --example hsweeps
"$work/rust-old/target/release/examples/hsweeps" | sed 's/^/  /'
