# 0007 — Hot paths use stack buffers, not an arena

**Status**: Accepted
**Date**: 2026-11-06

## Context

The bump allocator never frees within a run, and goonj's hot paths built every
intermediate vector on the heap, because hisab's `hvec3_add`, `hvec3_sub`,
`hvec3_scale`, `hvec3_neg` and `hvec3_cross` each allocate their result. Measured on
2.2.0:

| Operation | Allocations | Bytes left behind |
|---|---:|---:|
| `ray_wall_intersection` (hit) | 15 | 360 |
| `trace_ray`, 36 bounces | 1,808 | 49,824 |
| `generate_diffuse_rain`, 100 rays | 142,722 | 3,925,040 |
| `generate_ir`, 8 kHz, 1 s | 92,834 | 3,645,024 |

So this was two problems, not one. It is the 11–44× band in the
[Rust comparison](../benchmarks-rust-vs-cyrius.md), and it is a leak: a long-lived
consumer lost 3.6 MB per impulse response and could never get it back. It also crashed.
`solve_radiosity` leaked in its form factor, and at long iteration counts it ran out of
memory and faulted where Rust completes.

Allocation is cheap here, so the gain was never going to be the whole of that band. The
memory was the larger part of the problem.

## Decision

**Hot paths work in buffers the caller owns, usually on its stack. Only what a result
keeps goes on the heap.**

- `src/error.cyr` has out-param forms of hisab's allocating ops: `_v3_add`, `_v3_sub`,
  `_v3_scale`, `_v3_neg`, `_v3_cross`. They use the same operations in the same order,
  so they give the same bits. This is the pattern of hisab's own `hvec3_distance`.
- **A `var v: i64[3]` must not outlive its function.** Anything a result keeps goes
  through `_v3_keep`. This is the one new way to get this wrong, so
  `tests/hardening.tcyr` reads results back after further calls have reused the stack.
- **Loops that make many of something take a visitor or reuse it.** `_shoebox_images`
  visits each image on the stack instead of building a vec to discard, and
  `generate_diffuse_rain` reuses one set of bounces across rays.
- **Public signatures and results do not change.** A public function that returns a
  vector still allocates that vector.
- **`tests/hardening.tcyr` pins the budgets**, so a temporary creeping back fails a test.

## Consequences

- **Positive** — The same operations now cost:

  | Operation | Allocations | Bytes left behind |
  |---|---:|---:|
  | `ray_wall_intersection` | 0 | 0 |
  | `trace_ray`, 36 bounces | 77 | 5,320 |
  | `generate_diffuse_rain`, 100 rays | 383 | 21,016 |
  | `generate_ir`, 8 kHz, 1 s | 1,104 | 1,115,672 |

  What `generate_ir` keeps now is nearly all the IR itself.
- **Positive** — Ray paths run about 35% faster, and `solve_radiosity` no longer leaks.
- **Positive** — Nothing moved. All 172,180 cases of the hostile and realistic comparison
  are unchanged from 2.2.0, apart from radiosity cases that used to run out of memory.
- **Negative** — A stack buffer is invisible to `--poison`, which only guards the heap.
  Overrunning one would corrupt the stack silently. The buffers are fixed-size and written
  field by field, so the exposure is small, but it is new.
- **Negative** — `hvec3_*` reads better than `_v3_*`. The hot paths are harder to read
  than they were, so this stays limited to them.
- **Neutral** — What is left needs another project:
  - `bvh_query_ray` and `geo_ray_new` allocate inside hisab, about 4 of the 6
    allocations per bounce in `trace_ray_bvh`. Reusing a result vec needs a hisab API.
  - Output vectors are filled by `vec_push` from empty, which is doubling garbage. A
    presized vec needs a stdlib API.

## Alternatives considered

- **An arena for temporaries** — rejected, though the roadmap named it. hisab calls
  `alloc` directly, so an arena cannot catch its results, and nearly all the garbage is
  hisab's. It would need hisab to take an allocator through every vector op.
- **`alloc_reset` between rays** — rejected. It invalidates every pointer in the process,
  including the caller's.
- **Rewriting hot paths in scalars** (`ox`, `oy`, `oz`) — rejected. It is more code and
  easier to get subtly wrong, and `hvec3_dot` and `hvec3_length` already read a stack
  `HVec3`.
- **Fixing `hvec3_*` in hisab** — not possible while they return a pointer: something has
  to own the result.
