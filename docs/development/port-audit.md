# goonj — Rust → Cyrius Port Audit

Per-module parity ledger for the Rust → Cyrius port, **refreshed at 2.0.4**.
Every Cyrius module matches the Rust oracle function-for-function.

> **This is a port record, not a living per-module dashboard.** It is refreshed
> when the port itself changes, not every release — the live per-module view is
> the layer table in [`state.md`](state.md), and release detail is in the
> [CHANGELOG](../../CHANGELOG.md). Keeping two living per-module views is what
> let these numbers drift across 2.0.1–2.0.3 in the first place.

**Status:** ✅ ported & tested · 🟡 partial · ⬜ pending
**LOC** = Rust lines (incl. tests) at `rust-old/src/`.

## Conventions established (apply to every module)

- **f32 → f64** everywhere (hisab's `HVec3` is f64-only; widening is forced
  and improves precision). Test tolerances are loosened vs the f32 oracle
  where bit-exactness is not meaningful.
- **Float literals**: integers via `f64_from(n)`; non-integers as named
  module-top `var` constants holding the IEEE-754 hex bit pattern with the
  decimal in a comment (generate with
  `python3 -c "import struct;print(hex(struct.unpack('<Q',struct.pack('<d',X))[0]))"`).
- **`Vec3` → hisab `HVec3`**: `Vec3::new`→`hvec3_new`, `.dot/.cross/.length/
  .normalize/.distance`→`hvec3_*`, `+ - *`→`hvec3_add/sub/scale`, `.x/.y/.z`→
  `HVec3_x/y/z`, `Vec3::X/ZERO/splat`→`hvec3_unit_x/zero/splat`.
- **`enum` errors → integer codes** (see `src/error.cyr`).
- **`Vec<T>` → stdlib `vec`** (`vec_new`/`vec_push`/`vec_len`/`vec_get`/`vec_set`);
  f64 elements store directly in the 8-byte slots. Sorting f64 vecs uses a local
  insertion sort comparing with `f64_gt` (`resonance._vec_sort_f64`).
- **Closures → fn-pointer + context-pointer** (stdlib `fnptr`): `&fn_name` yields
  a pointer; `fncallN(fp, …)` invokes it. A captured closure becomes
  `callback(ctx, args…)` + a `ctx` struct holding the captures (mutated in place
  if the capture changes per call). Tuple returns become a small struct.
  Reference: `propagation.refract_ray_step` / `trace_ray_atmospheric` + `TraceCtx`.
- **structs** via `#derive(accessors)` + `alloc(sizeof(T))`; methods become
  free functions `type_verb(self, …)`.
- **Module files do not `include` each other** — the build/test entry includes
  them in dependency order (stdlib + hisab auto/explicit first).

## Ledger

### L0 — leaves (no goonj-internal deps)

| Module            | LOC | Status | Notes |
|-------------------|----:|--------|-------|
| error             |  60 | ✅ | Integer codes + `goonj_is_err`/`goonj_err_name`; shared `GOONJ_EPSILON`, `F64_NEG_INF`. Rust's own tests here covered only the `Display` impl and the `Result` alias — both dropped by design (ADR-0001), so this shipped without a suite until 2.0.4 added `tests/error.tcyr`. 27 tests. |
| propagation       | 831 | ✅ | Scalar core + Vec3 wind/temp profiles + Snell ray tracers (`refract_ray_step`, `trace_ray_atmospheric`). 44 tests green. Established the closure → fn-pointer+ctx pattern (`TraceCtx`) and tuple→struct (`RayStep`). |
| resonance         | 194 | ✅ | Room modes, Schroeder freq, modal density. `Vec<f32>` mode lists → stdlib `vec` (f64 in 8-byte slots) + f64 insertion sort. 15 tests green. Established the dynamic-array idiom. |
| ambisonics        | 259 | ✅ | B-format + 3rd-order HOA (real SH, ACN/SN3D). 20 tests. |
| dark_velvet_noise | 405 | ✅ | Sparse stochastic late reverb. Self-contained **xorshift64** PRNG (C-style bit ops `^ << >> &`) — no stdlib `random` needed. 18 tests. |
| scattering        | 165 | ✅ | Cosine-weighted hemisphere sampling. **Randoms are caller-supplied params** (`u1,u2`) — no internal RNG. orthonormal_basis tuple → struct. 211 tests (100-sample sweep). |
| logging           |  33 | ✅ | Real **sakshi-backed** logging (not a stub): `logging_init`/`logging_init_verbose` (WARN/TRACE thresholds), `logging_set_level`, `goonj_log_{fatal..trace}`. Verbose mode for diagnosis; level gating verified via the ring buffer. 2.0.4 restored the `GOONJ_LOG` env-var control the Rust original had. 20 tests. |

