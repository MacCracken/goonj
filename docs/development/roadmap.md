# goonj — Roadmap

> **Forward plan only.** Shipped work leaves this file: release history is in
> [`../../CHANGELOG.md`](../../CHANGELOG.md), live state in [`state.md`](state.md),
> per-module parity in [`port-audit.md`](port-audit.md), and the reasons behind
> decisions in [`../adr/`](../adr/).
>
> This file is the sequencing: what ships next, in what order, and what each step
> waits on.

## 2.2.x — Allocation strategy

Out-param returns or an arena/pool allocator for the alloc-heavy tiny ops:
`wall_intersection` and per-ray and per-reflection temporaries. This is the 11–44×
band in the [Rust comparison](../benchmarks-rust-vs-cyrius.md). Its earlier improvement
came from the stdlib allocator and the bench harness, not from goonj, so this work is
still fully open. hisab 3.4.0's stack-based `hvec3_distance` is a pattern to follow.

## 2.3.x — Number types and SIMD

The port is f64 throughout, with no SIMD. The original gate was Cyrius shipping these
features, and that has mostly cleared: `lib/simd.cyr` ships at 6.7.18, and hisab
already uses `f64v_*` inside `HVec3`.

- **SIMD on f64 inner loops:** the dwm solver, ray traversal and wall intersection.
  No gate.
- **Narrower integers** for grid indices, if measurement supports it. No gate.
- **f32 on hot paths.** **Gate: hisab's `HVec3` is f64-only**, so this needs hisab to
  move first.
- **Refresh [`../benchmarks-rust-vs-cyrius.md`](../benchmarks-rust-vs-cyrius.md)**
  once these land. It describes goonj's unoptimized first pass.

## 2.x.x — Unscheduled

### Consumer-green

At least one downstream consumer compiles and passes against `dist/goonj.cyr`. This is
not goonj's to close alone. Six repos declare `[deps.goonj]` at 2.0.4 or 2.0.5 (see
[`state.md`](state.md#consumers)), and nobody has confirmed that any of them compiles
and passes. Reaching 2.1.x needs each of them on cyrius ≥ 6.7.2 first. **Gate: dhvani,
shruti and kiran being ported to Cyrius.**

### New acoustics features

None scoped. Anything beyond the Rust surface waits for the sweeps above.

## Out of scope

- Re-architecting module boundaries. Refactor only when the code demands it
  (third-instance rule).
