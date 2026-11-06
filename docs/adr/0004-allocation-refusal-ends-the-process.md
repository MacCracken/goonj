# 0004 — A refused allocation ends the process

**Status**: Accepted
**Date**: 2026-11-06

## Context

goonj never checked `alloc`. Since hisab 3.4.0, hisab's `hvec3_*`, `geo_*` and
`bvh_*` also return 0 when an allocation is refused, and `vec_new` always has. The
port has 141 lines that call `alloc` and many more that call into hisab, so a policy
had to come first.

What a refusal did was measured with fault injection: a patched `alloc` refused the
Nth allocation, or every allocation from the Nth on, at sampled points across 16
pipelines. Each run was compared against an unrefused baseline.

| Outcome | Runs |
|---|---|
| Fail-stop: SIGSEGV at the null page, or `vec_push`'s abort | almost all |
| Clean completion, same result | a few (a refused allocation nothing needed) |
| **Silent wrong result** | one path, below |

Rust's answer is simple. Its allocator calls `handle_alloc_error`, which aborts, and
`Vec` can't return partially built. The stdlib's `vec_push` does the same.

The silent path was the BVH. hisab's 0 is overloaded: `bvh_build` returns it for an
empty room and for a refused build, and `bvh_query_ray` on a 0 root returns an empty
list. So a refused build produced a room in which every ray escaped, with no error.
`find_nearest_wall_bvh` had the same shape one level down: a refused `geo_ray_new`
read as a miss.

Two more things are not silent but matter:

- **A fixed-size struct always faults at the null page.** Its first store is at a small
  constant offset, on the line after `alloc`.
- **A variable-size buffer might not.** `alloc(n * 8)` refused, then
  `store64(buf + idx * 8, x)` with a large `idx` lands in the process image, not the
  null page. Every such site happens to write index 0 first, but that is luck, not a
  guarantee.

## Decision

**A refused allocation ends the process, as it does in Rust.** goonj has no result type
to carry it, and no caller can continue from half a room.

- **Sizes that depend on input go through `goonj_alloc`**, which aborts with
  `goonj: allocation refused` and exit 1: every grid, field and scratch buffer, the
  FDN delay lines, and the sort's scratch.
- **Fixed-size structs keep plain `alloc`.** Refusal is a fault at the null page. That
  is fail-stop already, and a check at each of about 130 sites would buy only a nicer
  message. The same goes for hisab's vectors: one 0 from `hvec3_add` faults at the next
  read, and wrapping every call is not a change worth making.
- **Any 0 that also means "empty" or "miss" is checked at the call:**
  `room_geometry_build_bvh` (a 0 for a non-empty wall list), and `geo_ray_new` and
  `bvh_query_ray` in `find_nearest_wall_bvh`.
- **The WAV writers keep their existing `ERR_COMPUTATION`.** Rust's writers return
  `Result`, so this is the one refusal the API can already report.

## Consequences

- **Positive** — Fault injection now finds no silent wrong result on any of the 16
  pipelines, in either refusal mode.
- **Positive** — Matches Rust and `vec_push`, so there is one behaviour to document.
- **Negative** — A consumer can't recover from exhaustion. Rust's could not either.
- **Negative** — Most refusals still end in SIGSEGV, not in the message. Both stop the
  process at the point of failure. Neither continues.
- **Neutral** — The fault-injection harness needs a patched `alloc`, so it lives outside
  the repo. Recheck this if a new 0 gains a second meaning.

## Alternatives considered

- **Wrap all 141 sites** — rejected. Most of the refusals in a real run come from
  hisab's vectors, so the wrapper would not change what most users see.
- **Return an error or an empty result** — rejected. It would touch every signature,
  Rust has no such path, and an empty result on exhaustion is precisely the failure the
  BVH had.
- **Return 0 and document it** — rejected. That is the status quo, and it is what let a
  room silently lose its walls.
