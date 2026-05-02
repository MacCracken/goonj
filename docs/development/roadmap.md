# Goonj Roadmap

## Current Release

**v1.4.2** — Second rung of the v1.4.x ladder: per-band frequency-dependent IIR boundary filters in DWM. Public API unchanged from v1.4.1; boundary reflections now carry frequency dependence fitted from `AcousticMaterial.absorption[band]`.

- 513 tests (+6), 33 benchmarks, 34 modules
- 1-pole IIR `H(z) = b0 / (1 − a1·z⁻¹)` per face; coefficients fitted so |H(0)| = R[63 Hz], |H(π)| = R[8 kHz]
- Per-cell filter state on each face (~15 KB total for a 30×25×20 grid)
- DWM bench 68.8 ms → 72.9 ms (+6% for the per-cell multiply-add)
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

### v1.4.0 — Digital Waveguide Mesh (MVP)
- **dwm core, rigid walls** — Smith / Van Duyne–Smith K=6 scattering junctions on a 3D Cartesian lattice. `DwmConfig` / `DwmSource` / `DwmReceiver` / `DwmResult` / `solve_dwm_3d` / `required_dx`. Lattice constraint `Δx = c·Δt·√3` enforced (warn at >1%, refuse at >10%). Rigid Neumann walls reflect a node's own outgoing wave back. Modal-frequency test validates first axial mode against `room_mode`. 76 ms for 30×25×20 × 1102 steps. (Bite 1 of v1.4.0.)
- **dwm scalar absorbing walls** — `DwmConfig::wall_absorption: f32` (`R = √(1 − α)` boundary reflection); `with_acoustic_material(&mat)` builder pulls `average_absorption()`. RT60-ordering test confirms carpet < concrete < rigid. No bench regression. (Bite 2 of v1.4.0.)

### v1.4.1 — Per-wall material assignment (BREAKING)
- **dwm: WallMaterials** — `DwmConfig.wall_absorption: f32` replaced by `wall_materials: WallMaterials` (named struct with `x_neg`/`x_pos`/`y_neg`/`y_pos`/`z_neg`/`z_pos` fields, each an `AcousticMaterial`). `WallMaterials::uniform` / `::rigid` constructors; new `with_wall_materials(walls)` builder. Solver precomputes 6 reflection coefficients per face. Asymmetric-walls test verifies per-face routing. Bench dropped from 76 ms → 68.8 ms (branch-free inner loop).

### v1.4.2 — Per-band frequency-dependent impedance walls
- **dwm: per-face 1-pole IIR boundary filter** — scalar reflection per face replaced by `H(z) = b0/(1 − a1·z⁻¹)` fitted so |H(0)| matches `R[63 Hz]` and |H(π)| matches `R[8 kHz]`: `a1 = (R_low − R_high)/(R_low + R_high)` clamped to `(−0.99, 0.99)`, `b0 = R_low·(1 − a1)`. Per-cell filter state on each face. Carpet-like materials → low-pass IIR (`a1 > 0`); glass-like → high-pass (`a1 < 0`). Public API unchanged. Bench 68.8 ms → 72.9 ms (+6% per-cell multiply-add).

---

## v1.4.x — Remaining ladder

Each rung is independently shippable. Any rung can be skipped if Cyrius reaches its readiness gate first; the ladder runs only as long as the port isn't ready. No new initiatives outside this list.

### v1.4.3 — Dispersion correction
- [x] Dispersion characterization helpers `mesh_frequency` and `dispersion_factor` for the 3D rectilinear lattice + a 2-tap FIR `DispersionCorrection` calibrated to boost the half-mesh frequency by a configurable amount (default 5%) without DC gain change. First-order only; paper-faithful Savioja IDWM phase equalization deferred until consumer demand.

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
