# Changelog

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
