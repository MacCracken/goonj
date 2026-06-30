# goonj — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**2.0.0** — Cyrius port *in progress*. goonj's Rust line shipped through
1.4.3; the Cyrius rewrite is a major break, so it lands as **2.0.0**. The
14,630-line Rust source is frozen at `rust-old/` as the parity oracle.

## Toolchain

- **Cyrius pin**: `6.3.12` (in `cyrius.cyml [package].cyrius`)
- Build: `cyrius build src/main.cyr build/goonj`
- Test: `cyrius test tests/<suite>.tcyr`

## Source

- Rust reference: 14,630 lines across 37 modules at `rust-old/` (frozen).
- Cyrius port: `src/main.cyr` (smoke) + per-module `src/*.cyr` (library,
  validated via `tests/*.tcyr`, not included by the smoke binary — same
  layout hisab uses).

## Port progress

Per-module parity is tracked in [`port-audit.md`](port-audit.md). Summary:

| Module        | Status            | Tests |
|---------------|-------------------|-------|
| error         | ✅ ported          | (via propagation suite) |
| propagation   | ✅ ported (full)   | 44 ✓  |
| material      | ✅ ported          | 65 ✓  |
| resonance     | ✅ ported          | 15 ✓  |
| *(32 others)* | ⬜ pending          | —     |

`propagation` is complete except `refract_ray_step` /
`trace_ray_atmospheric` — both take a closure (`speed_fn`) and need the
fnptr/callback pattern; see roadmap M1.

## Tests

- `tests/propagation.tcyr` — **44 assertions, all green**. Covers
  speed-of-sound, inverse-square, SPL/pressure, atmospheric absorption
  (ISO 9613-1), Doppler, Miki ground reflection, the Vec3 wind/temperature
  profiles, and the Snell-law ray tracers (refract_ray_step,
  trace_ray_atmospheric).
- `tests/material.tcyr` — **65 assertions, all green**. Covers the 7
  named-material absorption tables, average/band access, validated
  constructor, WallConstruction transmission loss/coefficient, and the
  JCAL porous model.
- `tests/resonance.tcyr` — **15 assertions, all green**. Covers room_mode,
  axial/all-axial mode lists (Vec + sort), Schroeder frequency, modal density.

All ported one-for-one from the Rust `#[test]` blocks. **124 assertions
total across 3 suites, all green.**

## Dependencies

Direct (declared in `cyrius.cyml`, locked in `cyrius.lock`):

- **stdlib** — syscalls, string, alloc, str, fmt, vec, io, args, assert,
  **math** (F64_* constants, clamp/min/max, exp/ln polyfills),
  **ganita** (transcendentals: pow/exp/ln/sin/cos/sqrt/acos/…).
- **hisab** 2.6.7 — math/geometry; consumed as the single `dist/hisab.cyr`
  bundle (HVec3 and friends). Pulls transitive `sakshi`.

## Consumers

dhvani, shruti, kiran/joshua, aethersafha — *none consuming the Cyrius
port yet* (gated on the distlib bundle, roadmap M5).

## Next

See [`roadmap.md`](roadmap.md). M1 remaining L0 leaves: `ambisonics`,
`dark_velvet_noise` (needs RNG), `scattering` (needs RNG), `logging`
(→ `sakshi`). Done: `error`, `propagation` (full), `material`, `resonance`.
The closure/`Vec`/manual-layout/string patterns are all proven — the L2
spine (gated only on `material`, already done) is now unblocked.
