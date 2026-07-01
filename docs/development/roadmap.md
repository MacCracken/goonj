# goonj — Roadmap

> Milestone plan to the **2.0.0** Cyrius port. State lives in
> [`state.md`](state.md); per-module parity in [`port-audit.md`](port-audit.md).
> This file is the sequencing — what ships, in what order, against what
> dependency gates.

## 2.0.0 criteria (Rust → Cyrius parity)

- [x] **All 37 modules ported**, function-for-function against `rust-old/`
- [x] **Each module has a `tests/*.tcyr` suite** ported from its Rust `#[test]`s
      (3585 assertions, 36 suites, all green)
- [x] Per-module cleanliness green: canonical `cyrius fmt` diff + `cyrius lint`
      (0 warnings) + whole-project `cyrius vet`.
- [x] Benchmarks captured — the **full suite** (34 benchmarks / 15 modules) ported
      to `.bcyr`, with a Rust→Cyrius comparison
      ([`../benchmarks-rust-vs-cyrius.md`](../benchmarks-rust-vs-cyrius.md)):
      honest ~6–18× on compute-bound arithmetic, alloc-dominated elsewhere.
- [~] `dist/goonj.cyr` distlib bundle builds ✅ (validated by `tests/bundle.tcyr`);
      ≥1 consumer compiles green — deferred (consumers not ported yet)
- [x] CHANGELOG complete for 2.0.0; ADR-0001 covers the port conventions

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

### M1 — Foundation (L0 + material) — ✅ complete

Acceptance: every L0 module + `material` ported with a green parity suite.

- [x] **error** — integer error codes (ports `GoonjError`)
- [x] **propagation** — full: scalar core + Vec3 profiles + Snell ray
      tracers (44 tests). Resolved the **closure → fn-pointer+ctx** pattern
      (`fncall2` + `TraceCtx`) and tuple→struct (`RayStep`).
- [x] **material** — absorption tables, wall TL, JCAL porous model (65 tests).
      Resolved manual struct layout (inline arrays) + string fields.
- [x] **resonance** — room modes, Schroeder freq, modal density (15 tests).
      Resolved the `Vec<T>` dynamic-array pattern (stdlib `vec` + f64 sort).
- [x] **ambisonics** — B-format + 3rd-order HOA encoding (20 tests)
- [x] **dark_velvet_noise** — sparse stochastic late reverb (18 tests;
      inline xorshift64 PRNG)
- [x] **scattering** — cosine-weighted hemisphere sampling (211 tests)
- [x] **logging** — real sakshi-backed logging + verbose mode
      (`logging_init_verbose`, `goonj_log_*`; 11 tests)

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

### M4 — Energy & metrics (L4) — ✅ 4/4

**diffuse ✅** (2306) + **diffraction ✅** (17) [L4 batch]; **beam ✅** (43)
[impulse/beam batch]; **analysis ✅** (48; C50/C80/D50/EDT/G/ts/LF/IACC/STI per
ISO 3382-1 + IEC 60268-16) [analysis/coupled/wav batch].

### M5 — Impulse responses & integration (L5–L6) — ✅ 6/6

**impulse ✅** (30) [impulse/beam batch]; **coupled ✅** (7) + **wav ✅** (16)
[analysis/coupled/wav batch]; **binaural ✅** (12; HRTF spatialization) +
**dhvani ✅** (5) + **kiran ✅** (6) + **soorat ✅** (16) [final batch]. Also
**logging ✅** (real sakshi-backed, verbose mode).

### M6 — 2.0.0 release close-out (all modules ported ✅)

1. ✅ `dist/goonj.cyr` distlib bundle — collision audit (2 clashes de-collided,
   zero remaining), `[lib]` in `cyrius.cyml`, validated by `tests/bundle.tcyr`.
2. ⏳ Green a downstream consumer — **deferred** (consumers not ported yet).
3. ✅ Diagnostics threaded — the 2 `tracing::warn!` sites (dwm) → `goonj_log_warn`.
4. ✅ `cyrius vet` clean; benchmarks captured (ray + dwm); CHANGELOG finalized;
   **2.0.0 tagged 2026-06-30**.

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

## Post-2.0.0 backlog

- **Review optimizing for various number types.** The port is f64-throughout and
  otherwise unoptimized (naive per-call `alloc`, no arena/pooling, no SIMD). The
  Rust→Cyrius benchmark comparison ([`../benchmarks-rust-vs-cyrius.md`](../benchmarks-rust-vs-cyrius.md))
  is deliberately an honest first pass so it points at exactly this. Cyrius is
  adding **more diverse float/integer types + related SIMD** (driven by model
  training/inference use); once those land, revisit goonj's hot paths (dwm solver,
  ray traversal, wall intersection) to pick appropriate widths (f32 where hisab
  allows, narrower ints for grid indices) and vectorize the inner loops. Expected
  to close much of the current codegen/precision gap. **Gate: Cyrius number-type +
  SIMD support shipping.**
- An arena/pool allocator (or out-param returns) for the alloc-heavy tiny ops
  (`wall_intersection`, per-ray/per-reflection temporaries) — independent of the
  above and doable sooner.

## Out of scope (for 2.0.0)

- New acoustics features beyond the Rust surface (defer to 2.1+).
- Re-architecting module boundaries — parity first; refactor only when the
  ported code demands it (third-instance rule).
