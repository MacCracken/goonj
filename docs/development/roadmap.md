# Goonj Roadmap

## Current Release

**v1.3.0** — Wave-based methods + emerging algorithms. Four new modules: Dark Velvet Noise reverb, acoustic metamaterials, GFPE outdoor terrain propagation, 2D FDTD modal solver.

- 479 tests (+52), 32 benchmarks (+4), 33 modules (+4)
- All four v1.3.0 roadmap items shipped, each as its own commit through the work loop
- Digital Waveguide Mesh carved out into v1.4.0 (next release)
- All formulas validated against peer-reviewed references (Botteldooren 1995, Gilbert & Di 1993, Fagerström 2024, Liu/Fang/Yang/Lee for metamaterials)
- All six cleanliness gates clean (fmt, clippy, test, audit, deny, doc)

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

### v1.3.0 — Wave-Based Methods + Emerging Algorithms
- **dark_velvet_noise** — Fagerström 2024 sparse-pulse late-reverb synthesis. Exponential or piecewise-dB decay envelope, time-varying low-pass coloration. RT60 within 4% of target. 165 μs for a 2 s 48 kHz IR (~290× real-time).
- **metamaterial** — engineered acoustic materials with Drude–Lorentz `LorentzianResonance` (`NegativeStiffness` / `NegativeDensity` / `DoublyNegative`) plus a `LookupTable` for manufacturer data. 8-band absorption from bulk-impedance approximation. 211 ns per band sweep.
- **gfpe** — Gilbert & Di 1993 outdoor parabolic-equation solver over range-dependent terrain. Linear sound-speed gradient + Miki ground impedance + staircase terrain. 374 μs for a 21 × 30 grid at 500 Hz.
- **fdtd** — Botteldooren 1995 2D FDTD modal solver with rigid Neumann walls, CFL-enforced time step, Goertzel `band_energies` helper for `hybrid::blend_results` integration. 1.13 ms for a 40 × 40 grid × 1102 steps at 22.05 kHz.

---

## v1.4.0 — Digital Waveguide Mesh (MVP)

Carved out from v1.3.0 because it's the largest of the wave-based items. Scope intentionally simplified at the top so deferred work can land cleanly across the v1.4.x ladder below.

### Wave Solvers
- [x] **3D rectilinear Digital Waveguide Mesh — core, rigid walls** — Smith / Van Duyne–Smith K=6 scattering junction on a Cartesian grid. New `src/dwm.rs` with `DwmConfig` / `DwmSource` / `DwmReceiver` / `DwmResult` / `solve_dwm_3d` / `required_dx`. Rigid Neumann walls. Plugs into `hybrid::blend_results` via re-exported `fdtd::band_energies`. (Bite 1 of v1.4.0.)
- [x] **DWM scalar absorbing walls** — uniform absorption coefficient on `DwmConfig`; boundary reflection `R = √(1 − α)`; `DwmConfig::with_acoustic_material(&mat)` builder helper. (Bite 2 of v1.4.0.)

---

## v1.4.x — Deferred-from-v1.4.0 ladder

Each rung is independently shippable. Any rung can be skipped if Cyrius reaches its readiness gate first; the ladder runs only as long as the port isn't ready. No new initiatives outside this list.

### v1.4.1 — Per-wall material assignment
- [ ] Replace `wall_absorption: f32` with `wall_materials: [AcousticMaterial; 6]` (one per ±x/±y/±z face). Each face pulls its own `average_absorption()`. Mechanical extension; same algorithm.

### v1.4.2 — Per-band frequency-dependent impedance walls
- [ ] Replace scalar reflection per face with a 1-pole IIR reflection filter, coefficients least-squares-fit to `AcousticMaterial.absorption[band]` across the 8 ISO octave centres. One filter state per wall face. The DWM-over-FDTD design dividend.

### v1.4.3 — Dispersion correction
- [ ] Frequency pre/post-warp (Savioja IDWM) to compensate the ~5% directional dispersion error near the mesh frequency on the 3D rectilinear lattice.

### v1.4.x+ — Triangular / hexagonal mesh variants (consumer-demand-gated)
- [ ] Lower-anisotropy mesh topologies. K=12 hexagonal close-packed in 3D. Ship only on demand from a downstream consumer (dhvani, kiran, shruti).

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
