# Changelog

## [Unreleased]

### 2.0.0 — Cyrius port (in progress)

Major break: goonj is being rewritten from Rust to **Cyrius** (sovereign
systems language, compiled by `cycc`). The 1.4.3 Rust source is frozen at
`rust-old/` as the parity oracle. Parity progress is tracked in
[`docs/development/port-audit.md`](docs/development/port-audit.md); the
milestone plan is in [`docs/development/roadmap.md`](docs/development/roadmap.md).

#### Added
- **Cyrius scaffold** — `cyrius port` layout: `src/main.cyr` smoke binary,
  per-module library sources, `cyrius.cyml` manifest (toolchain pin 6.3.12,
  stdlib + `math`/`ganita`, hisab 2.6.7 distlib dep), `cyrius.lock`.
- **error** — `src/error.cyr`. Integer error codes replace the `GoonjError`
  enum (`ERR_*` + `goonj_is_err`/`goonj_err_name`); shared `GOONJ_EPSILON`
  and `F64_NEG_INF`.
- **propagation** — `src/propagation.cyr`. Scalar core (speed of sound,
  inverse-square law, SPL↔pressure, ISO 9613-1 atmospheric absorption,
  Doppler, Miki 1990 ground reflection), the `HVec3` wind/temperature
  profiles (`refracted_speed`, `speed_at_height`), and the Snell-law
  atmospheric ray tracers (`refract_ray_step`, `trace_ray_atmospheric`).
  44 parity assertions in `tests/propagation.tcyr`, all green. Establishes
  the closure → fn-pointer + context-pointer idiom (`fncall2` + `TraceCtx`,
  stdlib `fnptr`) and the tuple→struct return (`RayStep`).
- **material** — `src/material.cyr`. `AcousticMaterial` (7 named tables,
  validated constructor, average/per-band absorption), `WallConstruction`
  (mass-law transmission loss + coefficient), and the `JcalMaterial` JCAL
  porous-absorber model. 65 parity assertions in `tests/material.tcyr`, all
  green. Establishes the manual-struct-layout (inline arrays) and string-
  field idioms for the rest of the port.
- **resonance** — `src/resonance.cyr`. `room_mode`, `axial_modes`/
  `all_axial_modes` (mode lists via stdlib `vec` + f64 insertion sort),
  `schroeder_frequency`, `modal_density`. 15 parity assertions in
  `tests/resonance.tcyr`, all green. Establishes the `Vec<T>` dynamic-array
  idiom for the rest of the port.

#### Changed (port-wide conventions)
- **f32 → f64** throughout — hisab's `HVec3` is f64-only.
- **Rust enums with String payloads → integer error codes**.
- **`hisab::Vec3` → hisab `HVec3`** (consumed via the `dist/hisab.cyr` bundle).

#### Breaking
- Entire public API moves from Rust to Cyrius. Rust consumers stay on the
  1.4.x line; Cyrius consumers target 2.0.0. Migration is per-module as the
  port lands — see the port audit.

## [1.4.3] - 2026-05-01

Third rung of the v1.4.x ladder: dispersion characterization and a
first-order correction for the 3D rectilinear DWM. Public additions
only — the solver itself is untouched. After this rung the only
remaining ladder item is consumer-demand-gated (triangular meshes).

