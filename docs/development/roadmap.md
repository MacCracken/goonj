# Goonj Roadmap

## Current Release

**v1.1.0** — Tiers 1–3 complete, P(-1) hardened, underwater acoustics + vibroacoustics added.

- 388 tests, 28 benchmarks, 30 modules
- All formulas verified against ISO/IEC standards and peer-reviewed references
- Supply chain clean

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

---

## v1.2.0 — Wave-Based Methods + Emerging Algorithms

Implementable now — algorithms are well-defined with clear references.

### Wave Solvers
- [ ] **2D FDTD modal solver** — explicit finite-difference time-domain below Schroeder frequency. Textbook algorithm (Botteldooren 1995). Plugs into hybrid crossover interface.
- [ ] **Digital Waveguide Mesh** — FDTD variant with waveguide interpretation for room simulation. Reference: Wayverb (reuk), Smith (Stanford CCRMA).

### Reverb Synthesis
- [ ] **Dark Velvet Noise reverb** — non-exponential decay modeling using sparse stochastic sequences. 4% RT60 error, 50% fewer filters. Reference: Fagerström et al., JAES 72(6), 2024.

### Outdoor Propagation
- [ ] **GFPE terrain propagation** — Green's Function Parabolic Equation for range-dependent outdoor environments with hills/ridges. Reference: Gilbert & Di (1993).

### Material Extensions
- [ ] **Acoustic metamaterial types** — frequency-dependent negative-stiffness and negative-density material models for engineered absorbers. Lookup-table approach from manufacturer data.

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
