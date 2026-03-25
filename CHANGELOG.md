# Changelog

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
