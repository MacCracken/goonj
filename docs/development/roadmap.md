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
- [~] **propagation** — scalar core + Vec3 profiles done (35 tests green).
      Remaining: `refract_ray_step`, `trace_ray_atmospheric` — both take a
      `speed_fn` closure; needs the **fnptr/callback** pattern (study
      `lib/fnptr.cyr` / `lib/callback.cyr`; hisab `calc`/`ode` use bare fn
      pointers). First abstraction decision of the port — capture in an ADR.
- [ ] **material** — frequency-dependent absorption/scattering/transmission
- [ ] **resonance** — room modes, Schroeder frequency, modal density
- [ ] **ambisonics** — B-format + 3rd-order HOA encoding
- [ ] **dark_velvet_noise** — sparse stochastic late reverb
- [ ] **scattering** — cosine-weighted hemisphere sampling
- [ ] **logging** — tracing shim (maps to `sakshi`)

### M2 — Geometric & wave spine (L2)

room, metamaterial, directivity, hybrid, fdn, gfpe, diffusion, fdtd,
outdoor, portal, udfa, underwater, vibroacoustics, bridge. Each gated on
material (+ propagation where used). Many are independent of one another →
candidates for a parallel porting **workflow** once the L1 recipe is proven.

### M3 — Acceleration & sources (L3)

ray (BVH-accelerated; hot path → benchmark), radiosity, image_source, dwm.

### M4 — Energy & metrics (L4)

diffuse, diffraction, analysis (C50/C80/D50/EDT/STI/…), beam.

### M5 — Impulse responses & integration (L5–L6)

impulse, coupled, wav, binaural, integration/*. Then build `dist/goonj.cyr`,
green a downstream consumer, tag **2.0.0**.

## Known port challenges (capture as ADRs when resolved)

- **Closures → fnptr/callback** (refract_ray_step, ray tracers, diffuse rain,
  optimizers). The first one sets the pattern for all.
- **`Vec<T>` / dynamic arrays** — Rust returns `Vec<Vec3>` paths, hit lists,
  etc. Map to `vec.cyr` or manual `alloc` + length-prefixed buffers.
- **f32 → f64 widening** — forced by hisab (HVec3 is f64). Loosen test
  tolerances vs the f32 oracle where bit-exactness isn't meaningful.
- **`serde` derive** — Rust types derive Serialize/Deserialize. No serde in
  Cyrius; (de)serialization, if any consumer needs it, is hand-rolled per type.
- **String-payload errors → integer codes** — already chosen for `error`.

## Out of scope (for 2.0.0)

- New acoustics features beyond the Rust surface (defer to 2.1+).
- Re-architecting module boundaries — parity first; refactor only when the
  ported code demands it (third-instance rule).
