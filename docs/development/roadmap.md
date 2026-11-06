# goonj — Roadmap

> **Forward plan only.** Shipped work leaves this file: release history is in
> [`../../CHANGELOG.md`](../../CHANGELOG.md), live state in [`state.md`](state.md),
> per-module parity in [`port-audit.md`](port-audit.md), and the reasons behind
> decisions in [`../adr/`](../adr/).
>
> This file is the sequencing: what ships next, in what order, and what each step
> waits on.

## 2.1.3 — P(-1) audit, hardening and optimization sweep

**Next release.** Audit the whole library, repair what the audit finds, then take
the optimizations that change no result. It is the last sweep with `rust-old/` in
the tree, so it should lean on the oracle while it can.

A full sweep is justified by the record: 2.0.2 and 2.0.4 found 18 defects between
them, and 2.0.6 found two more with every assertion green. A passing suite has
repeatedly proved little here.

### Audit

| Area | What to check | Surface |
|------|---------------|---------|
| Hostile input | Call every public fn with NaN, ±Inf, ±0, subnormal, negative, huge, empty and 0-handle arguments. Each must give a correct or documented degenerate result, per `SECURITY.md`. | Every public fn |
| ADR-0002 rules | Both bounds at every index guard, every `f64_to` bounded in f64, every allocation-gating product bounded, NaN rejected explicitly. Start with raw memory access, which fails silently. | 342 lines of `load64` / `store64` |
| Allocation refusal | goonj does not check `alloc`. Since hisab 3.4.0 its `hvec3_*`, `geo_*` and `bvh_*` return 0 on refusal, and `ray.cyr` passes a 0 from `bvh_query_ray` to `vec_len`. Choose a policy, record it in an ADR, apply it. | 141 lines of `alloc` |
| NaN parity | `f64_max(NaN, 1)` is 1 but `f64_max(1, NaN)` is NaN, and `f64_min` matches it. Rust's `max` and `min` ignore NaN either way. 2.0.6 fixed room bounds only, so check each site against the oracle. | 119 `f64_min` / `f64_max` / `f64_clamp` sites |
| Fuzzing | `tests/goonj.fcyr` is a scaffold stub, so `cyrius fuzz` passes without running any goonj code. Write real targets and run them with `--poison`. | 0 real targets today |

### Known repairs

- **`fdn_new` on a non-finite or overflowing room dimension** still takes about 8 s
  and 9.3 GiB, and aborts where that memory is unavailable
  ([ADR-0003](../adr/0003-f64-to-saturates-keep-bounding-in-f64.md)). Removes the
  known limit in `SECURITY.md` if it can be closed without moving the cap.
- **`bvh_query_ray` returning 0**, under the allocation-refusal policy above.

### Optimization

Take only changes that keep results bit-identical. Allocation strategy waits for
2.2.x.

- **Refresh the baselines first.** `bench-history.csv` and
  [`../benchmarks/results.md`](../benchmarks/results.md) were last recorded on
  cyrius 6.5.35.
- **Explain the slower rows from 2.0.6:** `propagation/ground_reflection` (186 → 262
  ns), `dvn/synthesize_2s_2khz_density` (+15%) and `gfpe/solve_100m_30m_500hz` (+11%).
- **Binary size.** The smoke binary grew from 1.30 to 2.14 MB across 2.0.6–2.1.1, and
  the compiler reports about 86% of it unreachable. Evaluate `CYRIUS_DCE=1`.
- **`fdn_new`'s zero-fill `vec_push` loops**, which also bear on the first repair.

### Done when

- Every finding is fixed and pinned in `tests/hardening.tcyr`, or listed here with a
  repro.
- Anything that moves a result has an ADR.
- `cyrius audit` still exits 0, and CI gates it.
- `SECURITY.md` reflects the outcome.

## 2.2.x — Retire the oracle, then allocation

### 2.2.0 — Retire `rust-old/`

Waits for 2.1.3, which is its last planned use. Deleting the tree removes 49 tracked
files and 3.5 GB of untracked `target/`. `CLAUDE.md`, `CONTRIBUTING.md` and the README
all name it as the parity oracle, so those rules change in the same release. Re-verify
first that build, suites and the bundle check pass without it.

Its benchmark baseline is already archived to
[`../benchmarks/rust-bench-history.csv`](../benchmarks/rust-bench-history.csv), and its
only runnable demo is ported to [`../../examples/basic.cyr`](../../examples/basic.cyr).
The source stays readable from git tags.

### Allocation strategy

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
