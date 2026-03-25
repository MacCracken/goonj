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

## Tier 2 — Major Feature Additions

### Sound Transmission Through Walls
- [ ] Frequency-dependent transmission loss (Sound Reduction Index Rw)
- [ ] Mass law and Davy partition theory
- [ ] Essential for multi-room simulation (shruti, kiran)

### Source Directivity Patterns
- [ ] Frequency-dependent balloon data (CLF/CF1/CF2 format)
- [ ] Directivity applied to ray/image-source energy computations
- [ ] Critical for realistic speaker and instrument simulation

### Portal-Based Propagation
- [ ] Sound through doorways, windows, openings between rooms
- [ ] Aperture diffraction modeling
- [ ] Coupled-room energy exchange
- [ ] High priority for kiran/joshua game audio

### 1st-Order Ambisonics (B-Format) Output
- [ ] W, X, Y, Z channel encoding of reflections
- [ ] Source/receiver orientation-independent sound field
- [ ] Essential for VR/AR (aethersafha) and flexible playback decoding

### Coupled Room Acoustics
- [ ] Multi-room energy exchange producing double-slope decay
- [ ] Statistical coupling model or diffusion equation approach
- [ ] Concert halls with coupled volumes, office spaces, game environments

### Biot-Tolstoy-Medwin (BTM) Diffraction
- [ ] Finite-edge diffraction with multiple virtual sources along edge
- [ ] Most accurate geometric diffraction model available
- [ ] Quality option alongside UTD for architectural verification

### Vector-Based Scattering Model
- [ ] Replace normal-blending with proper hemisphere sampling
- [ ] Frequency-dependent, size-dependent scattering coefficients
- [ ] ODEON-style `(1-s)*specular + s*random_hemisphere` model

### FDN/SDN Late Reverberation
- [ ] Feedback Delay Network alternative to diffuse rain
- [ ] Scattering Delay Network (geometry-aware reverb, wall-tied nodes)
- [ ] Industry standard for efficient late reverb

### 3rd-Order Higher-Order Ambisonics
- [ ] 16-channel HOA encoding for spatial precision
- [ ] Spherical harmonic coefficients per reflection

---

## Tier 3 — Competitive Differentiation

### Beam Tracing
- [ ] Volumetric beams replacing discrete rays — eliminates sampling artifacts
- [ ] Handles specular reflection and transmission without aliasing
- [ ] Reference: Funkhouser et al., JASA 2004

### Acoustic Radiosity
- [ ] Energy-exchange between surface patches (diffuse-only)
- [ ] Source/receiver-independent computation (move listener for free)
- [ ] ODEON-style ray-radiosity hybrid

### Acoustic Diffusion Equation (ADE)
- [ ] Energy density PDE (Fick's law analogy)
- [ ] Effective for long rooms, coupled rooms, industrial spaces
- [ ] Complementary to geometric methods

### ISO 9613-2 Outdoor Methods
- [ ] Barrier diffraction (screening)
- [ ] Foliage attenuation
- [ ] Meteorological correction factors
- [ ] Terrain profiling

### JCAL Porous Material Model
- [ ] Johnson-Champoux-Allard-Lafarge 6-parameter model
- [ ] Replaces Miki for detailed porous material characterization
- [ ] Requires porosity, tortuosity, characteristic lengths, thermal permeability

### Hybrid Frequency Crossover Interface
- [ ] Define API for wave-based results below Schroeder frequency
- [ ] Geometric results above crossover, blend in overlap region
- [ ] Architecture for future FDTD/FEM plugin

### Universal Diffraction Filter Approximation (UDFA)
- [ ] Filter-based diffraction for real-time applications (2024)
- [ ] Higher-order interactions and finite objects
- [ ] Reference: Acta Acustica 2024

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
