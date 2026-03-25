# Goonj

**Goonj** (गूँज — Hindi/Urdu for "echo, resonance") — acoustics engine for the [AGNOS](https://github.com/MacCracken/agnosticos) ecosystem.

Built on [hisab](https://github.com/MacCracken/hisab) for math. Provides sound propagation, room simulation, impulse response generation, acoustic ray tracing, diffraction, and resonance analysis.

## Features

- **Propagation** — speed of sound, inverse square law, atmospheric absorption, Doppler shift
- **Room geometry** — walls, shoebox constructor, surface area, volume
- **Materials** — frequency-dependent absorption (7 presets: concrete, carpet, glass, wood, curtain, drywall, tile)
- **Impulse response** — Sabine RT60, Eyring RT60, energy decay curve
- **Ray tracing** — acoustic ray intersection, specular reflection with energy absorption
- **Diffraction** — occlusion detection, edge diffraction loss (UTD simplified)
- **Resonance** — room modes, axial modes, Schroeder frequency, modal density

## Quick Start

```rust
use goonj::{propagation, impulse, material::AcousticMaterial, room::AcousticRoom};

// Speed of sound at 20°C
let speed = propagation::speed_of_sound(20.0);
assert!((speed - 343.4).abs() < 0.1);

// Create a shoebox room
let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());

// Compute reverberation time
let absorption = room.geometry.total_absorption();
let volume = room.geometry.volume_shoebox();
let rt60 = impulse::sabine_rt60(volume, absorption);
println!("RT60: {rt60:.2}s");
```

## Dependency Stack

```
goonj (acoustics)
  └── hisab (math)
```

## Consumers

- **dhvani** — impulse responses for convolution reverb
- **shruti** — room simulation for mixing
- **kiran/joshua** — game audio propagation, occlusion, spatial audio
- **aethersafha** — spatial audio for video conferencing

## License

GPL-3.0
