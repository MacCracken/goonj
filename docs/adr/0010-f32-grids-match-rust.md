# 0010 — The grid solvers run in f32, as Rust's did

**Status**: Accepted
**Date**: 2026-11-15

## Context

[ADR-0001](0001-rust-to-cyrius-port-conventions.md) made the port f64 everywhere, because
hisab's `HVec3` is f64-only. The three grid solvers, `solve_fdtd_2d`, `solve_dwm_3d` and
`solve_diffusion_2d`, never touch an `HVec3`. For them f64 was a choice, and it cost two
things.

**It was not Rust's answer.** A comparison harness generated 52 cases across the three
solvers: odd and even rows, rows long enough for the 256-bit kernels, CFL and dx-tolerance
edges, explicit and Gaussian sources, and receivers at corners and out of range. It ran
them through `rust-old` at tag 2.1.3 and through 2.3.1. 2.3.1's results, rounded to f32,
differed from Rust's in 33,319 of 40,224 values. FDTD's differed by up to 414,606 ULPs, and
one case at the CFL limit ran in Rust and was refused by 2.3.1. 2.3.1 had also
reassociated two sums: DWM added its six incoming waves as `((xp+xn) + (yp+yn)) + (zp+zn)`,
and diffusion added its four neighbours as `(left+right) + (below+above)`. Rust adds both
left to right.

**It was half the lanes and twice the memory.** [ADR-0008](0008-row-kernels-for-grid-solvers.md)
put the grids on `f64v_*` row kernels. `f32v_*` takes four lanes where `f64v_*` takes two,
and an f32 cell is half the bytes.

## Decision

**The three grid solvers hold their fields in f32 and compute in f32, in Rust's order, from
Rust's f32 inputs.**

- **Inputs are rounded to f32 on the way in.** These are the config fields Rust typed `f32`
  (dx, speed of sound, duration, dt, mean free path, max time), each source sample, the
  diffusion absorption and source fields, and the wall materials' absorption in the DWM
  filter fit. Every guard and step count is then decided on those f32 values. A
  configuration Rust refused is refused here, and the step count is Rust's.
- **Every operation is one IEEE f32 operation, in Rust's order.** `src/error.cyr` adds
  `_row32_add`, `_row32_sub` and `_row32_mul`, exact-length wrappers over `f32v_*` (4 lanes)
  and `f32v8_*` (8 lanes, with AVX2), on ADR-0008's pattern. They need wrappers for the same
  reason: the builtins round `n` up, and `f32v_add(out, a, b, 5)` writes eight. There is no
  packed scale, so a scalar operand is a row filled with it. Scalars are typed `f32` locals.
  Nothing is fused, because Rust doesn't contract `a * b + c`. Both reassociated sums are put
  back in Rust's order.
- **Interiors run as spans of whole rows,** not a row at a time. A span from (1, y0) to
  (nx − 2, y1) also takes in the wall cells between its rows, with the wrong neighbours.
  FDTD's wall copies and DWM's end-node pass overwrite every one of them, as Rust's do.
  Diffusion copies back only each row's interior, so its boundary keeps what Rust's keeps.
  Spans are cut at about 1,024 elements (`ROW32_CHUNK`), which keeps a chunk's operands in
  L1 and the scratch to a few KB. DWM's z walls keep their filter state as a plane, so a
  wall plane updates it in one pass. Every kernel is elementwise, so none of this can move a
  result.
- **Outputs keep their types.** Results are still vecs of f64, each the exact widening of an
  f32. That includes a result's `dt` and `dx` and a `BoundaryFilter`'s `b0` and `a1`.
- **Transcendentals.** `sqrt` in the filter fit is f64's, rounded to f32. That is correctly
  rounded, so it equals Rust's `sqrtf`. `exp` in the Gaussian pulse is f64's, rounded to
  f32. That was correctly rounded on all of 300,000 sampled arguments. glibc's `expf`, which
  Rust calls on Linux, is not on 162 of them. goonj keeps the correctly rounded value, which
  is one ULP from Rust's where they differ.
- **Out of scope.** `band_energies`, `mesh_frequency`, `dispersion_factor`,
  `DispersionCorrection` and `required_dx` stay f64. They process a result, they are not a
  grid. The ray paths stay f64 until hisab has an f32 vector.

