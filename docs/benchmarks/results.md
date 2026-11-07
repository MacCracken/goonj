# goonj — Benchmark results

Measured with `cyrius bench tests/<suite>.bcyr` (goonj **2.3.0**, toolchain **6.7.18**,
x86_64, idle host). Cyrius figures are the **min** the harness reports — as of Cyrius
6.5.19 that is a min over *per-chunk averages*, net of a per-clock-read floor the
harness measures and prints on the `[timer floor …]` line of every run (this run:
1.21–1.35 µs). Treat all figures as relative, not absolute (machine-dependent).
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
| `ray/wall_intersection`      | 99 ns    | 7 ns    | ~14× | 200,000 |
| `ray/trace_single_shoebox`   | 25.42 µs | 2.58 µs | ~10× | 20,000 |
| `ray/trace_100_rays_shoebox` | 2.04 ms | 221 µs  | ~9× | 200 |
| `ray/trace_linear_100_walls` | 354.7 µs | 48.1 µs | ~7× | 3,000 |
| `ray/trace_bvh_100_walls`    | 560.9 µs | 75.6 µs | ~7× | 3,000 |

**Finding — allocation, then logic.** These were the suite's alloc-heaviest ops: 15
allocations per `wall_intersection` and about 50 per bounce through 2.2.0. 2.2.1 moved
those temporaries to the stack ([ADR-0007](../adr/0007-stack-buffers-over-an-arena.md)),
which took them from ~16–23× to ~11–15×. 2.3.0 took them to ~7–14× by skipping polygon
tests that can't win and spelling out the one that runs most. The thesis got a direct test at
2.0.1: Cyrius 6.3.17 gave the stdlib allocator a single-threaded lock-elision fast
path, and this table roughly halved (e.g. `trace_100_rays_shoebox` 12.58 → 5.35 ms)
while the compute-bound suites moved by ~1%.

Two caveats on that halving: part of the `wall_intersection` movement (907 → 215 ns)
is the retired harness floor rather than real work, and the rest of the ray table's
gain has not been bisected between the allocator change, hisab 2.6.7 → 2.11.2, and
6.5.35 codegen. Don't attribute it to a single cause.

**Finding — BVH vs linear at 100 walls stays ~even** (560.9 vs 354.7 µs), and it's
~even in Rust too (75.6 vs 48.1 µs). The 94 axis-aligned partitions don't cull, so
the BVH crossover is past 100 walls in *both* languages — not a Cyrius artifact.
BVH now trails linear by 58% (it was 9% at v2.0.0, and is 57% in Rust), because the linear
path lost all its temporaries and the BVH query still allocates inside hisab. Both paths produce identical
bounce sequences (`bvh_trace_matches_linear_trace`), which still holds after
hisab 2.11.2 changed the BVH's degenerate-partition split from 1-vs-(n−1) to
median — that reshapes the tree, so query order can differ on exact float ties.

## dwm (`tests/dwm.bcyr`)

`solve_dwm_3d` on a rigid box (~220 steps at 22,050 Hz for the 0.01 s configs),
plus the dispersion-correction FIR.

| Benchmark | Cyrius (min) | iters |
|-----------|-------------:|------:|
| `solve_dwm_3d_16cubed_0.01s` (4,096 cells) | 12.62 ms | 30 |
| `solve_dwm_3d_24cubed_0.01s` (13,824 cells) | 34.62 ms | 15 |
| `solve_dwm_3d_30x25x20_50ms` (15,000 cells, 1,102 steps) | 154.8 ms | 5 |
| `dwm/dispersion_correct_22kHz_1s` | 10.98 µs | 2,000 |

`solve_dwm_3d_30x25x20_50ms` matches the Rust `dwm/solve_30x25x20_50ms` grid and
step count (15,000 cells, 1,102 steps; Rust 74.2 ms → **~2×**). Two differences that do
not touch the hot loop: Rust ran f32 where this runs f64, and Rust passed one receiver
where this passes none. Since 2.3.0 the solver runs a row at a time through the `f64v_*`
builtins ([ADR-0008](../adr/0008-row-kernels-for-grid-solvers.md)), which took it from
~15×. `dispersion_correct` got the same treatment and runs on the 256-bit kernels, and at
10.98 µs it edges past Rust's 11.4 µs.

**Finding — better than linear in cell count now.** 3.375× the cells (16³→24³) costs ~2.74×
the time (12.62 → 34.62 ms), because the end nodes of each row stay scalar and a longer
row spends more of its time in the array kernel.

Note that `solve_dwm_3d_24cubed_0.01s` and `solve_dwm_3d_30x25x20_50ms` now report
min == avg == max: one auto-sized chunk covers every iteration, so those two rows
carry no dispersion information.

