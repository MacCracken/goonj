# Benchmarks: Rust → Cyrius

A like-for-like comparison of goonj's full micro-benchmark suite before and after
the v2.0.0 port from Rust to [Cyrius](https://github.com/MacCracken/cyrius). Every
Cyrius `.bcyr` benchmark replicates the *same operation, same inputs* as the Rust
`benches/benchmarks.rs` — 34 benchmarks across 15 modules.

> **The Cyrius side is an unoptimized parity port.** The gap has three roughly
> co-equal causes, and this is deliberately an honest first pass so it points at
> what to do next:
> 1. **The Cyrius code isn't tuned** — a faithful 1:1 translation with naive
>    per-call `alloc()`, no arena/pooling, no SIMD, no algorithmic passes.
> 2. **f32 → f64 widening** — forced by hisab's f64-only `HVec3`; double-width
>    arithmetic + 2× memory traffic. A precision upgrade, not pure overhead.
> 3. **A young compiler** — Cyrius is self-hosting, assembly-up, no LLVM.
>
> Cyrius is adding **more diverse float/integer types + related SIMD** (for model
> training/inference); once those land, goonj's hot paths pick appropriate widths
> and vectorize — see the [post-2.0.0 backlog](development/roadmap.md#post-200-backlog).
> This table is the baseline that work is measured against.

- **Rust** — `rust-old/bench-history.csv`, final pre-port run (ns/iter, f32).
- **Cyrius** — `lib/bench.cyr` wall-clock **min** over N iterations, v2.0.0
  (f64), from `tests/*.bcyr`. `cyrius bench tests/<x>.bcyr`.

Both on the same x86_64 Linux host.

## The pattern

Three regimes, and the regime — not the module — sets the ratio:

| Regime | What it measures | Cyrius / Rust | Reading |
|--------|------------------|--------------:|---------|
| **Trivial scalar op** | one arithmetic call | (meaningless) | Rust folds to **0–12 ns**; Cyrius hits a **~0.5 µs bench-harness floor**. Neither measures the op. |
| **Compute-bound loop** | sustained arithmetic, little alloc | **~6–18×** | The **honest gap** — codegen maturity + f64 width + no SIMD. |
| **Allocation-heavy** | per-call / per-bounce / per-ray `alloc()` | **~22–130×** | Dominated by the bump allocator, **not the logic**. |

The number to take seriously is the **compute-bound ~6–18×**. The alloc-heavy
gaps shrink with an arena allocator; the trivial ratios vanish with a bench
harness that can resolve sub-µs work.

## Compute-bound (the honest gap: ~6–18×)

| Benchmark | Rust | Cyrius (min) | ratio |
|-----------|-----:|-------------:|------:|
| `resonance/all_axial_modes_200hz` | 368 ns  | 2.31 µs  | ~6× |
| `analysis/d50`                    | 36.0 µs | 296.6 µs | ~8× |
| `analysis/c80`                    | 33.9 µs | 283.9 µs | ~8× |
| `gfpe/solve_100m_30m_500hz`       | 379 µs  | 3.48 ms  | ~9× |
| `metamaterial/absorption_bands`   | 226 ns  | 2.30 µs  | ~10× |
| **`dwm/solve_30x25x20_50ms`**     | 74.2 ms | 1.070 s  | **~14×** |
| `analysis/sti`                    | 3.67 ms | 59.8 ms  | ~16× |
| `fdtd/solve_40x40_50ms`           | 1.17 ms | 19.4 ms  | ~17× |
| `image_source/shoebox_order_3`    | 4.64 µs | 79.6 µs  | ~17× |
| `impulse/energy_decay_curve`      | 206 µs  | 3.70 ms  | ~18× |

`dwm/solve_30x25x20_50ms` (identical config both eras — 15k cells, 1102 steps)
is the exemplar: a tight `f64` triple loop over raw `load64`/`store64` buffers,
**no per-iteration allocation**. Its ~14× is the cleanest codegen+precision gap
in the whole suite, and it is the direct target of the number-type + SIMD work.

## Allocation-heavy (~22–130× — allocator, not logic)

| Benchmark | Rust | Cyrius (min) | ratio |
|-----------|-----:|-------------:|------:|
| `wav/export_48k_2s`               | 97.0 µs | 2.10 ms  | ~22× |
| `dvn/synthesize_2s_2khz_density`  | 169 µs  | 3.73 ms  | ~22× |
| `propagation/atmospheric_ray_1000_steps` | 15.7 µs | 363.9 µs | ~23× |
| `dwm/dispersion_correct_22kHz_1s` | 11.4 µs | 317.6 µs | ~28× |
| `image_source/shoebox_order_5`    | 15.9 µs | 482.6 µs | ~30× |
| `diffraction/is_occluded_shoebox` | 30 ns   | 1.47 µs  | ~49× |
| `ray/trace_single_shoebox`        | 2.58 µs | 128.4 µs | ~50× |
| `ray/trace_bvh_100_walls`         | 75.6 µs | 3.81 ms  | ~50× |
| `diffuse/1000_rays_shoebox`       | 2.46 ms | 131.5 ms | ~54× |
| `impulse/generate_ir_shoebox`     | 2.51 ms | 136.2 ms | ~54× |
| `ray/trace_100_rays_shoebox`      | 221 µs  | 12.58 ms | ~57× |
| `binaural/generate_ir_shoebox`    | 7.0 µs  | 495.7 µs | ~71× |
| `ray/trace_linear_100_walls`      | 48.1 µs | 3.49 ms  | ~73× |
| `analysis/suggest_absorption`     | 148 ns  | 11.8 µs  | ~80× |
| `ray/wall_intersection`           | 7 ns    | 907 ns   | ~130× |

Every one of these allocates per unit of work — a fresh `HVec3` per intersection,
a `RayBounce` + multiband-ray clone per bounce, a result vec per ray. Against a
bump arena that never frees within a run, that allocation *is* the cost. An
arena/pool allocator or out-param returns (backlog item) collapses most of it.
BVH-vs-linear at 100 walls is ~even in **both** languages (Rust 76 vs 48 µs;
Cyrius 3.81 vs 3.49 ms) — the partition geometry doesn't cull, so the crossover
is past 100 walls regardless of language.

## Trivial ops (Cyrius bench floor ~0.4–0.5 µs)

Rust's criterion folds these near-away; Cyrius's fixed-loop harness can't resolve
below its per-call overhead. **Ratios here are noise, not signal.**

| Benchmark | Rust | Cyrius (min) |
|-----------|-----:|-------------:|
| `propagation/speed_of_sound`    | 0 ns  | 419 ns |
| `propagation/inverse_square_law`| 0 ns  | 489 ns |
| `impulse/sabine_rt60`           | 0 ns  | 489 ns |
| `propagation/doppler_shift`     | 1 ns  | 488 ns |
| `resonance/room_mode`           | 1 ns  | 489 ns |
| `resonance/schroeder_frequency` | 2 ns  | 489 ns |
| `impulse/eyring_rt60`           | 6 ns  | 488 ns |
| `diffraction/edge_loss`         | 10 ns | 489 ns |
| `propagation/ground_reflection` | 12 ns | 489 ns |

That they all land on ~489 ns confirms it's the harness floor, not the work.

## Methodology

1. **Mean vs min.** Rust = criterion mean (warm-up, outlier rejection). Cyrius =
   fixed N-loop min/avg/max (no warm-up). `min` is the fairest Cyrius per-op number.
2. **~0.5 µs Cyrius bench floor.** `bench_run` pays a `fncall0` + clock pair per
   iteration; anything faster than that reads as ~0.4–0.5 µs.
3. **The bump allocator never frees.** Per-iteration `alloc()` accumulates within
   a run (hence `avg > min` on alloc-heavy ops) and dominates their absolute time.
4. **f32 → f64 widening** — real 2× memory traffic + wider arithmetic on every op.
5. **Resolution.** Sub-µs figures are ±1 bucket; trust the ms/µs mins.

## What this means

The port traded raw speed for **sovereignty, parity, and a single toolchain**
(the whole stack — goonj + hisab + sakshi — is Cyrius, no non-Cyrius deps). See
[ADR 0001](adr/0001-rust-to-cyrius-port-conventions.md).

- **The real gap is ~6–18×, and it's the honest number.** Every benchmark that
  measures sustained arithmetic sits there. It is the direct target of the
  [post-2.0.0 backlog](development/roadmap.md#post-200-backlog): when Cyrius ships
  diverse number types + SIMD, these loops pick widths and vectorize.
- **The scary 50–130× numbers are allocation, not logic.** They shrink with an
  arena/pool allocator — independent of the compiler, doable sooner.
- **Correctness held at parity.** 3585 assertions across 36 suites reproduce the
  Rust oracle's behavior (`docs/development/state.md`); these measure the same
  work, not a reduced surface.

Where Cyrius *wins* — a downstream Cyrius consumer folding `dist/goonj.cyr`
directly, no Rust↔Cyrius FFI, no second toolchain — isn't captured here. For
goonj's profile (consumers generate impulse responses offline, then convolve),
the current gap is affordable, and the path to closing it is already scoped.

---

_Regenerate the Cyrius numbers: `cyrius bench tests/<module>.bcyr` (15 files:
propagation, resonance, impulse, image_source, diffuse, analysis, wav, binaural,
dark_velvet_noise, diffraction, fdtd, gfpe, metamaterial, ray, dwm). Rust
baseline: `rust-old/bench-history.csv`._
