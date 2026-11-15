# 0009 — The BVH query allocates into a stack scratch, through hisab's scope

**Status**: Accepted
**Date**: 2026-11-15

## Context

[ADR-0007](0007-stack-buffers-over-an-arena.md) moved goonj's own temporaries onto the
stack and left two kinds of garbage that needed another project. One was the BVH query.
On every bounce, `_nearest_wall_bvh` called `geo_ray_new`, which allocates a `GeoRay` and
its unit direction, and `bvh_query_ray`, which allocates a result vec. That was 192 bytes
and 4 of the 6 allocations per bounce in `trace_ray_bvh`, never reclaimed. Measured on
2.3.1:

| Trace, 50 bounces max | Linear: bytes kept | BVH: bytes kept |
|---|---:|---:|
| Concrete shoebox, 36 bounces | 5,320 | 12,424 |
| 100-wall room, 50 bounces | 7,000 | 21,464 |

ADR-0007 rejected an arena because hisab called `alloc` directly, so nothing could catch
its results. hisab 3.4.2 added an allocation scope. `hisab_alloc_scope(al)` routes every
allocation hisab makes to the `Allocator` `al` until the next call, and returns the scope it
replaced. The roadmap gate, "a hisab query that fills a caller's vec", was a narrower form of
the same thing.

## Decision

**`_nearest_wall_bvh` hands hisab a bump allocator over a 4 KB buffer in its own frame,
for the length of the query.**

- The allocator is the stdlib's five-slot `Allocator` vtable, built on the stack. Its state
  is `{ next, end }` over `var buf: i64[512]`. It frees nothing and its reset does nothing:
  returning from the frame is the reset.
- The caller's scope is saved and restored around `geo_ray_new` and `bvh_query_ray`, and
  nothing else runs under goonj's. The candidates are read in the same frame, after the
  restore.
- A request that does not fit comes from `alloc`, as all of them did before. 4 KB holds the
  ray, its direction and a candidate vec of up to 128 entries. Past that the rest spills,
  and is kept, as in 2.3.1.
- Results do not move. The query returns the same candidates in the same order, and only
  where they live has changed.

## Consequences

- **Positive** — A BVH trace keeps what a linear trace keeps: 5,320 and 7,000 bytes in the
  two traces above. `tests/hardening.tcyr` pins it, and its two budget assertions fail
  against 2.3.1.
- **Positive** — No state outlives a call. There is no global arena for `alloc_reset()` to
  invalidate, nothing to share between threads, and nothing to size up front.
- **Negative** — About 2% slower: `ray/trace_bvh_100_walls` goes from 583 to 595 µs.
  Memory from a scope is zeroed by hisab (`_hsb_zalloc`), where fresh bump memory comes
  zeroed for free, and each allocation takes two more indirect calls.
- **Negative** — `--poison` cannot see the buffer, as with any stack buffer (ADR-0007). An
  overrun by hisab would land in this frame. hisab writes only within its own vec's
  capacity here.
- **Neutral** — A caller's own scope still holds the `HVec3`s that goonj keeps in its
  results, because goonj builds them with `hvec3_new`. A `trace_ray_bvh` under a caller's
  arena puts its 36 bounce points (864 bytes) there, as 2.3.1 did, and resetting that arena
  invalidates them. In 2.3.1 the arena also took the query's 3,024 bytes. Making goonj's
  results independent of a caller's scope would mean setting scope 0 at every public entry
  point. No consumer sets a scope today.
- **Neutral** — The other ADR-0007 leftover is unchanged. Output vectors still grow from
  empty by doubling, and that needs a presized vec from the stdlib.

## Alternatives considered

- **A global growable arena, reset per query** — rejected. Its header and chunks would come
  from `alloc`, so a consumer's `alloc_reset()` would leave goonj holding a pointer to
  memory that is handed out again. hisab keeps its own vtable in static storage for the
  same reason.
- **A static buffer** — rejected. It would add 4 KB of state to every consumer, and a
  re-entrant or threaded caller would share it.
- **Building the `GeoRay` and walking the BVH in goonj** — rejected. It means reading
  hisab's private layouts, which the next hisab release is free to change.
- **Waiting for a `bvh_query_ray_into`** — not needed. The scope covers it without a new
  hisab API, and would also cover `geo_ray_new`, which a fill-a-vec query would not.
