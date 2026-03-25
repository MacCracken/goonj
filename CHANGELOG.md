# Changelog

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

### Stats
- 181 tests (174 unit + 6 integration + 1 doc)
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
