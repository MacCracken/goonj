# Changelog

## [Unreleased]

### Added
- **metamaterial** — engineered acoustic materials with frequency-dependent
  effective parameters. Three analytical models — `NegativeStiffness`,
  `NegativeDensity`, `DoublyNegative` — driven by a Drude–Lorentz
  `LorentzianResonance` (centre, plasma, damping, background_value) plus
  a `LookupTable` form for manufacturer absorption datasheets. Each
  variant produces an 8-band absorption profile via a bulk-impedance
  approximation `α = 1 − |R|²` with `R = (Z_eff − Z₀)/(Z_eff + Z₀)` and
  `Z_eff = √(ρ_eff·K_eff)` against air at Z₀ ≈ 413 rayls. Convenience
  constructor `LorentzianResonance::with_dc_value` lets callers specify
  the DC value directly instead of the canonical Drude–Lorentz
  background. Outputs convert to `AcousticMaterial` via
  `to_acoustic_material(name, scattering)` so a metamaterial panel
  plugs into the existing ray / IR pipeline as a wall. Benchmark
  `metamaterial/absorption_bands_negstiff`: 211 ns. References:
  Liu 2000, Fang 2006, Yang 2008, Lee 2010.
- **dark_velvet_noise** — Fagerström et al. 2024 reverb synthesis from
  sparse stochastic pulse sequences. New module exposes `DvnConfig`,
  `DecayEnvelope::{Exponential, PiecewiseDb}`, and
  `synthesize_dvn_ir(&config) -> Vec<f32>`. Supports non-exponential
  decay shapes (e.g. coupled-room double-slope) via piecewise-linear-dB
  breakpoints and "dark" tail coloration via a time-varying 1-pole
  low-pass interpolated from `coloration_initial_cutoff_hz` to
  `coloration_final_cutoff_hz`. RT60 recovery within the paper's 4%
  budget (regression test asserts `< 4%`). Benchmark
  `dvn/synthesize_2s_2khz_density`: 165 μs for a 2-second 48 kHz IR
  (~290× real-time, ~15× faster than `impulse::generate_ir`).

## [1.2.1] - 2026-05-01

Dependency bump release. Picks up the **hisab v1.0** stable line (we had
been running on the pre-v1 `0.24` series since project inception) and
brings the rest of the supply chain to current. Added supply-chain
checks to the cleanliness gates.

### Changed
- **deps** — `hisab 0.24 → 1.4` (first stable major). No goonj-side API
  changes required: every `hisab::Vec3`, `hisab::geo::Bvh`, and
  `hisab::geo::Ray` call site recompiles unchanged against v1.4.
- **deps** — `criterion 0.5 → 0.8`. Replaced the now-deprecated
  `criterion::black_box` import with `std::hint::black_box` in
  `benches/benchmarks.rs` (criterion 0.8 removes the re-export).
- **deps** — pinned minimum patch versions: `serde 1.0.228`,
  `thiserror 2.0.18`, `tracing 0.1.44`, `tracing-subscriber 0.3.23`,
  `serde_json 1.0.149`. No behaviour change; tightens the floor so
  `cargo audit` runs against a known-good baseline.

### Added
- **CI/dev gates** — `cargo audit` and `cargo deny check` are now part
  of the documented P(-1) and Working-Loop cleanliness sweeps. Both run
  clean on the new lockfile (91 transitive crates, 0 advisories,
  licenses ok, bans ok, sources ok). The existing `deny.toml` allowlist
  (GPL-3.0-only, MIT, Apache-2.0, Unicode-3.0) covers the v1.4 hisab
  tree without modification.

### Stats
- 411 tests passing (404 unit + 6 integration + 1 doc), 28 benchmarks
- All benchmarks hold within noise vs. v1.2.0 (e.g. `analysis/sti`
  3,624,973 ns → 3,621,627 ns) — no regression from the dep bump