### Added
- **dwm: dispersion characterization + first-order correction** —
  three new public items in `goonj::dwm`:
  - `mesh_frequency(&DwmConfig) -> f32` — the `c/(2·Δx)` upper-limit
    frequency above which the 3D rectilinear lattice can't resolve waves.
  - `dispersion_factor(&DwmConfig, frequency_hz) -> f32` — returns
    `f_sim / f_true` for axis-aligned propagation by inverting the
    dispersion relation `sin(ω_sim·Δt/2) = sin(ω_true·Δt·√3/2) / √3`;
    `1.0` at DC, drops monotonically toward the mesh frequency, `0.0`
    above it.
  - `DispersionCorrection { b0, b1 }` — a 2-tap FIR
    `y[n] = b0·x[n] + b1·x[n−1]` calibrated to keep DC gain at 1.0 and
    boost magnitude by a target factor at the half-mesh frequency.
    Constructors `for_config(&DwmConfig)` (default 5% boost),
    `calibrated(&DwmConfig, boost)` (custom), and `passthrough()`
    (identity). `apply(&self, &mut [f32])` runs the FIR in-place,
    iterating backwards so `x[i−1]` isn't clobbered by the
    just-written `y[i]`.

  Documented as a *first-order* correction: it boosts magnitudes at the
  upper octaves to compensate the dominant low-pass tilt of DWM
  dispersion. A paper-faithful Savioja IDWM all-pass phase equalizer is
  out of scope for v1.4.3 — `dispersion_factor` is provided so callers
  who need finer correction can design their own filter.

  Tests: mesh-frequency endpoint, dispersion factor at DC / mid-mesh /
  above-mesh, FIR DC gain unity, FIR boosts target frequency by the
  calibrated amount (≤5% error), passthrough identity, empty / single-
  sample signals, serde roundtrip. Benchmark
  `dwm/dispersion_correct_22kHz_1s`: 11.2 μs for a 22050-sample buffer
  (~2 GFLOPS effective).

### Stats
- 34 modules, 34 benchmarks (+1: `dwm/dispersion_correct_22kHz_1s`),
  **525 tests** passing (518 unit + 6 integration + 1 doc) — up from
  513 at v1.4.2
- DWM solver bench unchanged at ~72 ms — the correction is a separate
  post-process and doesn't touch the inner loop
- All six gates clean: `cargo fmt --check`,
  `cargo clippy --all-features --all-targets -- -D warnings`,
  `cargo test --all-features`, `cargo audit`, `cargo deny check`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps`

## [1.4.2] - 2026-05-01

Second rung of the v1.4.x ladder: per-band IIR boundary filters in DWM.
Behavioural upgrade only — the public API surface is unchanged from
v1.4.1, but boundary reflections now carry frequency dependence
fitted from `AcousticMaterial.absorption[band]` instead of collapsing
to a single average. The DWM-over-FDTD design dividend.

### Changed
- **dwm: per-band frequency-dependent impedance walls** — DWM boundary
  reflection upgrades from a scalar `R = √(1 − ᾱ)` to a per-face 1-pole
  IIR filter `H(z) = b0 / (1 − a1·z⁻¹)` fitted so that |H(0)| matches
  the material's reflection at 63 Hz and |H(π)| matches it at 8 kHz:
  `a1 = (R_low − R_high) / (R_low + R_high)`, `b0 = R_low · (1 − a1)`,
  with `a1` clamped to `(−0.99, 0.99)` for stability. Intermediate
  octave bands are interpolated by the IIR's natural frequency
  response — not least-squares-fit across all 8, but the 2-parameter
  fit captures the dominant low-vs-high tilt and is robust at endpoints
  (rigid → identity, fully-absorbing → zero).

  Carpet-like materials (high α at high freq, low α at low freq) yield
  `a1 > 0` (low-pass: high-freq content absorbed, low-freq content
  reflected). Glass-like materials (high α at low freq, low α at high
  freq) yield `a1 < 0` (high-pass: low-freq content absorbed). Tests
  `boundary_filter_carpet_attenuates_high_freq_more` and
  `boundary_filter_glass_attenuates_low_freq_more` validate both cases.

  Per-cell filter state is allocated per face (`ny·nz`, `nx·nz`, or
  `nx·ny` f32 values per face); ~15 KB total for a 30×25×20 grid,
  negligible. Source-side `DwmConfig` API is unchanged — same
  `wall_materials: WallMaterials` field, same builders. The behavioural
  upgrade is opt-in by virtue of using a material with non-flat
  band absorption.

  No regressions on the existing modal-frequency or asymmetric-walls
  tests. Bench `dwm/solve_30x25x20_50ms`: 72.9 ms (was 68.8 ms in
  v1.4.1; the per-cell IIR is one extra multiply-add per boundary cell
  per step, ~6% added cost for full per-band physics).

### Stats
- 34 modules, 33 benchmarks, **513 tests** passing (506 unit + 6
  integration + 1 doc) — up from 507 at v1.4.1
- Per-band filter validated: rigid → identity, fully-absorbing → zero,
  carpet → low-pass IIR (`a1 > 0`), glass → high-pass IIR (`a1 < 0`),
  all preset materials yield poles within the stable `(−0.99, 0.99)` range
- All six gates clean: `cargo fmt --check`,
  `cargo clippy --all-features --all-targets -- -D warnings`,
  `cargo test --all-features`, `cargo audit`, `cargo deny check`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps`

