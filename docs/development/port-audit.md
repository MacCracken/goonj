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
- **structs** via `#derive(accessors)` + `alloc(sizeof(T))`; methods become
  free functions `type_verb(self, …)`.
- **Module files do not `include` each other** — the build/test entry includes
  them in dependency order (stdlib + hisab auto/explicit first).

## Ledger

### L0 — leaves (no goonj-internal deps)

| Module            | LOC | Status | Notes |
|-------------------|----:|--------|-------|
| error             |  60 | ✅ | Integer codes + `goonj_is_err`/`goonj_err_name`; shared `GOONJ_EPSILON`, `F64_NEG_INF`. |
| propagation       | 831 | 🟡 | Scalar core + Vec3 wind/temp profiles ported, 35 tests green. Pending: `refract_ray_step`, `trace_ray_atmospheric` (closure `speed_fn` → fnptr/callback). |
| resonance         | 194 | ⬜ | Room modes, Schroeder freq, modal density. |
| ambisonics        | 259 | ⬜ | B-format + 3rd-order HOA. |
| dark_velvet_noise | 405 | ⬜ | Sparse stochastic late reverb (RNG state needed). |
| scattering        | 165 | ⬜ | Cosine-weighted hemisphere sampling (RNG). |
| logging           |  33 | ⬜ | Tracing shim → `sakshi`. |

### L1 — the spine

| Module   | LOC | Status | Notes |
|----------|----:|--------|-------|
| material | 589 | ⬜ | Freq-dependent absorption/scattering/transmission. Needed by most of L2–L6. |

### L2 — geometric & wave (dep: material, ±propagation)

| Module         | LOC | Status | Deps |
|----------------|----:|--------|------|
| room           | 320 | ⬜ | material |
| hybrid         | 138 | ⬜ | material |
| directivity    | 251 | ⬜ | material |
| metamaterial   | 520 | ⬜ | error, material |
| fdn            | 252 | ⬜ | propagation |
| gfpe           | 589 | ⬜ | propagation |
| diffusion      | 220 | ⬜ | propagation |
| fdtd           | 513 | ⬜ | hybrid, material |
| outdoor        | 262 | ⬜ | material, propagation |
| portal         | 203 | ⬜ | material, propagation |
| udfa           | 202 | ⬜ | material, propagation |
| underwater     | 476 | ⬜ | material, propagation |
| vibroacoustics | 358 | ⬜ | material, propagation |
| bridge         | 329 | ⬜ | material, propagation |

### L3 — acceleration & sources

| Module       |  LOC | Status | Deps |
|--------------|-----:|--------|------|
| ray          | 1146 | ⬜ | material, room — **hot path**, benchmark |
| radiosity    |  283 | ⬜ | material, room |
| image_source |  634 | ⬜ | material, propagation, room |
| dwm          | 1407 | ⬜ | fdtd, hybrid, material — largest module |

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

**Totals:** 1 / 37 done, 1 partial, 35 pending · 14,630 Rust lines.
