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

_Future: add a many-wall (>50) scene to demonstrate the BVH crossover, and a
`.bcyr` history CSV once more hot-path modules (dwm, image_source) are benched._