## [1.4.1] - 2026-05-01

First rung of the v1.4.x ladder. Per-wall material assignment lands as a
single-bite breaking change to `DwmConfig`. No new modules, no algorithm
changes — just the shape of the boundary input. Migration path
documented inline below.

### Changed
- **dwm: per-wall material assignment (BREAKING)** — `DwmConfig` field
  `wall_absorption: f32` is replaced by `wall_materials: WallMaterials`,
  a new struct with one `AcousticMaterial` per face (`x_neg`, `x_pos`,
  `y_neg`, `y_pos`, `z_neg`, `z_pos`). Each face's reflection amplitude
  is `R = √(1 − ᾱ)` where ᾱ is that face's `average_absorption()`,
  clamped to `[0, 1]`. The solver precomputes the six reflection
  coefficients before the time loop, so the inner loop cost is
  unchanged (one multiply per boundary cell, same as v1.4.0).

  `WallMaterials::uniform(material)` clones a single material to all
  six faces. `WallMaterials::rigid()` constructs zero-absorption walls
  (the new `Default`). Builder helpers: `with_acoustic_material(&mat)`
  routes through `WallMaterials::uniform`; new
  `with_wall_materials(walls)` accepts a fully-populated struct.

  Tests `asymmetric_walls_drain_more_than_all_concrete` and
  `boundary_reflection_endpoints` (R=1 at α=0, R=0 at α=1) confirm
  per-face routing and the energy-correct convention. Out-of-range
  absorption (e.g. direct field-construct an `AcousticMaterial` with
  α > 1, bypassing `AcousticMaterial::new` validation) is clamped
  silently inside `boundary_reflection`.

  Bench `dwm/solve_30x25x20_50ms`: 68.8 ms (was 76 ms in v1.4.0;
  precomputing the six reflection coefficients is marginally cheaper
  than the old single-coef-with-branches path).

  Migration: replace `wall_absorption: 0.0` with
  `wall_materials: WallMaterials::rigid()`. Replace `wall_absorption: α`
  with `wall_materials: WallMaterials::uniform(material_with_alpha)` or
  build a `WallMaterials` directly. The `with_acoustic_material(&mat)`
  helper signature is unchanged.

### Stats
- 34 modules, 33 benchmarks, **507 tests** passing (500 unit + 6
  integration + 1 doc) — up from 503 at v1.4.0
- All formulas validated against Smith CCRMA, Van Duyne & Smith 1993,
  Wayverb (reuk)
