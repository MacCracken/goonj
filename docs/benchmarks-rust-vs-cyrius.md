# Benchmarks: Rust → Cyrius

A like-for-like comparison of goonj's micro-benchmark suite against the Rust engine it
was ported from. Every Cyrius `.bcyr` benchmark replicates the *same operation, same
inputs* as the Rust `benches/benchmarks.rs`: 34 benchmarks across 15 modules.

- **Rust** — [`benchmarks/rust-bench-history.csv`](benchmarks/rust-bench-history.csv),
  the final pre-port run (ns/iter, **f32**). Frozen. See *Provenance* below.
- **Cyrius v2.0.1** — the unoptimized port, toolchain 6.5.35, f64.
- **Cyrius v2.3.0** — toolchain 6.7.18, f64.

Both on the same x86_64 Linux host. Ratios compare v2.3.0 with Rust. The speedups didn't
come from computing something else: 172,180 compared outputs are identical between 2.2.1
and 2.3.0, and 2.2.0 and 2.2.1 agree the same way.

> **The Cyrius side is f64 and Rust's is f32.** hisab's `HVec3` is f64-only, so that is
> twice the memory traffic on every operation. It's a precision upgrade, not pure overhead,
> and it sits inside every ratio below.

## Provenance of the Rust column

Every Rust figure in this document is the **final run** recorded in
[`benchmarks/rust-bench-history.csv`](benchmarks/rust-bench-history.csv): 841 `cargo bench`
rows across several runs, of which the last covers all 34 benchmarks. Spot-check:
`dwm/solve_30x25x20_50ms` 74,249,133 ns → 74.2 ms; `analysis/sti` 3,668,267 ns → 3.67 ms;
`fdtd/solve_40x40_50ms` 1,166,514 ns → 1.17 ms.

That file was archived out of `rust-old/bench-history.csv` when the Rust tree was retired
at 2.2.0. The benchmarks can be re-run only by restoring the source from tag `2.1.3`
([ADR-0006](adr/0006-retire-rust-old-keep-its-bar.md)), so treat it as a permanent record.
The Cyrius column is reproducible at any time with `scripts/bench-history.sh`.

## The pattern

The kind of work, not the module, sets the ratio:

| Kind of work | Cyrius / Rust | What decides it |
|--------------|--------------:|-----------------|
| **Array arithmetic** | **~1.0–6.0×** | Runs in the `f64v_*` builtins, close to what the hardware can do with f64. |
| **Ray and geometry** | **~7–14×** | One vector at a time, through function calls, with temporaries already removed. |
| **Scalar loops** | **~5–42×** | The compiler's scalar code, about 2.5 ns per element, and vec bounds checks. |

The band that used to be the honest gap, ~6–17×, split once SIMD arrived. Grid solvers
left it entirely. What remains in it is work that can't be vectorized without moving a
result: sums, recursive filters and calls to `sin` and `cos`.

## Array arithmetic

| Benchmark | Rust | v2.0.1 | v2.3.0 | ratio |
|-----------|-----:|-------:|-------:|------:|
| `dwm/dispersion_correct_22kHz_1s` | 11.4 µs | 311.1 µs | 10.98 µs | ~1.0× |
| `dwm/solve_30x25x20_50ms` | 74.2 ms | 1.052 s | 154.8 ms | ~2.1× |
| `fdtd/solve_40x40_50ms` | 1.17 ms | 18.67 ms | 6.99 ms | ~6.0× |

Each row of a grid is now a run of array ops
([ADR-0008](adr/0008-row-kernels-for-grid-solvers.md)). `dwm/solve_30x25x20_50ms` is the
exemplar: same grid in all three columns, 15,000 cells and 1,102 steps, and about 2× behind
Rust while doing f64 where Rust did f32. `dispersion_correct` runs on the 256-bit kernels
and now edges past Rust's recorded figure. These grids have short rows, which stay on
128-bit lanes: larger grids run 15–34% faster again on AVX2.

## Ray and geometry

| Benchmark | Rust | v2.0.1 | v2.3.0 | ratio |
|-----------|-----:|-------:|-------:|------:|
| `ray/trace_bvh_100_walls` | 75.6 µs | 1.811 ms | 560.9 µs | ~7.4× |
| `ray/trace_linear_100_walls` | 48.1 µs | 1.493 ms | 354.7 µs | ~7.4× |
| `propagation/atmospheric_ray_1000_steps` | 15.7 µs | 177.7 µs | 115.7 µs | ~7.4× |
| `image_source/shoebox_order_3` | 4.64 µs | 57.09 µs | 38.03 µs | ~8.2× |
| `diffuse/1000_rays_shoebox` | 2.46 ms | 57.41 ms | 20.77 ms | ~8.4× |
| `ray/trace_100_rays_shoebox` | 221 µs | 5.348 ms | 2.04 ms | ~9.2× |
| `impulse/generate_ir_shoebox` | 2.51 ms | 59.80 ms | 23.24 ms | ~9.3× |
| `ray/trace_single_shoebox` | 2.58 µs | 53.53 µs | 25.42 µs | ~9.9× |
| `image_source/shoebox_order_5` | 15.9 µs | 399.1 µs | 154.4 µs | ~9.7× |
| `diffraction/is_occluded_shoebox` | 30 ns | 796 ns | 363 ns | ~12× |
| `ray/wall_intersection` | 7 ns | 215 ns | 99 ns | ~14× |