### L1 — the spine

| Module   | LOC | Status | Notes |
|----------|----:|--------|-------|
| material | 589 | ✅ | AcousticMaterial (inline 8-band array + cstring name, manual layout), WallConstruction (mass-law TL), JCAL porous model. 65 tests green. serde round-trip omitted (no serde). Established: manual struct layout, `pct`/`milli` literal helpers, string field via `streq`. |

### L2 — geometric & wave (dep: material, ±propagation)

13 of 14 ported in one parallel workflow (worktree-isolated agents, one per
module), then integrated + independently re-verified in main. Test counts are
the parity assertions that pass. `fdtd` landed in the next wave once `hybrid`
was done, completing L2 (14/14).

| Module         | LOC | Status | Tests | Deps |
|----------------|----:|--------|------:|------|
| room           | 320 | ✅ | 10 | material |
| hybrid         | 138 | ✅ | 20 | material |
| directivity    | 251 | ✅ | 19 | material |
| metamaterial   | 520 | ✅ | 51 | error, material |
| fdn            | 252 | ✅ | 14 | propagation |
| gfpe           | 589 | ✅ | 23 | propagation |
| diffusion      | 220 | ✅ |  7 | propagation |
| fdtd           | 513 | ✅ | 39 | hybrid, material |
| outdoor        | 262 | ✅ | 28 | material, propagation |
| portal         | 203 | ✅ | 14 | material, propagation |
| udfa           | 202 | ✅ | 15 | material, propagation |
| underwater     | 476 | ✅ | 36 | material, propagation |
| vibroacoustics | 358 | ✅ | 25 | material, propagation |
| bridge         | 329 | ✅ | 29 | material, propagation |

### L3 — acceleration & sources

| Module       |  LOC | Status | Tests | Deps |
|--------------|-----:|--------|------:|------|
| ray          | 1146 | ✅ | 277 | material, room — hot path; ported solo + `tests/ray.bcyr` benchmark |
| radiosity    |  283 | ✅ | 7 | material, room |
| image_source |  634 | ✅ | 30 | material, propagation, room |
| dwm          | 1407 | ✅ | 54 | fdtd ✅, hybrid, material — largest module. Full: config, `BoundaryFilter`, dispersion, and the `solve_dwm_3d` 3D grid solver (raw-buffer hot loop) + `tests/dwm.bcyr` benchmark. `DWM_`-prefixed + `dwm_band_energies` to avoid fdtd's flat-namespace symbols. The 1st-axial-mode test landed at 2.0.4 as its own suite, `tests/dwm_modal.tcyr` (37³ runtime, ~15 s). |

### L4 — energy & metrics

| Module      | LOC | Status | Tests | Deps |
|-------------|----:|--------|------:|------|
| diffuse     | 513 | ✅ | 2306 | material, propagation, ray, room — inline xorshift64 (`_diffuse_`-prefixed), fibonacci_sphere, diffuse-rain via ray API |
| diffraction | 311 | ✅ | 17 | material, propagation, ray, room — UTD/BTM edge diffraction |
| analysis    | 895 | ✅ | 48 | impulse, material, room — C50/C80/D50/EDT/G/ts/LF/IACC/STI (ISO 3382-1, IEC 60268-16) |
| beam        | 297 | ✅ | 43 | diffuse, material, room — volumetric beam tracing |