- All six gates clean: `cargo fmt --check`,
  `cargo clippy --all-features --all-targets -- -D warnings`,
  `cargo test --all-features`, `cargo audit`, `cargo deny check`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps`
- DWM bench `dwm/solve_30x25x20_50ms`: 68.8 ms (was 76 ms at v1.4.0;
  precomputing six reflection coefficients trims the inner-loop
  branches the v1.4.0 single-coef path had)

## [1.4.0] - 2026-05-01

3D wave-based room acoustics via Digital Waveguide Mesh. Single new
module (`dwm`) shipped across two bites — core solver with rigid walls,
then scalar absorbing walls + `AcousticMaterial` bridge. Carved out from
v1.3.0 because DWM is the largest of the wave-based items and warranted
its own release cycle. Per-wall material assignment, per-band impedance
walls, and dispersion correction are planned as the v1.4.x ladder (see
`docs/development/roadmap.md`); the user will green-light each rung
explicitly, none are committed in advance.

### Added
- **dwm: scalar absorbing walls** — `DwmConfig` gains a `wall_absorption: f32`
  field (`0.0` = rigid Neumann, `1.0` = fully absorbing) applied uniformly
  at every face. Boundary update now scales the reflected outgoing wave
  by `R = √(1 − α)` so reflected-energy fraction equals `1 − α` per the
  standard acoustics convention. New builder helper
  `DwmConfig::with_acoustic_material(&AcousticMaterial)` pulls
  `material.average_absorption()` (mean of the 8 ISO octave-band
  coefficients) and clamps to `[0, 1]`. Out-of-range `wall_absorption`
  values are clamped silently. Tests confirm absorbing walls reduce
  field energy vs. rigid (carpet < concrete < rigid by RT60-ordering).
  No bench regression — boundary scaling is one multiply per boundary
  cell, ~5% of nodes. (Bite 2 of v1.4.0; v1.4.1 will replace the scalar
  with `[AcousticMaterial; 6]` for per-wall material assignment.)

- **dwm** — 3D rectilinear Digital Waveguide Mesh per Smith / Van Duyne &
  Smith. New module exposes `DwmConfig`, `DwmSource` (with `gaussian_pulse`
  constructor), `DwmReceiver`, `DwmResult`, `solve_dwm_3d`, and
  `required_dx(sample_rate, c)` helper. Each grid node is a `K = 6`-port
  scattering junction (±x ±y ±z); outgoing-wave buffers are rotated each
  step in a `node × K` flat layout. Pressure `p = (2/K)·Σ incoming`,
  outgoing `out_i = p − in_i`. Rigid Neumann walls implemented by
  reflecting a node's own outgoing wave back as the incoming wave at
  boundary faces.

  The 3D rectilinear lattice imposes `Δx = c · Δt · √3`. The solver
  validates the user's `dx` against this — `tracing::warn` if deviation
  exceeds 1%, empty result if it exceeds 10%. Source is transparent
  (additive into junction pressure); a hybrid-crossover convenience
  `dwm::band_energies` re-exports `fdtd::band_energies` so DWM plugs into
  `hybrid::blend_results` exactly like FDTD does.

  Modal-frequency validation: a 1 m³ rigid box's first axial mode at
  c/(2L) ≈ 171.5 Hz lands in the 63–500 Hz octave bands as expected
  (test uses a band-limited Gaussian to avoid pumping the lossless
  mesh-frequency dispersion modes). Memory bounded by
  `MAX_GRID_CELLS = 4·10⁶`. Benchmark `dwm/solve_30x25x20_50ms`: 76 ms
  for a 15 000-node grid × 1102 time steps at 22.05 kHz (~11 GFLOPS
  effective). References: Smith (Stanford CCRMA), Van Duyne & Smith
  ICMC 1993, Wayverb (reuk).

### Stats
- 34 modules (1 new: `dwm`), 33 benchmarks (1 new: `dwm/solve_30x25x20_50ms`),
  **503 tests** passing (496 unit + 6 integration + 1 doc) — up from
  479 at v1.3.0
- All formulas validated against peer-reviewed references (Smith CCRMA
  online, Van Duyne & Smith 1993, Wayverb)
- All six gates clean: `cargo fmt --check`,
  `cargo clippy --all-features --all-targets -- -D warnings`,
  `cargo test --all-features`, `cargo audit`, `cargo deny check`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps`
- No regressions on existing benches — DWM bench held at ~77 ms across
  bites 1 and 2 (boundary scaling cost negligible)

## [1.3.0] - 2026-05-01

