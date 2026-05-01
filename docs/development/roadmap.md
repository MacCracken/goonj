# Goonj Roadmap

## Current Release

**v1.2.1** — Dependency bump. Picks up the hisab v1.0 stable line; supply-chain checks added to the gate.

- hisab 0.24 → 1.4 (no source changes required)
- criterion 0.5 → 0.8 (deprecated `criterion::black_box` migrated to `std::hint::black_box`)
- `cargo audit` + `cargo deny check` now part of the cleanliness sweep — both clean
- Bench numbers hold within noise vs. v1.2.0; no regression from the dep bump
- 411 tests, 28 benchmarks, 30 modules

---

## Completed

### v1.0.0 — Core Acoustics Engine
- 8-band ISO 3382-1 frequency analysis (63–8000 Hz)
- Full ISO 9613-1 atmospheric absorption (O₂/N₂ molecular relaxation)
- Miki ground impedance, IEC 60268-16:2020 STI
- ISO 3382-1 metrics: C50, C80, D50, EDT, G, ts, LF, IACC
- Sabine, Eyring, Fitzroy RT60 + Kuttruff correction
- Ray tracing (multiband, BVH-accelerated), image-source method, diffuse rain
- Binaural IR (HRTF), WAV export, integration APIs (dhvani, kiran, soorat)

### v1.1.0 — Advanced Methods + New Domains
- Wall transmission (mass law + Davy), source directivity, portal propagation
- 1st/3rd-order Ambisonics (SN3D/ACN), coupled rooms, FDN reverb
- Beam tracing, acoustic radiosity, 2D diffusion equation
- ISO 9613-2 outdoor (barrier, foliage, meteorological, ground)
- JCAL porous materials, hybrid frequency crossover, UDFA diffraction
- K-P UTD wedge diffraction, vector-based scattering (ODEON-style)
- Underwater acoustics (Mackenzie, Francois-Garrison, Hamilton, Eckart)
- Vibroacoustics (radiation efficiency, modal radiation, impetus coupling)

### v1.2.0 — Final Scaffold Hardening
- STI estimate: per-band MTI loop hoisted (broadband IR → identical MTI per band → compute once and broadcast). 28.75 ms → 3.62 ms (~7.9× faster).
- Diffusion solver: added `speed_of_sound <= 0.0` early-return guard + regression test.
- Removed dead `_surface_area` binding in dhvani integration.
- `bench-history.sh`: pass `--all-features` so wav/binaural benches compile.
- No new public API; closes the P(-1) cycle ahead of v1.3 feature work.

### v1.2.1 — Dependency Bump
- hisab `0.24 → 1.4` (first stable major; no source changes required).
- criterion `0.5 → 0.8`; bench file migrated from `criterion::black_box` to `std::hint::black_box`.
- Patch-version floors pinned: serde 1.0.228, thiserror 2.0.18, tracing 0.1.44, tracing-subscriber 0.3.23, serde_json 1.0.149.
- `cargo audit` + `cargo deny check` added to the documented cleanliness sweep — both clean (91 crates, 0 advisories).

---

## v1.3.0 — Wave-Based Methods + Emerging Algorithms

Pushed back from v1.2.0. Batched into a single release — committed per item, shipped together. Algorithms are well-defined with clear references.

### Reverb Synthesis
- [x] **Dark Velvet Noise reverb** — non-exponential decay modeling using sparse stochastic sequences. 4% RT60 error, 50% fewer filters. Reference: Fagerström et al., JAES 72(6), 2024.

### Material Extensions
- [ ] **Acoustic metamaterial types** — frequency-dependent negative-stiffness and negative-density material models for engineered absorbers. Lookup-table approach from manufacturer data.

### Outdoor Propagation
- [ ] **GFPE terrain propagation** — Green's Function Parabolic Equation for range-dependent outdoor environments with hills/ridges. Reference: Gilbert & Di (1993).

### Wave Solvers
- [ ] **2D FDTD modal solver** — explicit finite-difference time-domain below Schroeder frequency. Textbook algorithm (Botteldooren 1995). Plugs into hybrid crossover interface.