- All six gates clean: `cargo fmt --check`,
  `cargo clippy --all-features --all-targets -- -D warnings`,
  `cargo test --all-features`, `cargo audit`, `cargo deny check`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps`

## [1.2.0] - 2026-05-01

Final P(-1) scaffold-hardening pass before opening v1.3 feature work. No
new public API; all changes are internal correctness / performance fixes.

### Performance
- **analysis** — `sti_estimate`: hoisted the broadband MTI computation out
  of the per-band loop. The same broadband IR was being reduced 7× with
  identical inputs and outputs; the result is now computed once and
  broadcast to `band_mti`. Benchmark: `analysis/sti` 28,751,483 ns →
  3,624,973 ns (−87%, **~7.9× faster**). Formula identity preserved:
  `Σαₖ·MTI − Σβₖ·√(MTIₖ·MTIₖ₊₁)` collapses to `(Σαₖ − Σβₖ)·MTI` when
  all bands share a value, exactly as before.

### Fixed
- **diffusion** — `solve_diffusion_2d` now also early-returns when
  `speed_of_sound <= 0.0`. Previously a zero/negative speed propagated
  into `d_coeff = c · mfp / 3`, leaving `dt_max = dx² / 0` to flow
  through `min(config.dt)` and producing a degenerate-but-running
  simulation. Added a regression test.
- **integration/dhvani** — removed dead `_surface_area` binding in
  `generate_dhvani_ir`; per-band Sabine RT60 already pulls absorption
  area directly from `wall.area()`.
- **scripts/bench-history.sh** — pass `--all-features` so feature-gated
  benchmarks (`wav/export_48k_2s`, `binaural/generate_ir_shoebox`)
  actually compile. The script silently failed on these benches after
  those features were added in v0.2.0 / v1.1.0.

### Stats
- 411 tests passing (404 unit + 6 integration + 1 doc), 28 benchmarks, 30 modules
- New regression test: `diffusion::tests::diffusion_zero_speed_of_sound_returns_empty`
- All formulas remain verified against ISO/IEC and peer-reviewed sources
- `cargo fmt`, `cargo clippy --all-features --all-targets -- -D warnings`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps` all clean

## [1.1.1] - 2026-03-29

### Changed
- **license** — GPL-3.0 → GPL-3.0-only across Cargo.toml, README, CLAUDE.md, deny.toml

## [1.1.0]

### Added
- **bridge** — cross-crate primitive-value bridges for pavan (wind attenuation, Doppler, effective speed of sound), badal (air absorption from weather, speed of sound, pressure conversion), ushma (absorption temperature scaling, sound speed gradient), bijli (EM resonance coupling, piezoelectric acoustic power)
- **underwater** (feature-gated) — Mackenzie ocean sound speed, Francois-Garrison seawater absorption, Hamilton seabed sediment model (sand/silt/clay/rock), Rayleigh bottom reflection, Eckart sea surface scattering

### Tier 1–3 Accuracy & Standards
- **8 octave bands** (63–8000 Hz), full ISO 9613-1 atmospheric absorption, Miki ground impedance
- **IEC 60268-16:2020 STI** with correct α_k/β_k redundancy weights
- **ISO 3382-1**: EDT, G, ts, LF, IACC, octave-band filtering
- **Fitzroy RT60** + Kuttruff correction, full UTD wedge diffraction (K-P 1974)

### Tier 2 Features
- Wall transmission (mass law + Davy), source directivity, portal propagation
- 1st/3rd-order Ambisonics (SN3D/ACN), coupled rooms, vector scattering
- FDN reverb (zero-allocation Householder), JCAL porous materials

### Tier 3 Features
- Beam tracing, acoustic radiosity, 2D diffusion equation solver
- ISO 9613-2 outdoor (barrier, foliage, meteorological, ground)
- Hybrid frequency crossover, UDFA filter-based diffraction

### Correctness (P(-1) hardening)
- Ambisonics ACN 10/14 SN3D factor: `sqrt(15/2)/2` → `sqrt(15)/2`
- Coupled rooms: amplitude rate (6.908) → energy rate (13.816) for gamma
- STI: correct IEC 60268-16:2020 formula `STI = Σ(α_k×MTI_k) − Σ(β_k×√(MTI_k×MTI_{k+1}))`
- C_met: `h_avg` → `h_s + h_r`, removed spurious squared factor
- UTD Fresnel argument: fixed N± computation per K-P 1974
- Miki model: corrected coefficients to 0.0699 / -0.1071
- FDN: zero-allocation `process_sample` (pre-allocated scratch buffer)
- Portal: two-stage inverse square `A/(4π×d₁²×d₂²)`

### Security
- Capped image-source max_order (20 shoebox / 6 general)
- Capped diffusion grid (2000×2000), radiosity patches (100/wall)
- Overflow-safe WAV data size, IR generation capped at 10min@192kHz