Wave-based methods + emerging algorithms. Four new modules — Dark Velvet
Noise reverb, acoustic metamaterials, GFPE outdoor terrain propagation,
and a 2D FDTD modal solver — each landed as its own commit through the
work loop. Digital Waveguide Mesh was carved out into v1.4.0.

### Added
- **fdtd** — 2D explicit finite-difference time-domain solver per
  Botteldooren 1995. New types: `FdtdConfig`, `FdtdSource` (with
  `gaussian_pulse(...)` constructor), `FdtdReceiver`, `FdtdResult`
  (per-receiver pressure traces + final pressure-field snapshot).
  Standard 5-point Laplacian + leapfrog update, rigid (Neumann) walls
  via single-row mirror. CFL `c·Δt/Δx ≤ 1/√2` enforced — solver
  early-returns empty on violation rather than diverging silently.
  Memory bounded by `MAX_GRID_CELLS = 4·10⁶` and `MAX_TIME_STEPS = 10⁶`.
  Plugs into `hybrid::blend_results` via the new `band_energies(signal,
  sample_rate)` helper, which evaluates Goertzel at each ISO octave
  centre to yield a `[f32; NUM_BANDS]` energy vector. Modal-frequency
  validation: a 2 m × 2 m rigid box's first axial mode (c/(2L) ≈ 86 Hz)
  lands in the 63–250 Hz bands as expected. Benchmark
  `fdtd/solve_40x40_50ms`: 1.13 ms (40×40 grid × 1102 time steps at
  22.05 kHz). Reference: Botteldooren, JASA 98(6), 1995.
- **gfpe** — Green's Function Parabolic Equation outdoor solver per Gilbert
  & Di 1993. Marches range by Δr in a 2D vertical (range × height) slice
  using direct (O(N²)) convolution against the parabolic-equation
  Green's function plus an image-source ground term. New types: `GfpeConfig`,
  `GfpeAtmosphere`, `GfpeTerrain` (range vs. ground-elevation polyline,
  linearly interpolated), `GfpeResult` (excess-attenuation field as
  row-major `Vec<f32>`). Inputs: source frequency / height, max range,
  max height, range/height steps, `propagation::GroundImpedance` (Miki
  complex form computed inline at a representative grazing angle), and
  a linear sound-speed gradient. Terrain handled as a staircase that
  zeros the field below the local ground elevation each range step —
  simple but captures the dominant "shadow behind hill" effect (test
  `hill_blocks_field_below_crest` asserts +∞ excess attenuation in the
  shadow column). Output is excess attenuation (dB) relative to 2D
  cylindrical free-field reference. Memory bounded by `MAX_GRID_CELLS`
  (10 M cells). Benchmark `gfpe/solve_100m_30m_500hz`: 374 μs for a
  21-range × 30-height grid. Reference: Gilbert & Di, JASA 94(4), 1993.
- **metamaterial** — engineered acoustic materials with frequency-dependent
  effective parameters. Three analytical models — `NegativeStiffness`,
  `NegativeDensity`, `DoublyNegative` — driven by a Drude–Lorentz
  `LorentzianResonance` (centre, plasma, damping, background_value) plus
  a `LookupTable` form for manufacturer absorption datasheets. Each
  variant produces an 8-band absorption profile via a bulk-impedance
  approximation `α = 1 − |R|²` with `R = (Z_eff − Z₀)/(Z_eff + Z₀)` and
  `Z_eff = √(ρ_eff·K_eff)` against air at Z₀ ≈ 413 rayls. Convenience
  constructor `LorentzianResonance::with_dc_value` lets callers specify
  the DC value directly instead of the canonical Drude–Lorentz
  background. Outputs convert to `AcousticMaterial` via
  `to_acoustic_material(name, scattering)` so a metamaterial panel
  plugs into the existing ray / IR pipeline as a wall. Benchmark
  `metamaterial/absorption_bands_negstiff`: 211 ns. References:
  Liu 2000, Fang 2006, Yang 2008, Lee 2010.