## Consequences

- **Positive** — Rust's bits. The harness ran 24 generated sets, 1,248 cases and 951,482
  values on the default path, and every set again on the 128-bit path and on the 256-bit
  path from 8 elements. Every value is Rust 2.1.3's except in 2 cases, where Rust's `expf`
  rounded a Gaussian sample differently and the mesh carried it. The three paths always
  agree. `tests/hardening.tcyr`'s golden hashes for DWM, FDTD and diffusion are now Rust's:
  the same sweeps, run through `rust-old` at 2.1.3, give the same hashes. They match on
  aarch64 under qemu too.
- **Positive** — Faster, and less memory. See the table below.
- **Negative** — Every grid result moves. Values change at f32 precision, and long runs
  accumulate rounding as Rust's did. Where `duration / dt` falls near a whole number the step
  count can differ from 2.3.1's by one. A configuration at the CFL or dx limit can be
  accepted or refused differently from 2.3.1. In each case it now does what Rust did.
- **Negative** — Two sets of row kernels, and three solvers whose interiors are spans with a
  fix-up, not rows. A span relies on its fix-up covering every cell it gets wrong, and the bit
  tests catch a gap. An overrun can hide, though: a mutant `_row32_add` that rounds its length
  up moves the DWM hash, but not FDTD's or diffusion's, because there it lands on a cell that
  is overwritten anyway. That is ADR-0008's warning again. The new f32 row test fails that
  mutant on every path.
- **Neutral** — The comparison harness is kept, in `scripts/rust-parity/`. It extracts
  `rust-old` at tag 2.1.3 into `build/`, so it needs cargo and is not part of CI. The golden
  hashes are what CI checks. The f32 ray paths on the roadmap will need the same harness.

| Benchmark | 2.3.1 (f64) | 2.3.2 (f32) | Rust |
|---|---:|---:|---:|
| `solve_dwm_3d_30x25x20_50ms` | 162.3 ms | **85.5 ms** | 74.2 ms |
| `solve_dwm_3d_24cubed_0.01s` | 34.9 ms | **19.4 ms** | |
| `solve_dwm_3d_16cubed_0.01s` | 13.1 ms | **8.85 ms** | |
| `fdtd/solve_40x40_50ms` | 7.52 ms | **2.48 ms** | 1.17 ms |
| diffusion, 40×40, 0.05 s | 7.60 ms | **2.96 ms** | |
| diffusion, 200×150, 0.01 s | 24.6 ms | **12.1 ms** | |

Medians of three interleaved rounds, both trees built with cyrius 6.7.18 and pinned to one
core. The diffusion rows come from a scratch benchmark: there is no `.bcyr` for it, and Rust
had none. DWM goes from about 2.2× behind Rust to 1.15×, and FDTD from 6.4× to 2.1×. The
256-bit kernels now see long runs: a span is about 1,024 elements where a row was `nx − 2`.

| Bytes kept by one solve | 2.3.1 (f64) | 2.3.2 (f32) |
|---|---:|---:|
| `solve_dwm_3d`, 30×25×20, 110 steps | 1,852,552 | **1,102,152** |
| `solve_fdtd_2d`, 200×150, 110 steps | 1,247,864 | **904,696** |
| `solve_diffusion_2d`, 200×150, 100 steps | 1,291,576 | **904,224** |

What is left of those bytes is mostly the three f32 grids and the result vec, which still
grows from empty (the roadmap's presized-vec item).

## Alternatives considered

- **Keep f64** — rejected. It is half the lanes and twice the memory, and 83% of its
  values were not Rust's.
- **f32 storage with f64 arithmetic** — rejected. It halves memory, but it converts on every
  load and store and matches neither version's bits.
- **f32 in the f64 code's association** — rejected. It is less work, but those two sums were
  the whole difference between FDTD, which matched Rust at once, and DWM and diffusion.
- **Replicate glibc's `expf`** — rejected. Rust calls the platform's libm, so that would give
  Rust's bits on Linux only, and it would be wrong where glibc is.
- **`f32v_fmadd` / `f32v8_fma`** — rejected. They save an operation per multiply-add, but a
  fused result is rounded once where Rust's is rounded twice.
- **Pad the rows and let the kernels overrun** — rejected for the reason ADR-0008 gave.
