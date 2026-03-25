# Goonj Roadmap

## Status

**v0.2.0** — Full roadmap implemented. All planned features delivered.

## Completed

### Ray Tracing Improvements (v0.2.0)
- [x] Image-source method for early reflections (exact specular paths)
- [x] Diffuse rain (stochastic rays for late reverb)
- [x] Frequency-dependent ray tracing (per-band absorption per bounce)
- [x] BVH acceleration for large room geometries (via hisab spatial structures)

### Impulse Response (v0.2.0)
- [x] Generate full IR from ray trace results (early + late)
- [x] Binaural IR (HRTF convolution for headphone spatialization)
- [x] IR export to WAV for dhvani convolution reverb

### Advanced Propagation (v0.2.0)
- [x] Wind gradient refraction (sound bending in atmosphere)
- [x] Temperature gradient effects (thermal inversions)
- [x] Ground reflection with impedance model (Delany-Bazley)

### Room Analysis (v0.2.0)
- [x] Clarity (C50, C80) — speech intelligibility metrics
- [x] Definition (D50) — early-to-total energy ratio
- [x] Speech Transmission Index (STI) estimation
- [x] Optimal absorption placement suggestions

### Integration (v0.2.0)
- [x] dhvani: direct IR handoff for convolution reverb
- [x] soorat: acoustic visualization (ray paths, pressure maps, mode patterns)
- [x] kiran: real-time occlusion queries for game audio

## Engineering Backlog

No open items.

## v1.0.0 Criteria

- API frozen
- Zero unwrap/panic in library code ✓
- 90%+ test coverage
- Benchmark history with golden numbers ✓
- 3+ downstream consumers
- docs.rs complete
- Supply chain clean ✓