- **dark_velvet_noise** — Fagerström et al. 2024 reverb synthesis from
  sparse stochastic pulse sequences. New module exposes `DvnConfig`,
  `DecayEnvelope::{Exponential, PiecewiseDb}`, and
  `synthesize_dvn_ir(&config) -> Vec<f32>`. Supports non-exponential
  decay shapes (e.g. coupled-room double-slope) via piecewise-linear-dB
  breakpoints and "dark" tail coloration via a time-varying 1-pole
  low-pass interpolated from `coloration_initial_cutoff_hz` to
  `coloration_final_cutoff_hz`. RT60 recovery within the paper's 4%
  budget (regression test asserts `< 4%`). Benchmark
  `dvn/synthesize_2s_2khz_density`: 165 μs for a 2-second 48 kHz IR
  (~290× real-time, ~15× faster than `impulse::generate_ir`).

### Stats
- 33 modules (4 new), 32 benchmarks (4 new), **479 tests** passing
  (472 unit + 6 integration + 1 doc) — up from 427 at v1.2.1
- All formulas validated against peer-reviewed references (Botteldooren
  1995, Gilbert & Di 1993, Liu 2000 / Fang 2006 / Yang 2008 / Lee 2010,
  Fagerström 2024)
- All six gates clean: `cargo fmt --check`,
  `cargo clippy --all-features --all-targets -- -D warnings`,
  `cargo test --all-features`, `cargo audit`, `cargo deny check`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps`
- No regressions on existing benches — STI 3.62 ms, image-source/diffuse
  rain / IR generation all within noise of v1.2.1

## [1.2.1] - 2026-05-01

Dependency bump release. Picks up the **hisab v1.0** stable line (we had
been running on the pre-v1 `0.24` series since project inception) and
brings the rest of the supply chain to current. Added supply-chain
checks to the cleanliness gates.

### Changed
- **deps** — `hisab 0.24 → 1.4` (first stable major). No goonj-side API
  changes required: every `hisab::Vec3`, `hisab::geo::Bvh`, and
  `hisab::geo::Ray` call site recompiles unchanged against v1.4.
- **deps** — `criterion 0.5 → 0.8`. Replaced the now-deprecated
  `criterion::black_box` import with `std::hint::black_box` in
  `benches/benchmarks.rs` (criterion 0.8 removes the re-export).
- **deps** — pinned minimum patch versions: `serde 1.0.228`,
  `thiserror 2.0.18`, `tracing 0.1.44`, `tracing-subscriber 0.3.23`,
  `serde_json 1.0.149`. No behaviour change; tightens the floor so
  `cargo audit` runs against a known-good baseline.

### Added
- **CI/dev gates** — `cargo audit` and `cargo deny check` are now part
  of the documented P(-1) and Working-Loop cleanliness sweeps. Both run
  clean on the new lockfile (91 transitive crates, 0 advisories,
  licenses ok, bans ok, sources ok). The existing `deny.toml` allowlist
  (GPL-3.0-only, MIT, Apache-2.0, Unicode-3.0) covers the v1.4 hisab
  tree without modification.

### Stats
- 411 tests passing (404 unit + 6 integration + 1 doc), 28 benchmarks
- All benchmarks hold within noise vs. v1.2.0 (e.g. `analysis/sti`
  3,624,973 ns → 3,621,627 ns) — no regression from the dep bump
- All six gates clean: `cargo fmt --check`,
  `cargo clippy --all-features --all-targets -- -D warnings`,
  `cargo test --all-features`, `cargo audit`, `cargo deny check`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps`

## [1.2.0] - 2026-05-01

Final P(-1) scaffold-hardening pass before opening v1.3 feature work. No
new public API; all changes are internal correctness / performance fixes.

