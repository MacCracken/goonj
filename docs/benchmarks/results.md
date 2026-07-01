# goonj — Benchmark results

Measured with `cyrius bench tests/<suite>.bcyr` (toolchain 6.3.14, x86_64).
Cyrius figures are the **min** over N iterations (the fairest per-op number —
no warm-up); treat as relative, not absolute (machine-dependent). Rust baselines
and the full 34-benchmark cross-language table live in the
**[Rust → Cyrius comparison](../benchmarks-rust-vs-cyrius.md)**.

## ray (`tests/ray.bcyr`)

Two scenes, matching the Rust `ray/*` benchmarks config-for-config:
- **shoebox** — 6-wall concrete shoebox (10×8×3 m), source (5, 1.5, 4), 50-bounce traces.
- **100-wall room** — a 20×20×3 m shoebox plus 94 drywall partitions
  (`make_many_wall_room(94)`), ray from (10, 1.5, 10) along +X, 50 bounces.

| Benchmark | Cyrius (min) | Rust | ratio | iters |
|-----------|-------------:|-----:|------:|------:|
| `ray/wall_intersection`      | 907 ns   | 7 ns    | ~130× | 200,000 |
| `ray/trace_single_shoebox`   | 128.4 µs | 2.58 µs | ~50×  | 20,000 |
| `ray/trace_100_rays_shoebox` | 12.58 ms | 221 µs  | ~57×  | 200 |
| `ray/trace_linear_100_walls` | 3.49 ms  | 48.1 µs | ~73×  | 3,000 |
| `ray/trace_bvh_100_walls`    | 3.81 ms  | 75.6 µs | ~50×  | 3,000 |

**Finding — allocation, not logic.** These are the suite's alloc-heaviest ops:
`wall_intersection` allocates a fresh `HVec3` per call, and each trace allocates a
`RayBounce` + multiband-ray clone per bounce. Against a bump allocator that never
frees within a run, that allocation *is* the cost — hence the 50–130× gaps (vs the
honest ~6–18× on compute-bound loops). An arena/pool allocator collapses most of
it (post-2.0.0 backlog).

**Finding — BVH vs linear at 100 walls is ~even** (3.81 vs 3.49 ms), and it's
~even in Rust too (75.6 vs 48.1 µs). The 94 axis-aligned partitions don't cull, so
the BVH crossover is past 100 walls in *both* languages — not a Cyrius artifact.
Both paths produce identical bounce sequences (`bvh_trace_matches_linear_trace`).

## dwm (`tests/dwm.bcyr`)

`solve_dwm_3d` on a rigid box (~220 steps at 22,050 Hz for the 0.01 s configs),
plus the dispersion-correction FIR.

| Benchmark | Cyrius (min) | iters |
|-----------|-------------:|------:|
| `solve_dwm_3d_16cubed_0.01s` (4,096 cells) | 58.2 ms | 30 |
| `solve_dwm_3d_24cubed_0.01s` (13,824 cells) | 195.8 ms | 15 |
| `solve_dwm_3d_30x25x20_50ms` (15,000 cells, 1,102 steps) | 1.070 s | 5 |
| `dwm/dispersion_correct_22kHz_1s` | 317.6 µs | 2,000 |

`solve_dwm_3d_30x25x20_50ms` matches the Rust `dwm/solve_30x25x20_50ms` config
exactly (Rust 74.2 ms → **~14×**) — the cleanest codegen+precision gap in the
whole suite, a tight `f64` triple loop over raw `load64`/`store64` buffers with
**no per-iteration allocation**. It's the direct target of the number-type + SIMD
work. `dispersion_correct` is alloc-heavy by contrast (~28× vs Rust's 11.4 µs).

**Finding — near-linear in cell count.** 3.375× the cells (16³→24³) costs ~3.37×
the time (58.2→195.8 ms), consistent with the O(cells · steps) grid sweep. Each
node does 6 waveguide-port updates per step; the hot loop uses double-buffered
outgoing waves.

_Future: a `.bcyr` history CSV once more hot-path modules are benched._
