# Goonj Roadmap

## Status

**v0.1.0** — Initial scaffold with real physics implementations.

## Engineering Backlog

No open items.

## Future Features (demand-gated)

### Ray Tracing Improvements
- Image-source method for early reflections (exact specular paths)
- Diffuse rain (stochastic rays for late reverb)
- Frequency-dependent ray tracing (per-band absorption per bounce)
- BVH acceleration for large room geometries (via hisab spatial structures)

### Impulse Response
- Generate full IR from ray trace results (early + late)
- Binaural IR (HRTF convolution for headphone spatialization)
- IR export to WAV for dhvani convolution reverb

### Advanced Propagation
- Wind gradient refraction (sound bending in atmosphere)
- Temperature gradient effects (thermal inversions)
- Ground reflection with impedance model

### Room Analysis
- Clarity (C50, C80) — speech intelligibility metrics
- Definition (D50) — early-to-total energy ratio
- Speech Transmission Index (STI) estimation
- Optimal absorption placement suggestions

### Integration
- dhvani: direct IR handoff for convolution reverb
- soorat: acoustic visualization (ray paths, pressure maps, mode patterns)
- kiran: real-time occlusion queries for game audio

## v1.0.0 Criteria

- API frozen
- Zero unwrap/panic in library code
- 90%+ test coverage
- Benchmark history with golden numbers
- 3+ downstream consumers
- docs.rs complete
- Supply chain clean
