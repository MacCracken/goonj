# Goonj Architecture

goonj is a Cyrius library (v2.0.2): 37 self-contained modules in `src/*.cyr`
(plus `src/main.cyr`, a smoke binary that does not include the library). Modules
carry no `include` lines; stdlib + hisab resolve from `cyrius.cyml`. All 37
concatenate — in the dependency order below — into `dist/goonj.cyr` via
`cyrius distlib`. There are **no Cargo-style feature flags**; every module ships
in the bundle, and consumers pull the single file. `cyrius distlib` also emits
`dist/goonj.deps`, a sidecar naming the 13 stdlib leaves the bundle needs in
scope; downstream `cyrius deps` reads it, so it ships with the bundle. Both
files are tracked — a consumer that has to hand-declare those leaves gets no
error when it under-declares.

## Module map (by dependency layer)

```
L0  error.cyr             — integer error codes (ERR_*), GOONJ_EPSILON, F64_NEG_INF
    propagation.cyr       — speed_of_sound, inverse_square_law, doppler_shift, atmospheric_absorption,
                            Miki ground reflection, Snell atmospheric ray tracing (fnptr callback)
    resonance.cyr         — room_mode, axial_modes, schroeder_frequency, modal_density
    ambisonics.cyr        — B-format + 3rd-order HOA (real spherical harmonics, ACN/SN3D)
    scattering.cyr        — cosine-weighted hemisphere sampling (caller-supplied randoms)
    dark_velvet_noise.cyr — sparse stochastic late reverb (inline xorshift64 PRNG)
    logging.cyr           — real sakshi-backed logging + verbose mode (goonj_log_*)
L1  material.cyr          — AcousticMaterial (7 presets), WallConstruction (TL), JcalMaterial (JCAL)
L2  hybrid, directivity, room (Wall/RoomGeometry/AcousticRoom/AcceleratedRoom-BVH), metamaterial,
    fdn, gfpe, diffusion, fdtd, outdoor, portal, udfa, underwater, vibroacoustics, bridge
L3  ray (AcousticRay/MultibandRay/RayPath, trace_ray, trace_ray_bvh), radiosity,
    image_source (compute_early_reflections), dwm (3D Digital Waveguide Mesh solver)
L4  diffuse (generate_diffuse_rain, fibonacci_sphere), diffraction (UTD/BTM), beam,
    analysis (C50/C80/D50/EDT/G/ts/LF/IACC/STI)
L5  impulse (RT60 estimators, ImpulseResponse, MultibandIr, generate_ir), coupled
L6  wav (16-bit PCM byte-buffer export), binaural (HRTF), dhvani, kiran, soorat (integration APIs)
```

The integration modules are **flat top-level files** (`src/dhvani.cyr`,
`src/kiran.cyr`, `src/soorat.cyr`) — not a `src/integration/` subdirectory.

## Data flow

```
Source + Listener + Room
        │
        ├── image_source: compute_early_reflections ──→ early reflections (exact specular)
        ├── diffuse: generate_diffuse_rain ───────────→ late reverb (stochastic)
        │                                                    │
        └── impulse: generate_ir ←───────────────────────────┘
                │
                ├── MultibandIr (8 bands) ──→ broadband ──→ ImpulseResponse
                ├── analysis: C50/C80/D50/EDT/STI/…
                ├── wav: write_wav_mono/stereo (byte buffer)
                └── binaural: generate_binaural_ir (+ HRTF)
```

## Dependency stack

```
goonj (Cyrius acoustics)
  ├── hisab (math/geometry — HVec3, geo, BVH; consumed as dist/hisab.cyr)
  ├── sakshi (structured logging; transitive via hisab; used by logging.cyr)
  └── Cyrius stdlib (math, ganita, vec, fnptr, bench, string, …)
```

No Rust dependencies (serde/thiserror/tracing/criterion dropped in the port).

## Consumers

dhvani (impulse responses for convolution reverb), shruti (room simulation for
mixing), kiran/joshua (game audio propagation, occlusion), aethersafha (spatial
audio for conferencing). None consume the Cyrius bundle yet.
