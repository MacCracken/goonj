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

- **Rust** — [`benchmarks/rust-bench-history.csv`](benchmarks/rust-bench-history.csv),
  final pre-port run (ns/iter, f32). Frozen. See *Provenance* below.
- **Cyrius v2.0.0** — `lib/bench.cyr` wall-clock **min**, toolchain **6.3.14**.
- **Cyrius v2.0.1** — same suites, toolchain **6.5.35**, idle host.

Both on the same x86_64 Linux host. `cyrius bench tests/<x>.bcyr`.

> ### ⚠ The two Cyrius columns come from different instruments
>
> Read the v2.0.0 → v2.0.1 movement as **two effects, not one**, and never as a
> clean speedup delta:
>
> 1. **A real allocator win.** Cyrius 6.3.17 gave `lib/alloc.cyr`'s lock a
>    single-threaded elision fast path (`_threads_active == 0` early-return),
>    removing a documented alloc tax. goonj is single-threaded, so every
>    allocation-heavy row benefits. This is genuine.
> 2. **A changed measuring instrument.** Cyrius 6.5.19 rewrote the vendored
>    harness: it now *measures* the per-clock-read floor and subtracts it from
>    every sample, and `bench_run` auto-sizes a batch under one clock pair
>    instead of wrapping a clock pair around each iteration. The v2.0.0 column
>    therefore carries roughly one clock read (~489 ns on the boot that produced
>    it) as additive per-iteration overhead; the v2.0.1 column carries ~none.
>
> **Do not diff the two columns for any row under ~10 µs** — below that scale you
> are mostly reading the retired instrument. The measured floor is also per-boot
> (`lib/bench.cyr` documents ~400 ns to ~1,700 ns on this host), so record the
> `[timer floor …]` line each run prints rather than assuming a constant. This
> run measured **1.33–1.35 µs**, stable to ±1% across all 16 suites.
>
> Ratios below are computed against the **v2.0.1** column and the frozen Rust
> baseline. Source is unchanged between the two Cyrius columns — `git diff -w`
> across `src/` and `tests/` is empty for 2.0.1 — so none of the movement is code.

## Provenance of the Rust column

Every Rust figure in this document is the **final run** recorded in
[`benchmarks/rust-bench-history.csv`](benchmarks/rust-bench-history.csv) — 841
`cargo bench` rows across several runs, of which the last covers all 34
benchmarks. Spot-check: `dwm/solve_30x25x20_50ms` 74,249,133 ns → 74.2 ms;
`analysis/sti` 3,668,267 ns → 3.67 ms; `fdtd/solve_40x40_50ms` 1,166,514 ns →
1.17 ms; `binaural/generate_ir_shoebox` 6,995 ns → 7.0 µs.

That file was archived out of `rust-old/bench-history.csv` when the frozen Rust
tree was retired. It is the **only** evidence for the Rust column — the
benchmarks cannot be re-run once `rust-old/` is gone (no `Cargo.toml`, no Rust
toolchain in this project), so treat it as a permanent record rather than a
regenerable artifact. The Cyrius column, by contrast, is reproducible at any
time with `scripts/bench-history.sh`.

## The pattern

Three regimes, and the regime — not the module — sets the ratio:

| Regime | What it measures | Cyrius / Rust | Reading |
|--------|------------------|--------------:|---------|
| **Trivial scalar op** | one arithmetic call | **~7–12×** | Now genuinely resolvable (5–146 ns). At v2.0.0 these were all buried under a ~489 ns harness floor. |
| **Compute-bound loop** | sustained arithmetic, little alloc | **~6–17×** | The **honest gap** — codegen maturity + f64 width + no SIMD. |
| **Allocation-heavy** | per-call / per-bounce / per-ray `alloc()` | **~11–44×** | Dominated by the bump allocator, **not the logic**. |

The number to take seriously is still the **compute-bound ~6–17×** — and it barely
moved across the toolchain bump, which is exactly what should happen to rows that
don't allocate and aren't floor-bound. The headline change is that the alloc-heavy
band roughly halved (from ~22–130×), and that trivial ops now land *inside* the
compute-bound band instead of being unmeasurable.

## Compute-bound (the honest gap: ~6–17×)

