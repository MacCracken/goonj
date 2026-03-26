# Changelog

## [Unreleased]

### Added
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
