# goonj — Benchmark results

Measured with `cyrius bench tests/<suite>.bcyr` (toolchain 6.3.14, x86_64).
Numbers are per-op averages; treat as relative, not absolute (machine-dependent).

## ray (`tests/ray.bcyr`)

Scene: 6-wall concrete shoebox (10×8×3 m), 20-bounce trace from (5, 1.5, 4) along +Z.

| Benchmark | avg | min | iters |
|-----------|----:|----:|------:|
| `ray_wall_intersection` | 1.38 µs | 838 ns | 200,000 |
| `trace_ray_linear_20`   | 55.1 µs | 52.0 µs | 5,000 |
| `trace_ray_bvh_20`      | 79.6 µs | 75.4 µs | 5,000 |

**Finding:** for a 6-wall room the BVH path is *slower* than linear (79.6 vs
55.1 µs) — the `geo_ray_new` + BVH tree-traversal + candidate-vec allocation
overhead isn't amortized at low wall counts. This matches the Rust design note
("most beneficial for rooms with many walls, >20"). The BVH win appears as wall
count grows; the shoebox is the worst case for it. Both paths produce identical
bounce sequences (verified by `bvh_trace_matches_linear_trace`).

_Future: add a many-wall (>50) scene to demonstrate the BVH crossover._

## dwm (`tests/dwm.bcyr`)

Full `solve_dwm_3d` on a rigid box, 0.01 s (~220 steps at 22,050 Hz).

| Benchmark | avg | min | iters |
|-----------|----:|----:|------:|
| `solve_dwm_3d_16cubed` (4,096 cells) | 59.3 ms | 58.2 ms | 30 |
| `solve_dwm_3d_24cubed` (13,824 cells) | 199.7 ms | 195.8 ms | 15 |
| `solve_dwm_3d_30x25x20_50ms` (15,000 cells, 1102 steps) | 1.071 s | 1.065 s | 5 |

The last row matches the Rust `dwm/solve_30x25x20_50ms` config exactly — see the
**[Rust → Cyrius comparison](../benchmarks-rust-vs-cyrius.md)** (~14× on this
alloc-free hot loop; ~120× on the tiny alloc-heavy `wall_intersection`).

**Finding:** cost scales near-linearly in cell count — 3.375× the cells (16³→24³)
costs 3.35× the time — consistent with the O(cells · steps) grid sweep. Each node
does 6 waveguide-port updates per step; the hot loop uses raw `load64`/`store64`
buffers with double-buffered outgoing waves.

_Future: a `.bcyr` history CSV once more hot-path modules are benched._