## What moved from 6.5.35 to 2.3.0

Measured against the last baseline (2.0.1 on 6.5.35), then against clean 2.1.2 on
6.7.18 to isolate each release's work. Figures are `min`.

| Benchmark | 6.5.35 | 2.1.2 on 6.7.18 | 2.1.3 | 2.2.1 | 2.3.0 |
|-----------|-------:|----------------:|------:|------:|------:|
| `solve_dwm_3d_30x25x20_50ms` | 1.059 s | 1.093 s | 1.064 s | 1.128 s | **154.8 ms** |
| `dwm/dispersion_correct_22kHz_1s` | 313.5 µs | 322.6 µs | 324.2 µs | 327.8 µs | **10.98 µs** |
| `fdtd/solve_40x40_50ms` | 18.83 ms | 18.80 ms | 18.88 ms | 19.40 ms | **6.99 ms** |
| `ray/wall_intersection` | 215 ns | 173 ns | 158 ns | **106 ns** | 99 ns |
| `ray/trace_linear_100_walls` | 1.509 ms | 1.105 ms | 1.022 ms | **673.1 µs** | **354.7 µs** |
| `ray/trace_bvh_100_walls` | 1.816 ms | 1.280 ms | 1.198 ms | **866.9 µs** | **560.9 µs** |
| `impulse/generate_ir_shoebox` | 60.28 ms | 49.03 ms | 44.56 ms | **30.33 ms** | **23.24 ms** |
| `diffuse/1000_rays_shoebox` | 57.74 ms | 42.63 ms | 42.16 ms | **27.24 ms** | **20.77 ms** |
| `image_source/shoebox_order_5` | 402.3 µs | 438.2 µs | **161.5 µs** | 154.1 µs | 157.7 µs |
| `image_source/shoebox_order_3` | 56.67 µs | 61.45 µs | **40.51 µs** | 38.12 µs | 38.62 µs |
| `propagation/ground_reflection` | 145 ns | 256 ns | 235 ns | 240 ns | 252 ns |
| `dvn/synthesize_2s_2khz_density` | 3.048 ms | 3.583 ms | 3.512 ms | 3.585 ms | 3.622 ms |
| `gfpe/solve_100m_30m_500hz` | 3.290 ms | 3.826 ms | 3.784 ms | 3.871 ms | 3.941 ms |

**Finding — array builtins beat a scalar loop by far more than two lanes explain.** SSE2
doubles the width, but `f64v_add` is 5.5 times faster than a Cyrius loop, because it also
skips the compiler's general register handling. That is why the DWM solver gained 7×.

**Finding — AVX2 only pays on long rows.** `f64v256_*` is faster in isolation, but under
about 64 elements it loses in a solver, once each call's `vzeroupper` is paid: at 16³,
15.1 ms with it against 12.5 ms without. These benchmark grids have short rows, so they
run on 128-bit lanes. Rows of 64 and up get the 256-bit kernels, which cut run time by
15–34% on FDTD and DWM grids with 66 to 500 cells per row.

**Finding — the memory mattered more than the time.** Stack buffers took about 35% off
the ray paths, but a 100-ray diffuse trace went from 142,722 allocations to 383, and from
3.9 MB left behind to 21 KB. [ADR-0007](../adr/0007-stack-buffers-over-an-arena.md) has
the table.

**Finding — the three slower rows are accuracy the stdlib bought.** `f64_pow` doubled
(103 → 203 ns, 6.6.2 to 6.7.18) when ganita's general path, up to 909 ulp out, became
fdlibm `e_pow`, within 1 ulp (6.6.10). `ground_reflection` makes one call, and `dvn` one
per pulse through its decay envelope. `f64_sin` and `f64_cos` became fdlibm at 6.6.9,
which halves them below π/4 and costs about 10% on wide arguments (41 → 45 ns), and
`gfpe`'s convolution calls them with phases across thousands of radians. None of this
is a goonj regression, and none of it is worth undoing.

**Finding — the sort was the image-source bottleneck.** `early_reflections` sorted with
an insertion sort. The shared stable merge sort in 2.1.3 gives the same order and takes
63% off order 5. The rest of the 2.1.3 column is within this host's ±8% run-to-run noise,
so the bounds checks added to the band accessors cost nothing measurable.

## Binary size

The smoke binary is 2,136,824 bytes without dead-code elimination and 305,912 with it,
because it reaches about 14% of what it links. `[build] dce = true` has been on since
2.1.3. It does not change a consumer's build: they compile `dist/goonj.cyr` into their
own binary and choose for themselves.

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