---

## v1.4.0 — Digital Waveguide Mesh

Carved out from v1.3.0 because it's the largest of the wave-based items and warrants its own release cycle. Anything that doesn't make the v1.3.0 cut cascades back here.

### Wave Solvers
- [ ] **Digital Waveguide Mesh** — FDTD variant with waveguide interpretation for room simulation. Reference: Wayverb (reuk), Smith (Stanford CCRMA).

---

## Not Mature — Watch

Theory not proven at production quality, or requires infrastructure (ML runtimes, massive solvers) that doesn't exist yet. Gate: implement only when independent validation or production-quality open-source reference appears.

| Item | Why Not Mature | Watch For |
|------|---------------|-----------|
| **Neural late-reverb completion** | MESH2IR (2022), Neural Acoustic Fields (2023) — papers only, no production open-source impl | A validated open-source model with reproducible results |
| **ML HRTF interpolation** | HRTFformer (2025), FiLM-HRTF (2023) — requires inference runtime, adds ML deps to pure-math lib | Lightweight inference (WASM-compatible) or pre-baked lookup tables |
| **Physics-Informed Neural Networks** | SIREN, Helmholtz-regularized — active research, no convergence on best approach | Consensus on architecture + open training pipeline |
| **Non-linear propagation (Burgers)** | Very specialized (>140 dB SPL) — no downstream consumer needs it | kiran/joshua explosion audio or sonic boom simulation demand |
| **BEM (Boundary Element Method)** | Massive engineering effort (complex linear system solver, surface meshing) — separate crate scale | Rust BEM library emerges, or consumer demand justifies the investment |
| **Full structural-acoustic FEM coupling** | Craig-Bampton substructuring needs a dedicated FEM solver beyond impetus scope | impetus grows elastic FEM, or a Rust FEM crate appears |

---

## Cross-Crate Bridges

- [ ] `bridge.rs` module — primitive-value conversions for cross-crate acoustics (no deps on sibling crates)
- [ ] **pavan bridge**: wind speed (m/s) → outdoor sound propagation attenuation factor; Mach number → Doppler shift
- [ ] **badal bridge**: humidity (%) → air absorption coefficients; temperature (°C) → speed of sound
- [ ] **ushma bridge**: temperature (°C) → material absorption coefficient scaling
- [ ] **bijli bridge**: EM frequency (Hz) → acoustic resonance coupling factor (vibroacoustics)

---

## References

- Funkhouser et al., "Beam Tracing for Interactive Architectural Acoustics," JASA 115(2), 2004
- Siltanen et al., "Room Acoustic Rendering Equation," JASA 122(3), 2007
- Valeau et al., "Diffusion Equation for Room-Acoustic Prediction," JASA 119(3), 2006
- De Sena et al., "Scattering Delay Network," AES 41st Conf., 2011
- Fagerström et al., "Non-Exponential Reverberation with Dark Velvet Noise," JAES 72(6), 2024
- Miki, "Acoustical properties of porous materials," J. Acoust. Soc. Japan, 1990
- Mackenzie, "Nine-term equation for sound speed in the oceans," JASA 70(3), 1981
- Francois & Garrison, "Sound absorption based on ocean measurements," JASA 72(6), 1982
- Hamilton, "Geoacoustic modeling of the sea floor," JASA 68(5), 1980
- Kouyoumjian & Pathak, "A UTD for perfectly conducting wedges," IEEE TAP, 1974
- ISO 3382-1:2009 — Room acoustics measurement
- ISO 9613-1:1993 — Atmospheric absorption of sound
- ISO 9613-2:1996 — Outdoor sound propagation
- IEC 60268-16:2020 — Speech Transmission Index
- Botteldooren, "Finite-difference time-domain simulation of low-frequency room acoustic problems," JASA 98(6), 1995
- Gilbert & Di, "A fast Green's function method for one-way sound propagation in the atmosphere," JASA 94(4), 1993
