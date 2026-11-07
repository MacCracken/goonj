# 0008 — Grid solvers run as row kernels over `f64v_*`

**Status**: Accepted
**Date**: 2026-11-06

## Context

A scalar Cyrius loop costs about 2.5 ns per element, because each operation goes through
the compiler's general register handling. The `f64v_*` builtins take a length and emit
a tight SSE2 loop (NEON on aarch64): 0.46 ns per element for `f64v_add`, 5.5 times
faster, with no CPU detection needed. `f64v256_*` does the same over 4 lanes, but needs
AVX2 and traps without it.

That favours whole arrays over element loops, and the grid solvers are almost entirely
arithmetic on arrays. But DWM stored its six ports per node, `out[node * 6 + k]`, so a
row of nodes wasn't a run of memory for any one port.

The builtins also have a trap. **They step 2 or 4 lanes at a time with no scalar tail, so
a length that isn't a multiple rounds up.** `f64v_add(out, a, b, 1)` writes two elements. The first version
of these kernels overran by one element on every odd row. Every suite passed, because
they compare within a tolerance. It was caught by comparing hashes against 2.2.1.

## Decision

**Grid interiors run a row at a time, through exact-length wrappers.**

- **`_row_add`, `_row_sub`, `_row_mul` and `_row_scale`** in `src/error.cyr` wrap the
  builtins, and finish any remainder with 2 lanes and then one scalar op. Nothing in
  `src/` outside them calls the builtins with a variable length. They don't rely on an
  overrun landing somewhere harmless, even where one would.
- **Rows of 64 or more use the 256-bit kernels when the CPU has AVX2**, through
  `simd_has_avx2`, which is why `simd` joined the stdlib list. Below that they lose: each
  call pays a `vzeroupper`. At 16³ the solver took 15.1 ms with them from 8 lanes up and
  12.5 ms without. The threshold comes from running the solvers, because a bare
  `f64v256_add` loop looks faster even at `n = 8`. Results don't depend on the path.
- **DWM keeps each port in its own array of `n`,** so ports are runs of memory. Walls
  update their filter state row by row, and the two end nodes of each row stay scalar,
  because they have an x wall.
- **FDTD, diffusion and dispersion correction** use the same wrappers. A vec's storage is
  reached through `_vec_data`, which relies on the layout `lib/vec.cyr` documents, and a
  test trips if that moves.
- **Every element gets the same IEEE operations, in the same order, as the scalar code,**
  so results are bit-identical. The one exception to "same order" is the operand order of
  a multiplication, which can't change a finite result.
- **Bits are tested, not values.** `tests/hardening.tcyr` hashes sweeps of every row
  parity and source class against hashes taken from 2.2.1, the last scalar solvers.

Sums over a whole signal stay scalar. `f64v_dot` adds in a different order, and floating
addition isn't associative, so it would move results.

**Ray traversal got no SIMD.** A single 3-vector is already one builtin inside `hvec3_dot`,
and vectorizing across walls would need a per-wall layout that consumers can't then edit.
What helped was logic: `_nearest_wall` skips the polygon test for a plane that can't beat
the best hit, and `point_in_convex_polygon` is spelled out as scalar expressions, which is
safe there because only the sign of the dot is read.

**Narrower integers are dropped.** The roadmap asked whether they would help if measured.
Nothing in goonj stores indices in bulk: grid indices are computed, never kept, so there
is nothing to narrow.

## Consequences

- **Positive** — Measured against 2.2.1, on the same scenes:

  | Benchmark | 2.2.1 | 2.3.0 | Rust |
  |---|---:|---:|---:|
  | `solve_dwm_3d_30x25x20_50ms` | 1.128 s | **154.8 ms** | 74.2 ms |
  | `fdtd/solve_40x40_50ms` | 19.40 ms | **6.99 ms** | 1.17 ms |
  | `dwm/dispersion_correct_22kHz_1s` | 327.8 µs | **10.98 µs** | 11.4 µs |
  | `ray/trace_linear_100_walls` | 673.1 µs | **354.7 µs** | 48.1 µs |

- **Positive** — The DWM solver goes from about 15× behind Rust to about 2×, and that is
  with f64 against Rust's f32. On grids with rows of 66 to 500, AVX2 cuts another 15–34%.
- **Positive** — Portable: the golden hashes are the same on x86_64 and on aarch64 under
  qemu, and on the 128-bit and 256-bit paths.
- **Negative** — The DWM solver is longer, and a stencil now reads as a list of array ops.
  Comments carry the formula beside each kernel.
- **Negative** — A row kernel has one more way to go wrong than a loop does, and a tolerance
  won't catch it. Anything that changes one must keep the hash tests, which run on each path.
- **Negative** — `simd` is a new stdlib leaf, which reaches consumers through
  `dist/goonj.deps`. `cyrius deps` picks it up, and it needs nothing else from them.

## Alternatives considered

- **Let odd rows overrun and pad every buffer** — rejected. It works only while each
  overrun lands somewhere harmless, and that has to stay true for every kernel from then
  on. The first version's hashes moved for exactly this reason.
- **`f64v4` wrappers** — rejected. `lib/simd.cyr` keeps them on 128-bit lanes, and they take
  4 elements at a time.
- **256-bit kernels on every row** — rejected on measurement, as above.
- **Vectorize `f64v_dot`-style sums** — rejected, because it moves results.
- **f32 grids** — not taken in 2.3.0, and still open. It would change every solver result,
  so it needs its own ADR. See the roadmap.
