# goonj — Roadmap

> Milestone plan to the **2.0.0** Cyrius port. State lives in
> [`state.md`](state.md); per-module parity in [`port-audit.md`](port-audit.md).
> This file is the sequencing — what ships, in what order, against what
> dependency gates.

## 2.0.0 criteria (Rust → Cyrius parity)

- [ ] All 37 modules ported, function-for-function against `rust-old/`
- [ ] Each module has a `tests/*.tcyr` suite ported from its Rust `#[test]`s
- [ ] Cleanliness gates green: `cyrius fmt --check`, `cyrius lint`, `cyrius vet`
- [ ] Benchmarks captured (`docs/benchmarks/`) for the hot paths
      (ray intersection, propagation, mode computation)
- [ ] `dist/goonj.cyr` distlib bundle builds; ≥1 consumer compiles green
- [ ] CHANGELOG complete for 2.0.0; ADRs for the non-obvious port choices

## Porting order (dependency layers)

Derived from the `use crate::X` graph in `rust-old/src/`. Port bottom-up so
every module's goonj-internal deps already exist. hisab (HVec3) and the
stdlib are available to every layer.

- **L0 — leaves (no internal deps):** error, propagation, resonance,
  ambisonics, dark_velvet_noise, scattering, logging
- **L1:** material (→ error) — *the spine; most modules need it*
- **L2:** hybrid, directivity, room, metamaterial, fdn, gfpe, diffusion,
  fdtd, outdoor, portal, udfa, underwater, vibroacoustics, bridge
- **L3:** ray, radiosity, image_source, dwm
- **L4:** diffuse, diffraction, analysis, beam
- **L5:** impulse, coupled
- **L6:** wav, binaural, integration/{dhvani,kiran,soorat}

## Milestones

### M0 — Port scaffold (✅ shipped 2026-06-30)

- `cyrius port` scaffold landed; Rust frozen at `rust-old/`
- `cyrius.cyml` wired: stdlib + math/ganita + hisab 2.6.7; `cyrius.lock` committed
- Smoke binary builds and runs

### M1 — Foundation (L0 + material) — *in progress*

Acceptance: every L0 module + `material` ported with a green parity suite.

- [x] **error** — integer error codes (ports `GoonjError`)
- [x] **propagation** — full: scalar core + Vec3 profiles + Snell ray
      tracers (44 tests). Resolved the **closure → fn-pointer+ctx** pattern
      (`fncall2` + `TraceCtx`) and tuple→struct (`RayStep`).
- [x] **material** — absorption tables, wall TL, JCAL porous model (65 tests).
      Resolved manual struct layout (inline arrays) + string fields.
- [x] **resonance** — room modes, Schroeder freq, modal density (15 tests).
      Resolved the `Vec<T>` dynamic-array pattern (stdlib `vec` + f64 sort).
- [ ] **ambisonics** — B-format + 3rd-order HOA encoding
- [ ] **dark_velvet_noise** — sparse stochastic late reverb
- [ ] **scattering** — cosine-weighted hemisphere sampling
- [ ] **logging** — tracing shim (maps to `sakshi`)

### M2 — Geometric & wave spine (L2) — ✅ 13/14 (parallel workflow)

room, hybrid, directivity, metamaterial, fdn, gfpe, diffusion, outdoor,
portal, udfa, underwater, vibroacoustics, bridge — **ported in one 13-agent
parallel workflow** (worktree-isolated), then integrated + re-verified in main
(291 assertions). **fdtd** later landed in wave 2, completing L2 (14/14).
The workflow proved the recipe scales: no new patterns, all gates green.

### M3 — Acceleration & sources (L3) — ✅ 4/4

radiosity ✅ + image_source ✅ (wave 2); **ray ✅** (275 assertions + benchmark);
**dwm ✅** — ported solo in two sub-bites (too large for one): **A** non-solver
API (config, `BoundaryFilter`, dispersion, 36 tests); **B** the `solve_dwm_3d`
3D grid solver (raw-buffer hot loop) + `tests/dwm.bcyr` benchmark (53 tests
total). `DWM_`-prefixed to dodge fdtd's flat-namespace symbols.

### M4 — Energy & metrics (L4) — 🟡 3/4

**diffuse ✅** (2306) + **diffraction ✅** (17) [L4 batch]; **beam ✅** (43;
volumetric beam tracing) [impulse/beam batch]. Remaining: **analysis**
(C50/C80/D50/EDT/STI/…) — now unblocked (impulse done).

### M5 — Impulse responses & integration (L5–L6) — 🟡 1/6

**impulse ✅** (30; RT60 estimators + Schroeder EDC + generate_ir via
image-source early reflections + diffuse-rain late reverb) [impulse/beam batch].
Remaining: coupled + wav (unblocked), binaural (needs wav), integration ×3.
Then build `dist/goonj.cyr`, green a downstream consumer, tag **2.0.0**.

## Known port challenges (capture as ADRs when resolved)

- ~~**Closures → fnptr/callback**~~ — RESOLVED in `propagation`: fn-pointer +
  context-pointer (`fncall2` + `TraceCtx`). Reuse for ray tracers, diffuse
  rain, optimizers.
- ~~**`Vec<T>` / dynamic arrays**~~ — RESOLVED in `resonance`: stdlib `vec`
  (f64 in 8-byte slots). Reuse for paths, hit lists, IR sample buffers.
- ~~**Fixed arrays / String fields in structs**~~ — RESOLVED in `material`:
  manual `alloc` + `store64`/`load64` offset layout; cstring + `streq`.
- **f32 → f64 widening** — forced by hisab (HVec3 is f64). Loosen test
  tolerances vs the f32 oracle where bit-exactness isn't meaningful.
- **`serde` derive** — Rust types derive Serialize/Deserialize. No serde in
  Cyrius; (de)serialization, if any consumer needs it, is hand-rolled per type.
- **String-payload errors → integer codes** — already chosen for `error`.

## Out of scope (for 2.0.0)

- New acoustics features beyond the Rust surface (defer to 2.1+).
- Re-architecting module boundaries — parity first; refactor only when the
  ported code demands it (third-instance rule).