| Benchmark | Rust | Cyrius v2.0.0 | Cyrius v2.0.1 | ratio |
|-----------|-----:|--------------:|--------------:|------:|
| `resonance/all_axial_modes_200hz` | 368 ns  | 2.31 µs  | 2.033 µs  | ~5.5× |
| `analysis/d50`                    | 36.0 µs | 296.6 µs | 260.6 µs  | ~7.2× |
| `analysis/c80`                    | 33.9 µs | 283.9 µs | 247.2 µs  | ~7.3× |
| `metamaterial/absorption_bands_negstiff` | 226 ns | 2.30 µs | 1.695 µs | ~7.5× |
| `gfpe/solve_100m_30m_500hz`       | 379 µs  | 3.48 ms  | 3.275 ms  | ~8.6× |
| `image_source/shoebox_order_3`    | 4.64 µs | 79.6 µs  | 57.09 µs  | ~12× |
| **`dwm/solve_30x25x20_50ms`**     | 74.2 ms | 1.070 s  | 1.052 s   | **~14×** |
| `fdtd/solve_40x40_50ms`           | 1.17 ms | 19.4 ms  | 18.67 ms  | ~16× |
| `analysis/sti`                    | 3.67 ms | 59.8 ms  | 59.58 ms  | ~16× |
| `impulse/energy_decay_curve`      | 206 µs  | 3.70 ms  | 3.564 ms  | ~17× |

`dwm/solve_30x25x20_50ms` (identical grid all three eras — 15k cells, 1102 steps)
is the exemplar: a tight `f64` triple loop over raw `load64`/`store64` buffers,
**no per-iteration allocation**. Its ~14× is the cleanest codegen+precision gap
in the whole suite, and it is the direct target of the number-type + SIMD work.
It moved 1.070 s → 1.052 s (−1.7%) across the toolchain bump — i.e. not at all,
which is the point: nothing here allocates, so neither the allocator fast path
nor the floor subtraction has anything to grip.

## Allocation-heavy (~11–44× — allocator, not logic)

| Benchmark | Rust | Cyrius v2.0.0 | Cyrius v2.0.1 | ratio |
|-----------|-----:|--------------:|--------------:|------:|
| `propagation/atmospheric_ray_1000_steps` | 15.7 µs | 363.9 µs | 177.7 µs | ~11× |
| `dvn/synthesize_2s_2khz_density`  | 169 µs  | 3.73 ms  | 3.024 ms  | ~18× |
| `ray/trace_single_shoebox`        | 2.58 µs | 128.4 µs | 53.53 µs  | ~21× |
| `wav/export_48k_2s`               | 97.0 µs | 2.10 ms  | 2.039 ms  | ~21× |
| `diffuse/1000_rays_shoebox`       | 2.46 ms | 131.5 ms | 57.41 ms  | ~23× |
| `impulse/generate_ir_shoebox`     | 2.51 ms | 136.2 ms | 59.80 ms  | ~24× |
| `ray/trace_bvh_100_walls`         | 75.6 µs | 3.81 ms  | 1.811 ms  | ~24× |
| `ray/trace_100_rays_shoebox`      | 221 µs  | 12.58 ms | 5.348 ms  | ~24× |
| `image_source/shoebox_order_5`    | 15.9 µs | 482.6 µs | 399.1 µs  | ~25× |
| `diffraction/is_occluded_shoebox` | 30 ns   | 1.47 µs  | 796 ns    | ~27×† |
| `dwm/dispersion_correct_22kHz_1s` | 11.4 µs | 317.6 µs | 311.1 µs  | ~27× |
| `ray/wall_intersection`           | 7 ns    | 907 ns   | 215 ns    | ~31×† |
| `ray/trace_linear_100_walls`      | 48.1 µs | 3.49 ms  | 1.493 ms  | ~31× |
| `binaural/generate_ir_shoebox`    | 7.0 µs  | 495.7 µs | 301.4 µs  | ~43× |
| `analysis/suggest_absorption`     | 148 ns  | 11.8 µs  | 6.452 µs  | ~44×† |

† Sub-10 µs at v2.0.0 — its v2.0.0 figure is inflated by the retired harness
floor, so the apparent improvement is part instrument. The v2.0.1 ratio is sound;
the *delta* is not.

Every one of these allocates per unit of work — a fresh `HVec3` per intersection,
a `RayBounce` + multiband-ray clone per bounce, a result vec per ray. Against a
bump arena that never frees within a run, that allocation *is* the cost, which is
why removing a single allocator spinlock on the single-threaded path moved this
whole band and left the compute-bound table untouched. An arena/pool allocator or
out-param returns (backlog item) is expected to take more.

BVH-vs-linear at 100 walls remains ~even in **both** languages (Rust 76 vs 48 µs;
Cyrius 1.811 vs 1.493 ms) — the partition geometry doesn't cull, so the crossover
is past 100 walls regardless of language. BVH is now 21% behind linear, up from 9%.

## Trivial ops — the harness floor is gone