### Performance
- **analysis** — `sti_estimate`: hoisted the broadband MTI computation out
  of the per-band loop. The same broadband IR was being reduced 7× with
  identical inputs and outputs; the result is now computed once and
  broadcast to `band_mti`. Benchmark: `analysis/sti` 28,751,483 ns →
  3,624,973 ns (−87%, **~7.9× faster**). Formula identity preserved:
  `Σαₖ·MTI − Σβₖ·√(MTIₖ·MTIₖ₊₁)` collapses to `(Σαₖ − Σβₖ)·MTI` when
  all bands share a value, exactly as before.

### Fixed
- **diffusion** — `solve_diffusion_2d` now also early-returns when
  `speed_of_sound <= 0.0`. Previously a zero/negative speed propagated
  into `d_coeff = c · mfp / 3`, leaving `dt_max = dx² / 0` to flow
  through `min(config.dt)` and producing a degenerate-but-running
  simulation. Added a regression test.
- **integration/dhvani** — removed dead `_surface_area` binding in
  `generate_dhvani_ir`; per-band Sabine RT60 already pulls absorption
  area directly from `wall.area()`.
- **scripts/bench-history.sh** — pass `--all-features` so feature-gated
  benchmarks (`wav/export_48k_2s`, `binaural/generate_ir_shoebox`)
  actually compile. The script silently failed on these benches after
  those features were added in v0.2.0 / v1.1.0.

