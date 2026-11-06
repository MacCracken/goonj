# 0005 — What hostile input should do, and where the port stays different

**Status**: Accepted
**Date**: 2026-11-06

## Context

The 2.1.3 sweep ran the Rust oracle and the port on the same 172,076 hostile cases
across 108 entry points, comparing outcome classes and values, and ran the public
API through a fuzz harness and a line-by-line read against `rust-old/`. Most of what
it found fell into four classes:

- **Negated comparisons.** ADR-0002 rule 4 covers a port of `!(a..=b).contains(x)`.
  The same inversion hides in any guard written as the opposite of Rust's test: Rust's
  `if x < EPS { skip }` became `if x >= EPS { body }`, and both comparisons are false
  for NaN, so NaN flipped sides. It was at 10 sites, including the nearest-wall search.
- **`f64_min` and `f64_max` with NaN second.** `f64_max(1, NaN)` is NaN, where Rust's
  `max` returns 1. A fold over `[5, NaN, 1]` even lost its peak: the NaN reset the
  accumulator, so samples were normalized by 1 and not 5.
- **Rust's array bounds.** `energy[i]` panics. The port's public accessors were raw
  `load64`, and `mbr_set_energy_band` was a wild write through the public API. There
  were 21 of them.
- **Rust's unsigned floor, beyond indices.** ADR-0002 covers index guards. Rates and
  counts lost theirs too: `sample_rate == 0` guards let a negative rate through.

The bar is still "matches what Rust did, diverge only with an ADR". Most Rust behaviour
here is arbitrary, but it is the bar, and matching it means there is one rule to apply.
Where Rust has no behaviour to match, because the input can't exist or Rust dies, this
ADR picks one.

## Decision

**Where Rust produces a result, match it, NaN included.** Where it can't produce one, the
port returns a degenerate result, and does not crash, hang or write out of bounds.

1. **Extend ADR-0002 rule 4 to every comparison.** A guard keeps Rust's comparison and
   tests it for `== 0`: `f64_lt(x, EPS) == 0`, not `f64_ge(x, EPS) == 1`.
2. **A min or max that can see NaN second uses `goonj_fmin` or `goonj_fmax`.** A constant
   second argument is fine as `f64_max(x, 0)`.
3. **A public index out of range reads 0 and writes nothing**, as Rust's own
   `absorption_at_band` does. Rust panics in the others.
4. **A negative rate or count acts as Rust's `u32` floor**: the same result as 0.

The recorded differences from Rust:

| Input | Rust | Port | Why |
|---|---|---|---|
| Band index out of range | panics | 0 | Rule 3. A panic is not a contract. |
| `sample_rate` < 0 or > `u32::MAX` in a WAV | can't exist | `ERR_COMPUTATION` | A 32-bit field would store a different rate: -48000 becomes 4294919296 Hz. |
| Byte rate at ≥ 2²⁸ Hz (mono) or ≥ 2²⁷ Hz (stereo) | wraps to a wrong value | correct value | Rust's is overflow, and it panics in debug. |
| A wall with no vertices in `compute_image_sources_general` | panics | wall skipped | Same as rule 3. |
| `axial_modes` past 1,000,000 modes | no cap; a `u32` counter wraps after 2³² pushes | stops at 1,000,000 | Rust exhausts memory first. 1,000,000 modes is an 8.6 km dimension at 20 kHz. |
| `axial_modes` with a NaN argument | pushes NaN until out of memory | empty | Same. |
| `trace_ray_bvh` with a NaN or non-finite ray | NaN slabs are skipped, so bounces are NaN or none | no bounces | hisab's Cyrius BVH reports no leaf on a NaN slab, and `Inf` differs too. `trace_ray` still matches Rust. |
| `new_hoa_ir` order of 65,535 or more | `u32` overflow wraps to a small channel count | allocates what was asked for, until `vec` aborts | Rust's result is overflow, and it panics in debug. This is resource use the caller chose. |
| Sort order among NaN keys | driftsort's order | merge order | Unspecified in Rust, and `partial_cmp` makes the keys non-transitive. |
| `fdn_new` line past 115.2M samples | up to `u32::MAX` | capped | [ADR-0003](0003-f64-to-saturates-keep-bounding-in-f64.md), which now covers hand-built configs too. |
| A 0 handle | can't exist | faults at the null page | Fail-stop. Checking every handle in every function buys nothing a caller can use. |

**Not divergences, though the differential flags them.** Rust computes in f32 and the
port in f64, which shows up as:

- A value past f32's range goes to `Inf` or `0` in Rust only.
- `sin(2π)` is about 1e-7 in f32 and about 1e-16 in f64, so a tiny sign can differ.
- Exact-corner ties in a box resolve differently, because the f32 `EPSILON` guard is
  kept in f64 arithmetic. On geometry that does not hit a corner exactly, a 50-bounce
  `trace_beam` visits the same wall sequence.

## Consequences

- **Positive** — The next reader can apply rules 1 to 4 without the oracle, which is
  deleted in 2.2.0. `tests/hardening.tcyr` pins each class.
- **Positive** — Each band accessor gains two comparisons on valid input, and the rest
  of the results do not move: seven whole pipelines are bit-identical to 2.1.2.
- **Negative** — Matching Rust means NaN now reaches more outputs than it did. A NaN
  listener gives NaN reflections, where 2.1.2's accidental filter gave none. A consumer
  forwarding untrusted coordinates should still reject non-finite ones first.
- **Negative** — `trace_ray` and `trace_ray_bvh` no longer agree on a non-finite ray.
- **Neutral** — `apply_axis_attenuation` is private in Rust and was public by name here.
  It is `_apply_axis_attenuation` now. Its callers pass constants, so the rename is what
  closes its unchecked indices.

## Alternatives considered

- **Reject NaN at every public entry** — rejected. It is a saner API, but it is a new
  one, and the port's bar is parity. A consumer can do this in one place.
- **Make out-of-range indices abort, like `vec_get`** — rejected. SECURITY.md asks for an
  error or a degenerate result, and `absorption_at_band` already set the pattern.
- **Match Rust's BVH on NaN by re-testing each wall** — tried, then removed. A linear
  fallback matched NaN rays but not `Inf` origins, which Rust prunes. Matching both needs
  a second copy of `ray_aabb` in the hot path, for rays that are already garbage.
- **Fix `f64_max` in the stdlib** — out of scope for goonj, and only part of the class.
  It is worth reporting upstream.
