# goonj — Rust → Cyrius Port Audit

Per-module parity ledger for the 2.0.0 port. The Rust oracle is frozen at
`rust-old/`; every Cyrius module must match it function-for-function. Update
the relevant row whenever a module's status changes.

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
| error             |  60 | ✅ | Integer codes + `goonj_is_err`/`goonj_err_name`; shared `GOONJ_EPSILON`, `F64_NEG_INF`. |
| propagation       | 831 | ✅ | Scalar core + Vec3 wind/temp profiles + Snell ray tracers (`refract_ray_step`, `trace_ray_atmospheric`). 44 tests green. Established the closure → fn-pointer+ctx pattern (`TraceCtx`) and tuple→struct (`RayStep`). |
| resonance         | 194 | ✅ | Room modes, Schroeder freq, modal density. `Vec<f32>` mode lists → stdlib `vec` (f64 in 8-byte slots) + f64 insertion sort. 15 tests green. Established the dynamic-array idiom. |
| ambisonics        | 259 | ✅ | B-format + 3rd-order HOA (real SH, ACN/SN3D). 20 tests. |
| dark_velvet_noise | 405 | ✅ | Sparse stochastic late reverb. Self-contained **xorshift64** PRNG (C-style bit ops `^ << >> &`) — no stdlib `random` needed. 18 tests. |
| scattering        | 165 | ✅ | Cosine-weighted hemisphere sampling. **Randoms are caller-supplied params** (`u1,u2`) — no internal RNG. orthonormal_basis tuple → struct. 211 tests (100-sample sweep). |
| logging           |  33 | ⬜ | Tracing shim → `sakshi`. |

### L1 — the spine

| Module   | LOC | Status | Notes |
|----------|----:|--------|-------|
| material | 589 | ✅ | AcousticMaterial (inline 8-band array + cstring name, manual layout), WallConstruction (mass-law TL), JCAL porous model. 65 tests green. serde round-trip omitted (no serde). Established: manual struct layout, `pct`/`milli` literal helpers, string field via `streq`. |

### L2 — geometric & wave (dep: material, ±propagation)

13 of 14 ported in one parallel workflow (worktree-isolated agents, one per
module), then integrated + independently re-verified in main. Test counts are
the parity assertions that pass. `fdtd` waits on `hybrid` (now done) → next wave.

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
| ray          | 1146 | ✅ | 275 | material, room — hot path; ported solo + `tests/ray.bcyr` benchmark |
| radiosity    |  283 | ✅ | 7 | material, room |
| image_source |  634 | ✅ | 30 | material, propagation, room |
| dwm          | 1407 | ⬜ | — | fdtd ✅, hybrid, material — largest module |

### L4 — energy & metrics

| Module      | LOC | Status | Deps |
|-------------|----:|--------|------|
| diffuse     | 513 | ⬜ | material, propagation, ray, room |
| diffraction | 311 | ⬜ | material, propagation, ray, room |
| analysis    | 895 | ⬜ | impulse, material, room |
| beam        | 297 | ⬜ | diffuse, material, room |

### L5–L6 — impulse responses & integration

| Module              | LOC | Status | Deps |
|---------------------|----:|--------|------|
| impulse             | 643 | ⬜ | diffuse, image_source, material, propagation, room |
| coupled             | 177 | ⬜ | impulse, material, portal, propagation, room |
| wav                 | 257 | ⬜ | error, impulse |
| binaural            | 317 | ⬜ | error, image_source, impulse, material, propagation, room, wav |
| integration/dhvani  | 129 | ⬜ | (consumer API) |
| integration/kiran   | 168 | ⬜ | (consumer API) |
| integration/soorat  | 165 | ⬜ | (consumer API) |

**Totals:** 24 / 37 done, 13 pending · 14,630 Rust lines · **1015 parity assertions green**.
Toolchain: cyrius **6.3.14** (pinned deliberately — held here, not chased to
newer wrappers). Two parallel workflows landed the L2 batch (13) and wave 2 (6);
`ray` (hot path, 275 assertions + a benchmark) was ported solo. Remaining: dwm
(all deps done), logging (→ sakshi); then L4 (diffuse, diffraction, analysis,
beam), L5 (impulse, coupled), L6 (wav, binaural, integration ×3).

### RNG / randomness (no new pattern needed)
- **scattering** takes `u1,u2` uniform randoms as **parameters** (caller's RNG).
- **dark_velvet_noise** ships its own **xorshift64** via C-style bit ops
  (`^ << >> &`); determinism tests don't depend on exact shift semantics.
No stdlib `random` dependency was introduced.

## Note on the parallel workflow

The L2 batch was ported by a 13-agent workflow (each agent in its own git
worktree). Agents self-verify, but **integration re-verification in main is the
real gate** — it caught fmt continuation-line drift in 4 source + 6 test files
that the agents' `cyrius fmt --check` missed (a 6.3.12 quirk: `--check` exits 1
with empty output, so "no output" ≠ clean). Definitive fmt check:
`cyrius fmt <file>` (writes canonical to stdout) diffed against the file.
