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
| propagation   | 🟡 partial (scalar core + Vec3 profiles) | 35 ✓ |
| *(34 others)* | ⬜ pending          | —     |

`propagation` is complete except `refract_ray_step` /
`trace_ray_atmospheric` — both take a closure (`speed_fn`) and need the
fnptr/callback pattern; see roadmap M1.

## Tests

- `tests/propagation.tcyr` — **35 assertions, all green**. Covers
  speed-of-sound, inverse-square, SPL/pressure, atmospheric absorption
  (ISO 9613-1), Doppler, Miki ground reflection, and the Vec3 wind/
  temperature profiles. Ported one-for-one from the Rust `#[test]` blocks.

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

See [`roadmap.md`](roadmap.md). Immediate: finish M1 (foundation layer:
`error` ✅, `propagation` 🟡, then `material`, `resonance`, `ambisonics`,
`dark_velvet_noise`, `scattering`).
