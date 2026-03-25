# Goonj Architecture

## Module Map

```
goonj
├── error.rs              — GoonjError (5 variants)
├── material.rs           — AcousticMaterial, 7 presets, frequency-dependent absorption
├── propagation.rs        — speed_of_sound, inverse_square_law, doppler_shift, atmospheric_absorption
│                           WindProfile, TemperatureProfile, GroundImpedance, atmospheric ray tracing
├── room.rs               — Wall, RoomGeometry, AcousticRoom, AcceleratedRoom (BVH)
├── impulse.rs            — ImpulseResponse, MultibandIr, IrConfig, generate_ir
│                           sabine_rt60, eyring_rt60, energy_decay_curve
├── ray.rs                — AcousticRay, MultibandRay, RayPath, RayBounce
│                           ray_wall_intersection, reflect_ray, trace_ray, trace_ray_bvh
├── image_source.rs       — Image-source method: compute_early_reflections, ImageSource
├── diffuse.rs            — Diffuse rain: generate_diffuse_rain, fibonacci_sphere
├── analysis.rs           — Room metrics: clarity_c50, clarity_c80, definition_d50
│                           sti_estimate, suggest_absorption_placement
├── diffraction.rs        — edge_diffraction_loss, is_occluded, diffraction_path_extra
├── resonance.rs          — room_mode, axial_modes, schroeder_frequency, modal_density
├── wav.rs [feature: wav] — write_wav_mono, write_wav_stereo (16-bit PCM)
├── binaural.rs [feature: binaural] — BinauralIr, HrtfDataset, generate_binaural_ir
└── integration/
    ├── dhvani.rs [feature: dhvani-compat]  — DhvaniIr, generate_dhvani_ir
    ├── kiran.rs  [feature: kiran-compat]   — OcclusionEngine, OcclusionResult
    └── soorat.rs [feature: soorat-compat]  — RayVisualization, PressureMap, ModeVisualization
```

## Data Flow

```
Source + Listener + Room
        │
        ├── Image-Source Method ──→ Early Reflections (exact specular)
        │                                    │
        ├── Diffuse Rain ─────────→ Late Reverb (stochastic)
        │                                    │
        └── generate_ir() ←─────────────────┘
                │
                ├── MultibandIr (6 bands) ──→ to_broadband() ──→ ImpulseResponse
                │
                ├── analysis: C50, C80, D50, STI
                │
                ├── wav: write_wav_mono/stereo
                │
                └── binaural: generate_binaural_ir (+ HRTF)
```

## Consumers

- **dhvani** — computed impulse responses feed convolution reverb
- **shruti** — room simulation for mixing (virtual studio acoustics)
- **kiran/joshua** — game audio propagation, occlusion, spatial effects
- **aethersafha** — spatial audio for video conferencing

## Dependency Stack

```
goonj (acoustics)
  └── hisab (math — Vec3, geometry, BVH, FFT)
```

## Feature Flags

| Feature | Modules | Purpose |
|---------|---------|---------|
| `wav` | `wav.rs` | WAV file export |
| `binaural` | `binaural.rs` | Binaural IR with HRTF |
| `dhvani-compat` | `integration/dhvani.rs` | dhvani IR handoff |
| `kiran-compat` | `integration/kiran.rs` | Game audio occlusion |
| `soorat-compat` | `integration/soorat.rs` | Visualization data |
| `logging` | `logging.rs` | tracing-subscriber init |
