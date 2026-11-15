# goonj — Roadmap

> **Forward plan only.** Shipped work leaves this file: release history is in
> [`../../CHANGELOG.md`](../../CHANGELOG.md), live state in [`state.md`](state.md),
> per-module parity in [`port-audit.md`](port-audit.md), and the reasons behind
> decisions in [`../adr/`](../adr/).
>
> This file is the sequencing: what ships next, in what order, and what each step
> waits on.

## 2.3.x — Number types

2.3.0 took SIMD as far as it goes without moving a result
([ADR-0008](../adr/0008-row-kernels-for-grid-solvers.md)), and 2.3.2 put the grid solvers
in f32, where they give Rust's bits ([ADR-0010](../adr/0010-f32-grids-match-rust.md)).
What is left is the ray paths.

- **f32 on the ray paths.** **Gate: hisab's `HVec3` is f64-only** (still so at 3.5.0), so
  this needs hisab to move first. `scripts/rust-parity/` checks the grids against `rust-old`
  at tag 2.1.3, and would need ray cases added.

## 2.x.x — Unscheduled

### Remaining allocation

2.2.1 removed goonj's own temporaries
([ADR-0007](../adr/0007-stack-buffers-over-an-arena.md)), and 2.3.2 the BVH query's
([ADR-0009](../adr/0009-bvh-query-scratch-through-hisabs-scope.md)). What is left
belongs to another project:

- **Output vectors:** an IR band is filled by `vec_push` from empty, which leaves doubling
  garbage. **Gate: a stdlib presized vec.**

### New acoustics features

None scoped. Anything beyond the Rust surface waits for the sweeps above.

## Out of scope

- Re-architecting module boundaries. Refactor only when the code demands it
  (third-instance rule).