These allocated on every operation until 2.2.1
([ADR-0007](adr/0007-stack-buffers-over-an-arena.md)). What remains is call overhead per
vector op. At 100 walls the BVH is 58% behind linear search here and 57% in Rust: the
partitions don't cull, so the crossover is past 100 walls in both languages. goonj's BVH
query also still allocates inside hisab.

## Scalar loops

| Benchmark | Rust | v2.0.1 | v2.3.0 | ratio |
|-----------|-----:|-------:|-------:|------:|
| `resonance/all_axial_modes_200hz` | 368 ns | 2.033 µs | 1.96 µs | ~5.3× |
| `metamaterial/absorption_bands_negstiff` | 226 ns | 1.695 µs | 1.40 µs | ~6.2× |
| `analysis/d50` | 36.0 µs | 260.6 µs | 270.9 µs | ~7.5× |
| `analysis/c80` | 33.9 µs | 247.2 µs | 258.8 µs | ~7.6× |
| `gfpe/solve_100m_30m_500hz` | 379 µs | 3.275 ms | 3.88 ms | ~10× |
| `analysis/sti` | 3.67 ms | 59.58 ms | 59.68 ms | ~16× |
| `impulse/energy_decay_curve` | 206 µs | 3.564 ms | 3.48 ms | ~17× |
| `wav/export_48k_2s` | 97.0 µs | 2.039 ms | 1.67 ms | ~17× |
| `dvn/synthesize_2s_2khz_density` | 169 µs | 3.024 ms | 3.59 ms | ~21× |
| `analysis/suggest_absorption` | 148 ns | 6.452 µs | 4.63 µs | ~31× |
| `binaural/generate_ir_shoebox` | 7.0 µs | 301.4 µs | 295.5 µs | ~42× |

`sti` and `gfpe` spend their time in `sin` and `cos`, and the two filters are recursive.
The energy rows are sums, where a vector version would add in a different order and move
the result. `binaural/generate_ir_shoebox` and `analysis/suggest_absorption` are the two
outliers, and nobody has looked at why yet.

## Trivial ops

| Benchmark | Rust | v2.0.1 | v2.3.0 | ratio |
|-----------|-----:|-------:|-------:|------:|
| `propagation/speed_of_sound` | 0 ns | 5 ns | 6 ns | — |
| `impulse/sabine_rt60` | 0 ns | 9 ns | 8 ns | — |
| `propagation/inverse_square_law` | 0 ns | 10 ns | 8 ns | — |
| `propagation/doppler_shift` | 1 ns | 9 ns | 9 ns | ~9.0× |
| `resonance/room_mode` | 1 ns | 11 ns | 8 ns | ~8.0× |
| `resonance/schroeder_frequency` | 2 ns | 14 ns | 9 ns | ~4.5× |
| `impulse/eyring_rt60` | 6 ns | 55 ns | 51 ns | ~8.5× |
| `diffraction/edge_loss` | 10 ns | 69 ns | 71 ns | ~7.1× |
| `propagation/ground_reflection` | 12 ns | 146 ns | 238 ns | ~20× |

Rows with a 0 ns Rust baseline have no meaningful ratio, because criterion folded them away.
At this scale a ratio is rough either way: these figures are only comparable within one
boot.

## Methodology

1. **Mean vs min.** Rust = criterion mean (warm-up, outlier rejection). Cyrius = min over
   the harness's per-chunk averages, net of a measured clock floor, with no warm-up.
2. **The clock floor is per-boot.** `lib/bench.cyr` documents ~400 ns to ~1,700 ns on this
   host, and it printed 1.21–1.35 µs for this run. Don't compare a row under ~10 µs across
   boots. `bench-history.csv` marks each row's comparability.
3. **The v2.0.1 column** is measured with the harness of cyrius 6.5.19 and later, so it is
   comparable with v2.3.0. It predates stdlib changes to `f64_pow`, `f64_sin` and `f64_cos`:
   see [`results.md`](benchmarks/results.md).
4. **Single-chunk rows carry no dispersion.** The heaviest rows report min == avg == max,
   because one auto-sized chunk covered every iteration.

## What this means

The port traded raw speed for **sovereignty, parity, and a single toolchain**: the whole
stack, goonj + hisab + sakshi, is Cyrius, with no non-Cyrius deps. See
[ADR 0001](adr/0001-rust-to-cyrius-port-conventions.md).

- **The wave solvers are no longer the bottleneck.** They were ~14–16× behind and are now
  ~2–6×, in f64, and faster still on the long rows of a real grid.
- **The ray paths are next-best placed.** They went from ~21–31× to ~7–14× by no longer
  allocating, and what remains is call overhead.
- **Correctness held at parity.** Neither step moved a result.

Where Cyrius *wins*, a downstream consumer folding `dist/goonj.cyr` directly with no FFI and
no second toolchain, isn't captured here.

---

_Regenerate the Cyrius numbers: `scripts/bench-history.sh`. Rust baseline:
[`benchmarks/rust-bench-history.csv`](benchmarks/rust-bench-history.csv)._
