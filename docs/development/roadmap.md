# Goonj Roadmap

## Status

**v1.0.0** — All original criteria met. Roadmap below is demand-gated future work informed by deep external research.

## v1.0.0 Criteria (all met)

- API frozen ✓
- Zero unwrap/panic in library code ✓
- 90%+ test coverage ✓ (97.58%)
- Benchmark history with golden numbers ✓ (28 benchmarks)
- 3+ downstream consumers ✓ (dhvani, kiran, shruti, soorat)
- docs.rs complete ✓
- Supply chain clean ✓

---

## Tier 1 — Accuracy & Standards Compliance (completed)

### Completed
- [x] **8 octave bands** (63–8000 Hz) — ISO 3382-1 standard, `NUM_BANDS = 8`
- [x] **Full ISO 9613-1 atmospheric absorption** — O₂/N₂ molecular relaxation, temperature/pressure/humidity
- [x] **Miki ground impedance** — corrected Delany-Bazley with positive real impedance
- [x] **IEC 60268-16:2020 STI** — 7 bands × 14 modulation frequencies, speech spectrum weighting, redundancy
- [x] **ISO 3382-1 parameters** — EDT, Sound Strength G, Centre Time ts
- [x] **Fitzroy RT60** — non-uniform absorption per axis pair + Kuttruff variance correction
- [x] **Full UTD diffraction** — Kouyoumjian-Pathak wedge coefficients with Fresnel transition

### Remaining Tier 1 (completed)
- [x] Lateral Energy Fraction LF (JLF) — binaural figure-8 microphone model
- [x] IACC (Interaural Cross-Correlation) — from binaural IR with ±1ms lag search
- [x] Octave-band filtering (2nd-order Butterworth bandpass)
- [x] Kouyoumjian-Pathak UTD wedge diffraction with geometry parameters

---

## Tier 2 — Major Feature Additions (completed)

- [x] **Wall transmission** — `WallConstruction` with mass law + Davy model, TL in dB, τ coefficient
- [x] **Source directivity** — `DirectivityPattern` enum (omni, cardioid, sub/supercardioid, figure-8, tabulated balloon)
- [x] **Portal propagation** — `Portal` with aperture diffraction model, `portal_energy_transfer()`
- [x] **1st-order Ambisonics** — `BFormatIr` with W/X/Y/Z encoding, `encode_bformat()`
- [x] **Coupled rooms** — `CoupledRooms` + `coupled_room_decay()` with double-slope eigenvalue analysis
- [x] **UTD wedge diffraction** — `utd_wedge_diffraction()` with K-P coefficients and geometry params
- [x] **Vector scattering** — `cosine_hemisphere_sample()` + `scatter_direction()` (ODEON-style)
- [x] **FDN reverb** — 8-delay Householder FDN with room-derived delay lengths
- [x] **3rd-order HOA** — `HoaIr` with 16-channel ACN/SN3D spherical harmonic encoding

---

## Tier 3 — Competitive Differentiation (completed)

- [x] **Beam tracing** — `AcousticBeam`, `trace_beam()`, `generate_beam_set()` with frustum-based propagation
- [x] **Acoustic radiosity** — `Patch`, `create_patches()`, `solve_radiosity()` with form factor computation
- [x] **Acoustic diffusion equation** — 2D FTCS solver for energy density PDE (`solve_diffusion_2d()`)
- [x] **ISO 9613-2 outdoor methods** — `barrier_insertion_loss()`, `foliage_attenuation()`, `meteorological_correction()`, `ground_attenuation()`
- [x] **JCAL porous material model** — `JcalMaterial` with 6-parameter characterization and absorption coefficient computation
- [x] **Hybrid frequency crossover** — `CrossoverConfig`, `blend_weights()`, `blend_results()` for wave/geometric blending
- [x] **UDFA filter-based diffraction** — `DiffractionFilter`, `compute_diffraction_filter()`, `chain_diffraction_filters()`

---

## Tier 4 — Research Track (Future-Proofing)

### Wave-Based Methods
- [ ] Simple 2D FDTD modal solver below Schroeder frequency
- [ ] Digital Waveguide Mesh for low-frequency room simulation
- [ ] BEM for exterior/loudspeaker problems

### Neural/ML Integration
- [ ] Neural late-reverb completion (geometric early + ML tail)
- [ ] ML-based HRTF interpolation interface (spherical harmonics)
- [ ] Dark Velvet Noise reverb synthesis (2024, non-exponential decay)
- [ ] Physics-Informed Neural Networks for wave equation

### Emerging Phenomena
- [ ] Non-linear propagation (Burgers equation, >140 dB SPL)
- [ ] Acoustic metamaterial material types
- [ ] Outdoor terrain propagation (GFPE method)
- [ ] Structure-borne sound / vibroacoustics coupling
- [ ] Underwater acoustics (different physics entirely)

---

## References

- Funkhouser et al., "Beam Tracing for Interactive Architectural Acoustics," JASA 115(2), 2004
- Siltanen et al., "Room Acoustic Rendering Equation," JASA 122(3), 2007
- Valeau et al., "Diffusion Equation for Room-Acoustic Prediction," JASA 119(3), 2006
- De Sena et al., "Scattering Delay Network," AES 41st Conf., 2011
- Fagerström et al., "Non-Exponential Reverberation with Dark Velvet Noise," JAES 72(6), 2024
- Miki, "Acoustical properties of porous materials — modifications of Delany-Bazley models," 1990
- ISO 3382-1:2009 — Room acoustics measurement
- ISO 9613-1:1993 — Atmospheric absorption of sound
- IEC 60268-16:2020 — Speech Transmission Index
- Lambert diffuse reflection revisited, JASA 156(6), 2024
