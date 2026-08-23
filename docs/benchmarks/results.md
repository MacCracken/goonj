# goonj — Benchmark results

Measured with `cyrius bench tests/<suite>.bcyr` (toolchain **6.5.35**, x86_64,
idle host). Cyrius figures are the **min** the harness reports — as of Cyrius
6.5.19 that is a min over *per-chunk averages*, net of a per-clock-read floor the
harness measures and prints on the `[timer floor …]` line of every run (this run:
1.33–1.35 µs). Treat all figures as relative, not absolute (machine-dependent).
Rust baselines and the full 34-benchmark cross-language table live in the
**[Rust → Cyrius comparison](../benchmarks-rust-vs-cyrius.md)**.

> **Numbers recorded on toolchain 6.3.14 (i.e. at v2.0.0) are not comparable to
> these below ~10 µs.** The 6.5.19 harness measures and subtracts its clock floor
> and auto-sizes batches; the older one wrapped a clock pair around every
> iteration and so carried ~489 ns of additive per-iteration overhead. The floor
> is per-boot (`lib/bench.cyr` documents ~400 ns–1,700 ns on this host), so
> cross-boot comparison of micro rows is unsound. See the methodology banner in
> the cross-language doc.

## ray (`tests/ray.bcyr`)

Two scenes, matching the Rust `ray/*` benchmarks config-for-config:
- **shoebox** — 6-wall concrete shoebox (10×8×3 m), source (5, 1.5, 4), 50-bounce traces.
- **100-wall room** — a 20×20×3 m shoebox plus 94 drywall partitions
  (`make_many_wall_room(94)`), ray from (10, 1.5, 10) along +X, 50 bounces.

| Benchmark | Cyrius (min) | Rust | ratio | iters |
|-----------|-------------:|-----:|------:|------:|
| `ray/wall_intersection`      | 215 ns   | 7 ns    | ~31× | 200,000 |
| `ray/trace_single_shoebox`   | 53.53 µs | 2.58 µs | ~21× | 20,000 |
| `ray/trace_100_rays_shoebox` | 5.348 ms | 221 µs  | ~24× | 200 |
| `ray/trace_linear_100_walls` | 1.493 ms | 48.1 µs | ~31× | 3,000 |
| `ray/trace_bvh_100_walls`    | 1.811 ms | 75.6 µs | ~24× | 3,000 |

**Finding — allocation, not logic.** These are the suite's alloc-heaviest ops:
`wall_intersection` allocates a fresh `HVec3` per call, and each trace allocates a
`RayBounce` + multiband-ray clone per bounce. Against a bump allocator that never
frees within a run, that allocation *is* the cost — hence the ~21–31× gaps here
against the honest ~6–17× on compute-bound loops. The thesis got a direct test at
2.0.1: Cyrius 6.3.17 gave the stdlib allocator a single-threaded lock-elision fast
path, and this table roughly halved (e.g. `trace_100_rays_shoebox` 12.58 → 5.35 ms)
while the compute-bound suites moved by ~1%. An arena/pool allocator collapses more
of it (post-2.0.0 backlog).

Two caveats on that halving: part of the `wall_intersection` movement (907 → 215 ns)
is the retired harness floor rather than real work, and the rest of the ray table's
gain has not been bisected between the allocator change, hisab 2.6.7 → 2.11.2, and
6.5.35 codegen. Don't attribute it to a single cause.

**Finding — BVH vs linear at 100 walls stays ~even** (1.811 vs 1.493 ms), and it's
~even in Rust too (75.6 vs 48.1 µs). The 94 axis-aligned partitions don't cull, so
the BVH crossover is past 100 walls in *both* languages — not a Cyrius artifact.
BVH now trails linear by 21% (it was 9% at v2.0.0). Both paths produce identical
bounce sequences (`bvh_trace_matches_linear_trace`), which still holds after
hisab 2.11.2 changed the BVH's degenerate-partition split from 1-vs-(n−1) to
median — that reshapes the tree, so query order can differ on exact float ties.

## dwm (`tests/dwm.bcyr`)