### L5–L6 — impulse responses & integration

| Module              | LOC | Status | Tests | Deps |
|---------------------|----:|--------|------:|------|
| impulse             | 643 | ✅ | 30 | diffuse, image_source, material, propagation, room — RT60 (Sabine/Eyring/Fitzroy/Kuttruff), ImpulseResponse (Schroeder EDC), MultibandIr, generate_ir |
| coupled             | 177 | ✅ | 7 | impulse, material, portal, propagation, room — multi-room energy exchange, double-slope decay |
| wav                 | 257 | ✅ | 16 | error, impulse — 16-bit PCM RIFF/WAVE via in-memory byte buffer (`store8`/LE encoding); no file-I/O syscalls |
| binaural            | 317 | ✅ | 12 | error, image_source, impulse, material, propagation, room, wav — HRTF spatialization, nearest-pair lookup, stereo IR + WAV export |
| dhvani (integ.)     | 129 | ✅ | 5 | impulse, material, room — convolution-reverb IR handoff (`dhvani_*`) |
| kiran (integ.)      | 168 | ✅ | 6 | diffraction, material, room — real-time occlusion queries (`kiran_*`) |
| soorat (integ.)     | 165 | ✅ | 16 | ray, resonance — visualization data (`soorat_*`) |

**Totals: 37 / 37 done — PORT COMPLETE.** 14,630 Rust lines · **3624 parity
assertions across 37 suites, all green** (cycc 6.5.35, pin 6.5.35, as of 2.0.4;
the port originally shipped 3585/36 on pin 6.3.14). `mod.rs`
was Rust module-organization only (feature-gated `pub mod`) — nothing to port.

Release close-out status:
- ✅ **Cross-module collision audit** — only 2 clashes across all 37 modules
  (`gfpe.MAX_GRID_CELLS`→`GFPE_MAX_GRID_CELLS`, `vibroacoustics.RHO_AIR`→
  `VIBRO_RHO_AIR`, both distinct values); zero remaining.
- ✅ **`dist/goonj.cyr` distlib bundle** (`[lib]` in `cyrius.cyml`; ~8.6k lines).
  Validated by `tests/bundle.tcyr` (cross-layer smoke, 11 assertions).
- ✅ **Diagnostics** — the 2 Rust `tracing::warn!` sites (dwm dx-tolerance) wired
  to `goonj_log_warn` + sakshi verbose mode.
- ✅ **2.0.0 released** — tagged 2026-06-30.
- ⏳ **Consumer-green** — deferred; downstream consumers (dhvani/shruti/kiran)
  aren't ported yet (working up the stack).
Toolchain: cyrius **6.3.14** (pinned deliberately — held here, not chased to
newer wrappers). Six parallel workflows plus solo bites landed all 37 modules;
`ray` and `dwm` (hot paths, with benchmarks) were ported solo. Remaining: none —
all 37 modules ported.

### RNG / randomness (no new pattern needed)
- **scattering** takes `u1,u2` uniform randoms as **parameters** (caller's RNG).
- **dark_velvet_noise** ships its own **xorshift64** via C-style bit ops
  (`^ << >> &`); determinism tests don't depend on exact shift semantics.
No stdlib `random` dependency was introduced.

## Note on the parallel workflow

The L2 batch was ported by a 13-agent workflow (each agent in its own git
worktree). Agents self-verify, but **integration re-verification in main is the
real gate** — it caught fmt continuation-line drift in 4 source + 6 test files
that the agents' `cyrius fmt --check` missed.

> ⚠ **The fmt advice that used to close this section is wrong on the current
> pin and was dangerous.** Under 6.5.35, `cyrius fmt <file>` **rewrites the file
> in place** and prints nothing — using it to "inspect" silently reformats your
> tree. `--check` and `--dry` both report a non-canonical file as clean, so
> neither can gate. The read-only tool is `cyrfmt`:
> `diff <(cyrfmt src/x.cyr) src/x.cyr`. Project-wide, `cyrius audit`'s fmt
> section is the gate. See [`state.md`](state.md#toolchain).