At v2.0.0 these nine all landed on ~419–489 ns, which was the harness, not the
work. The 6.5.19 harness resolves them, and they land at **~7–12×** — inside the
compute-bound band, which independently corroborates the compute-bound thesis.

| Benchmark | Rust | Cyrius v2.0.0 | Cyrius v2.0.1 | ratio |
|-----------|-----:|--------------:|--------------:|------:|
| `propagation/speed_of_sound`    | 0 ns  | 419 ns | 5 ns   | — |
| `impulse/sabine_rt60`           | 0 ns  | 489 ns | 9 ns   | — |
| `propagation/inverse_square_law`| 0 ns  | 489 ns | 10 ns  | — |
| `propagation/doppler_shift`     | 1 ns  | 488 ns | 9 ns‡  | ~9× |
| `resonance/room_mode`           | 1 ns  | 489 ns | 11 ns  | ~11× |
| `resonance/schroeder_frequency` | 2 ns  | 489 ns | 14 ns  | ~7× |
| `impulse/eyring_rt60`           | 6 ns  | 488 ns | 55 ns  | ~9× |
| `diffraction/edge_loss`         | 10 ns | 489 ns | 69 ns  | ~7× |
| `propagation/ground_reflection` | 12 ns | 489 ns | 146 ns | ~12× |

Rows with a 0 ns Rust baseline have no meaningful ratio — criterion folded them
away. ‡ `doppler_shift` reports min=0 ns at 1 ns resolution; its 9 ns *avg* is
used instead.

## Methodology

1. **Mean vs min.** Rust = criterion mean (warm-up, outlier rejection). Cyrius =
   min over the harness's samples, no warm-up.
2. **What "min" means changed at Cyrius 6.5.19.** v2.0.0: min over per-iteration
   samples, each carrying one clock-pair overhead. v2.0.1: min over *per-chunk
   averages*, net of a measured clock floor. Different statistic, different
   instrument — see the banner above.
3. **v2.0.0's ~0.4–0.5 µs floor** was real and load-bearing for reading that
   column: `bench_run` paid a `fncall0` + clock pair per iteration, so anything
   faster read as ~489 ns. That floor no longer exists at v2.0.1.
4. **The bump allocator never frees.** Per-iteration `alloc()` accumulates within
   a run (hence `avg > min` on alloc-heavy ops) and dominates their absolute time.
   Cyrius 6.3.17's single-threaded lock elision cut its constant, not its growth.
5. **f32 → f64 widening** — real 2× memory traffic + wider arithmetic on every op.
6. **Single-chunk rows carry no dispersion.** `diffuse/1000_rays_shoebox`,
   `impulse/generate_ir_shoebox`, `solve_dwm_3d_24cubed_0.01s` and
   `solve_dwm_3d_30x25x20_50ms` now report min == avg == max because one
   auto-sized chunk covered every iteration.

## What this means

The port traded raw speed for **sovereignty, parity, and a single toolchain**
(the whole stack — goonj + hisab + sakshi — is Cyrius, no non-Cyrius deps). See
[ADR 0001](adr/0001-rust-to-cyrius-port-conventions.md).

- **The real gap is ~6–17×, and it's the honest number.** Every benchmark that
  measures sustained arithmetic sits there, and it did not move across a
  five-minor toolchain bump. It is the direct target of the
  [post-2.0.0 backlog](development/roadmap.md#post-200-backlog): when Cyrius ships
  diverse number types + SIMD, these loops pick widths and vectorize.
- **The alloc-heavy band is ~11–44×, down from ~22–130×** — part real (the
  stdlib allocator's single-threaded fast path), part instrument (the harness now
  subtracts its own timer floor). It is still allocation, not logic, and an
  arena/pool allocator is expected to take more.
- **Correctness held at parity.** 3624 assertions across 37 suites reproduce the
  Rust oracle's behavior (`docs/development/state.md`); these measure the same
  work, not a reduced surface.

Where Cyrius *wins* — a downstream Cyrius consumer folding `dist/goonj.cyr`
directly, no Rust↔Cyrius FFI, no second toolchain — isn't captured here. For
goonj's profile (consumers generate impulse responses offline, then convolve),
the current gap is affordable, and the path to closing it is already scoped.

---

_Regenerate the Cyrius numbers: `cyrius bench tests/<module>.bcyr` (15 files:
propagation, resonance, impulse, image_source, diffuse, analysis, wav, binaural,
dark_velvet_noise, diffraction, fdtd, gfpe, metamaterial, ray, dwm). Record the
`[timer floor …]` line each run prints — it is per-boot and figures below ~10 µs
are only comparable within one boot. Rust baseline:
[`benchmarks/rust-bench-history.csv`](benchmarks/rust-bench-history.csv)._
