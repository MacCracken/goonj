# Goonj Architecture

## Module Map

```
goonj
├── error.rs         — GoonjError (5 variants)
├── material.rs      — AcousticMaterial, 7 presets, frequency-dependent absorption
├── propagation.rs   — speed_of_sound, inverse_square_law, doppler_shift, atmospheric_absorption
├── room.rs          — Wall, RoomGeometry, AcousticRoom, shoebox constructor
├── impulse.rs       — ImpulseResponse, sabine_rt60, eyring_rt60, energy_decay_curve
├── ray.rs           — AcousticRay, RayHit, ray_wall_intersection, reflect_ray
├── diffraction.rs   — edge_diffraction_loss, is_occluded, diffraction_path_extra
└── resonance.rs     — room_mode, axial_modes, schroeder_frequency, modal_density
```

## Consumers

- **dhvani** — computed impulse responses feed convolution reverb
- **shruti** — room simulation for mixing (virtual studio acoustics)
- **kiran/joshua** — game audio propagation, occlusion, spatial effects
- **aethersafha** — spatial audio for video conferencing

## Dependency Stack

```
goonj (acoustics)
  └── hisab (math — Vec3, geometry, BVH for spatial queries)
```