`solve_dwm_3d` on a rigid box (~220 steps at 22,050 Hz for the 0.01 s configs),
plus the dispersion-correction FIR.

| Benchmark | Cyrius (min) | iters |
|-----------|-------------:|------:|
| `solve_dwm_3d_16cubed_0.01s` (4,096 cells) | 57.26 ms | 30 |
| `solve_dwm_3d_24cubed_0.01s` (13,824 cells) | 195.1 ms | 15 |
| `solve_dwm_3d_30x25x20_50ms` (15,000 cells, 1,102 steps) | 1.052 s | 5 |
| `dwm/dispersion_correct_22kHz_1s` | 311.1 µs | 2,000 |

`solve_dwm_3d_30x25x20_50ms` matches the Rust `dwm/solve_30x25x20_50ms` grid and
step count (15,000 cells, 1,102 steps; Rust 74.2 ms → **~14×**). Two differences
that do not touch the hot loop: Rust ran f32 where this runs f64, and Rust passed
one receiver where this passes none — the cleanest codegen+precision gap in the
whole suite, a tight `f64` triple loop over raw `load64`/`store64` buffers with
**no per-iteration allocation**. It's the direct target of the number-type + SIMD
work, and it moved only −1.7% across the 6.3.14 → 6.5.35 bump, which is exactly
what a non-allocating loop should do. `dispersion_correct` is alloc-heavy by
contrast (~27× vs Rust's 11.4 µs).

**Finding — near-linear in cell count.** 3.375× the cells (16³→24³) costs ~3.41×
the time (57.26 → 195.1 ms), consistent with the O(cells · steps) grid sweep. Each
node does 6 waveguide-port updates per step; the hot loop uses double-buffered
outgoing waves.

Note that `solve_dwm_3d_24cubed_0.01s` and `solve_dwm_3d_30x25x20_50ms` now report
min == avg == max: one auto-sized chunk covers every iteration, so those two rows
carry no dispersion information.

## Recording history (`scripts/bench-history.sh`)

The `.bcyr` history CSV now exists. `scripts/bench-history.sh` runs every
`tests/*.bcyr` suite (16 today, discovered by glob rather than hardcoded) and
appends one row per benchmark to `bench-history.csv`:

```sh
scripts/bench-history.sh                 # every suite
scripts/bench-history.sh dwm ray         # named suites only
scripts/bench-history.sh -n              # parse + print, append nothing
```

A full sweep is ~48 s and yields 37 rows. Columns:

| Column | Meaning |
|--------|---------|
| `run_utc`, `host`, `boot_id` | when, where, and under which boot |
| `cyrius`, `commit`, `dirty` | toolchain version, short SHA, `1` if the tree had uncommitted changes |
| `suite`, `benchmark` | `tests/<suite>.bcyr` and the `bench_report` name |
| `avg_ns`, `min_ns`, `max_ns`, `iters` | the report row, every time in integer nanoseconds |
| `floor_ns` | the `[timer floor …]` value **this suite's process** measured |
| `cmp` | `global` if `min_ns` ≥ 10 µs, else `boot` |

Two things make the caveat at the top of this page mechanical instead of
remembered. First, `floor_ns` is recorded per suite, not per run — the floor is
printed once per process and each suite is its own process, so the value stored
is the one actually subtracted from that suite's samples. Second, `cmp=boot`
marks every row whose `min_ns` is under 10 µs: **those rows are only comparable
against rows sharing the same `boot_id`.** The floor is per-boot and ranges
~400 ns–1,700 ns on this host, which swamps a benchmark like
`propagation/speed_of_sound` (5 ns min) many times over. Filter on `cmp=global`
for any cross-boot or cross-toolchain trend.

The script parses `lib/bench.cyr`'s output with the exact glob that file's
comments promise (`*": "*" avg"*`), so the `[timer floor …]` line — which
deliberately omits `" avg"` — cannot land in the CSV as a fake benchmark. A row
that matches the glob but not the expected shape is reported as format drift
rather than silently dropped, and a run that parses nothing writes no file at
all.