### Stats
- 411 tests passing (404 unit + 6 integration + 1 doc), 28 benchmarks, 30 modules
- New regression test: `diffusion::tests::diffusion_zero_speed_of_sound_returns_empty`
- All formulas remain verified against ISO/IEC and peer-reviewed sources
- `cargo fmt`, `cargo clippy --all-features --all-targets -- -D warnings`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps` all clean

## [1.1.1] - 2026-03-29

### Changed
- **license** — GPL-3.0 → GPL-3.0-only across Cargo.toml, README, CLAUDE.md, deny.toml

## [1.1.0]

### Added
- **bridge** — cross-crate primitive-value bridges for pavan (wind attenuation, Doppler, effective speed of sound), badal (air absorption from weather, speed of sound, pressure conversion), ushma (absorption temperature scaling, sound speed gradient), bijli (EM resonance coupling, piezoelectric acoustic power)
- **underwater** (feature-gated) — Mackenzie ocean sound speed, Francois-Garrison seawater absorption, Hamilton seabed sediment model (sand/silt/clay/rock), Rayleigh bottom reflection, Eckart sea surface scattering

### Tier 1–3 Accuracy & Standards
- **8 octave bands** (63–8000 Hz), full ISO 9613-1 atmospheric absorption, Miki ground impedance
- **IEC 60268-16:2020 STI** with correct α_k/β_k redundancy weights
- **ISO 3382-1**: EDT, G, ts, LF, IACC, octave-band filtering
- **Fitzroy RT60** + Kuttruff correction, full UTD wedge diffraction (K-P 1974)

### Tier 2 Features
- Wall transmission (mass law + Davy), source directivity, portal propagation
- 1st/3rd-order Ambisonics (SN3D/ACN), coupled rooms, vector scattering
- FDN reverb (zero-allocation Householder), JCAL porous materials

### Tier 3 Features
- Beam tracing, acoustic radiosity, 2D diffusion equation solver
- ISO 9613-2 outdoor (barrier, foliage, meteorological, ground)
- Hybrid frequency crossover, UDFA filter-based diffraction

### Correctness (P(-1) hardening)
- Ambisonics ACN 10/14 SN3D factor: `sqrt(15/2)/2` → `sqrt(15)/2`
- Coupled rooms: amplitude rate (6.908) → energy rate (13.816) for gamma
- STI: correct IEC 60268-16:2020 formula `STI = Σ(α_k×MTI_k) − Σ(β_k×√(MTI_k×MTI_{k+1}))`
- C_met: `h_avg` → `h_s + h_r`, removed spurious squared factor
- UTD Fresnel argument: fixed N± computation per K-P 1974
- Miki model: corrected coefficients to 0.0699 / -0.1071
- FDN: zero-allocation `process_sample` (pre-allocated scratch buffer)
- Portal: two-stage inverse square `A/(4π×d₁²×d₂²)`

### Security
- Capped image-source max_order (20 shoebox / 6 general)
- Capped diffusion grid (2000×2000), radiosity patches (100/wall)
- Overflow-safe WAV data size, IR generation capped at 10min@192kHz

### Stats
- 378 tests, 28 benchmarks, 29 modules
- All formulas verified against references (ISO 9613-1, IEC 60268-16:2020, Miki 1990, K-P 1974, Mackenzie 1981, Francois-Garrison 1982, Hamilton 1980)

## [1.0.0] - 2026-03-25

Security hardening, correctness fixes, and documentation pass for v1.0.0 freeze.

### Security
- Capped `max_order` to 20 (shoebox) / 6 (general) to prevent O(n³) / O(W^N) DoS
- Added overflow-safe WAV data size calculation (`checked_mul` + `try_from`)
- Capped IR generation to 10min @ 192kHz max to prevent OOM
- Capped atmospheric trace to 1M iterations to prevent infinite loops

### Correctness
- Fixed `eyring_rt60` at absorption=1.0 → returns 0 (anechoic), not infinity
- Added module-level docs on `ray.rs`
- Documented shoebox normal convention (outward-facing)
- Added capacity hints for image source, atmospheric trace Vecs

### Documentation
- Zero `missing_docs` warnings — all public types, functions, fields documented
- Architecture overview updated with data flow diagram and feature flag table
- Roadmap expanded with 30 research-backed future items across 4 tiers

### Stats
- 240 tests, 28 benchmarks, 97.58% coverage
- All v1.0.0 criteria met

## [0.2.0] - 2026-03-25

Full roadmap implementation: frequency-dependent ray tracing, IR generation, room analysis, and downstream integration.

### New Modules
- **image_source** — Allen & Berkeley image-source method for exact early specular reflections (shoebox + general rooms)
- **diffuse** — Stochastic ray tracing (diffuse rain) for late reverb tails with Fibonacci sphere distribution
- **analysis** — Room acoustics metrics: C50, C80, D50, STI estimation, absorption placement suggestions
- **wav** (feature-gated) — 16-bit PCM WAV export for mono and stereo impulse responses
- **binaural** (feature-gated) — Binaural IR generation with user-provided HRTF datasets
- **integration/dhvani** (feature-gated) — IR handoff for convolution reverb
- **integration/kiran** (feature-gated) — Real-time occlusion queries with BVH acceleration
- **integration/soorat** (feature-gated) — Visualization data structures (ray paths, pressure maps, mode patterns)

### Enhanced Modules
- **ray** — MultibandRay with per-band energy [f32; 6], RayBounce, RayPath, trace_ray scene tracer, BVH-accelerated trace_ray_bvh
- **room** — Wall::aabb(), RoomGeometry::build_bvh(), AcceleratedRoom with cached BVH
- **impulse** — IrConfig, MultibandIr, generate_ir() combining image-source + diffuse rain
- **propagation** — WindProfile, TemperatureProfile, GroundImpedance, atmospheric ray tracing with Snell's law refraction, Delany-Bazley ground reflection

### Audit Hardening
- Removed all `unwrap()` from library code (replaced with `is_none_or`)
- Added `#[inline]` on hot-path functions: ray intersection, reflection, PRNG — 9–18% speedup
- Eliminated 384KB allocation in STI estimation (compute h² on-the-fly)
- Added Vec pre-allocation with capacity hints
- Added edge-case guards: zero rays, zero speed_of_sound, empty geometry
- Fixed `suggest_absorption_placement` to use `target_rt60` parameter

### Stats
- 216 tests (209 unit + 6 integration + 1 doc)
- 21 criterion benchmarks with history tracking
- All cleanliness checks passing (fmt, clippy, audit, deny)

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