### Stats
- 378 tests, 28 benchmarks, 29 modules
- All formulas verified against references (ISO 9613-1, IEC 60268-16:2020, Miki 1990, K-P 1974, Mackenzie 1981, Francois-Garrison 1982, Hamilton 1980)

## [1.0.0] - 2026-03-25

Security hardening, correctness fixes, and documentation pass for v1.0.0 freeze.

### Security
- Capped `max_order` to 20 (shoebox) / 6 (general) to prevent O(n³) / O(W^N) DoS
- Added overflow-safe WAV data size calculation (`checked_mul` + `try_from`)
- Capped IR generation to 10min @ 192kHz max to prevent OOM
- Capped atmospheric trace to 1M iterations to prevent infinite loops

### Correctness
- Fixed `eyring_rt60` at absorption=1.0 → returns 0 (anechoic), not infinity
- Added module-level docs on `ray.rs`
- Documented shoebox normal convention (outward-facing)
- Added capacity hints for image source, atmospheric trace Vecs

### Documentation
- Zero `missing_docs` warnings — all public types, functions, fields documented
- Architecture overview updated with data flow diagram and feature flag table
- Roadmap expanded with 30 research-backed future items across 4 tiers

### Stats
- 240 tests, 28 benchmarks, 97.58% coverage
- All v1.0.0 criteria met

## [0.2.0] - 2026-03-25

Full roadmap implementation: frequency-dependent ray tracing, IR generation, room analysis, and downstream integration.

### New Modules
- **image_source** — Allen & Berkeley image-source method for exact early specular reflections (shoebox + general rooms)
- **diffuse** — Stochastic ray tracing (diffuse rain) for late reverb tails with Fibonacci sphere distribution
- **analysis** — Room acoustics metrics: C50, C80, D50, STI estimation, absorption placement suggestions
- **wav** (feature-gated) — 16-bit PCM WAV export for mono and stereo impulse responses
- **binaural** (feature-gated) — Binaural IR generation with user-provided HRTF datasets
- **integration/dhvani** (feature-gated) — IR handoff for convolution reverb
- **integration/kiran** (feature-gated) — Real-time occlusion queries with BVH acceleration
- **integration/soorat** (feature-gated) — Visualization data structures (ray paths, pressure maps, mode patterns)

### Enhanced Modules
- **ray** — MultibandRay with per-band energy [f32; 6], RayBounce, RayPath, trace_ray scene tracer, BVH-accelerated trace_ray_bvh
- **room** — Wall::aabb(), RoomGeometry::build_bvh(), AcceleratedRoom with cached BVH
- **impulse** — IrConfig, MultibandIr, generate_ir() combining image-source + diffuse rain
- **propagation** — WindProfile, TemperatureProfile, GroundImpedance, atmospheric ray tracing with Snell's law refraction, Delany-Bazley ground reflection

### Audit Hardening
- Removed all `unwrap()` from library code (replaced with `is_none_or`)
- Added `#[inline]` on hot-path functions: ray intersection, reflection, PRNG — 9–18% speedup
- Eliminated 384KB allocation in STI estimation (compute h² on-the-fly)
- Added Vec pre-allocation with capacity hints
- Added edge-case guards: zero rays, zero speed_of_sound, empty geometry
- Fixed `suggest_absorption_placement` to use `target_rt60` parameter

### Stats
- 216 tests (209 unit + 6 integration + 1 doc)
- 21 criterion benchmarks with history tracking
- All cleanliness checks passing (fmt, clippy, audit, deny)

## [0.1.0] - 2026-03-24

Initial scaffold with real physics implementations.

### Modules
- **error** — GoonjError with 5 non-exhaustive variants
- **material** — AcousticMaterial with frequency-dependent absorption, 7 presets
- **propagation** — speed_of_sound, inverse_square_law, atmospheric_absorption, doppler_shift, dB SPL conversion
- **room** — Wall, RoomGeometry, AcousticRoom, shoebox constructor
- **impulse** — ImpulseResponse, sabine_rt60, eyring_rt60, energy_decay_curve
- **ray** — AcousticRay, RayHit, ray_wall_intersection, reflect_ray
- **diffraction** — edge_diffraction_loss, is_occluded, diffraction_path_extra
- **resonance** — room_mode, axial_modes, all_axial_modes, schroeder_frequency, modal_density
