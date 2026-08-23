# Goonj

**Goonj** (गूँज — Hindi/Urdu for "echo, resonance") — acoustics engine for the
[AGNOS](https://github.com/MacCracken/agnosticos) ecosystem.

Goonj is a **[Cyrius](https://github.com/MacCracken/cyrius) library** — the
completed Rust → Cyrius port (**v2.0.4**) of the original ~14,630-line Rust engine
(frozen at `rust-old/` as a parity oracle). It is compiled by `cycc` (toolchain
pinned `cyrius = "6.5.35"` in `cyrius.cyml`), built on
[hisab](https://github.com/MacCracken/hisab) for math/geometry, and ships as a
single distlib bundle, `dist/goonj.cyr`, for downstream Cyrius consumers.

37 modules covering sound propagation, room simulation, ray/beam/image-source
tracing, wave-based methods (DWM, FDTD, GFPE), impulse-response generation,
diffraction, resonance, and spatialization — **3,624 parity assertions across 37
test suites, all green** (41 suites / 3,667 assertions counting the hardening,
modal and smoke suites).

## Features

- **Propagation** — speed of sound, inverse-square law, ISO 9613-1 atmospheric
  absorption, Doppler shift, Snell-law atmospheric ray tracing, Miki ground reflection
- **Room & materials** — shoebox geometry, walls, BVH acceleration; frequency-dependent
  absorption/scattering (7 presets), wall transmission loss, JCAL porous model, metamaterials
- **Geometric acoustics** — ray tracing (single/multiband, BVH), image-source early
  reflections, radiosity, beam tracing, diffuse-rain late reverb, scattering
- **Wave-based** — 3D Digital Waveguide Mesh (DWM), 2D FDTD modal solver, GFPE outdoor
  propagation, hybrid crossover
- **Impulse responses & metrics** — Sabine/Eyring/Fitzroy/Kuttruff RT60, energy decay
  curves, room-acoustics metrics (C50/C80/D50/EDT/G/ts/LF/IACC/STI), FDN reverb, dark velvet noise
- **Diffraction & propagation models** — UTD half-plane and Kouyoumjian-Pathak
  wedge diffraction, occlusion, UDFA, outdoor (ISO 9613-2), underwater, coupled
  rooms, portals, vibroacoustics
- **Spatialization & I/O** — ambisonics (B-format + 3rd-order HOA), binaural HRTF,
  directivity patterns, 16-bit PCM WAV export
- **Integration APIs** — `dhvani` (IR handoff), `kiran` (occlusion), `soorat` (visualization)
- **Diagnostics** — real sakshi-backed logging with a verbose mode (`logging_init_verbose`)

## Quick start

```sh
cyrius deps                                  # resolve hisab (+ transitive sakshi) into lib/
cyrius build examples/basic.cyr build/basic  # build the demo, then ./build/basic
cyrius build src/main.cyr build/goonj        # build the smoke binary
cyrius test                                  # run every suite (auto-discovers all 41)
cyrius test tests/propagation.tcyr           # or just one, by path
cyrius bench tests/dwm.bcyr                  # run a benchmark
cyrius distlib                               # regenerate dist/goonj.cyr (the shippable bundle)
```

[`examples/basic.cyr`](examples/basic.cyr) is the shortest end-to-end tour — speed
of sound, a shoebox hall, RT60, Schroeder frequency, a room mode and a Doppler
shift. It builds against the **shipped bundle**, so it doubles as a worked example
of how a downstream consumer folds goonj in:

```
Speed of sound at 20 C      : 343.4 m/s
Hall volume                 : 7200.0 m3
Sabine RT60                 : 4.95 s
Schroeder frequency         : 52.0 Hz
```

Cyrius has a flat, f64 API (no `use`/methods). A room + RT60, for example:

```
var geom = room_geometry_shoebox(f64_from(10), f64_from(8), f64_from(3), acoustic_material_concrete());
var rt60 = sabine_rt60(room_geometry_volume_shoebox(geom), room_geometry_total_absorption(geom));
var c    = speed_of_sound(f64_from(20));   # ≈ 343.4 m/s at 20 °C
```

## Dependency stack

```
goonj (Cyrius acoustics)
  ├── hisab (math/geometry — HVec3, geo, BVH; consumed as dist/hisab.cyr, tag 2.11.2)
  ├── sakshi (structured logging; pulled transitively via hisab, 2.4.11)
  └── Cyrius stdlib (math, ganita, vec, fnptr, bench, …)
```

No Rust dependencies — serde, thiserror, tracing, and criterion were all dropped
in the port.

## Consumers

dhvani (impulse responses for convolution reverb), shruti (room simulation for
mixing), kiran/joshua (game audio propagation, occlusion), aethersafha (spatial
audio for conferencing). *None consume the Cyrius bundle yet — they await their
own ports.*

## Documentation

- [`docs/development/state.md`](docs/development/state.md) — live state
- [`docs/development/roadmap.md`](docs/development/roadmap.md) — milestones
- [`docs/benchmarks-rust-vs-cyrius.md`](docs/benchmarks-rust-vs-cyrius.md) — Rust→Cyrius perf comparison
- [`docs/guides/getting-started.md`](docs/guides/getting-started.md) — build, layout, how to change a module
- [`docs/adr/`](docs/adr/) — decision records ([0001](docs/adr/0001-rust-to-cyrius-port-conventions.md) port conventions, [0002](docs/adr/0002-signed-index-and-float-conversion-hazards.md) signed-index / float-conversion hazards)

## License

GPL-3.0-only
