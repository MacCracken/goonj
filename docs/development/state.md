# goonj — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**2.0.0** — Cyrius port *in progress*. goonj's Rust line shipped through
1.4.3; the Cyrius rewrite is a major break, so it lands as **2.0.0**. The
14,630-line Rust source is frozen at `rust-old/` as the parity oracle.

## Toolchain

- **Cyrius pin**: `6.3.14` (in `cyrius.cyml [package].cyrius`) — held deliberately; do not chase newer wrapper drift
- Build: `cyrius build src/main.cyr build/goonj`
- Test: `cyrius test tests/<suite>.tcyr`

## Source

- Rust reference: 14,630 lines across 37 modules at `rust-old/` (frozen).
- Cyrius port: `src/main.cyr` (smoke) + per-module `src/*.cyr` (library,
  validated via `tests/*.tcyr`, not included by the smoke binary — same
  layout hisab uses).

## Port progress

Per-module parity is tracked in [`port-audit.md`](port-audit.md). Summary:

**24 / 37 modules ported · 1015 parity assertions green across 23 suites.**

| Layer | Modules (✅) |
|-------|-------------|
| L0    | error, propagation (full, 44), resonance (15), ambisonics (20), scattering (211), dark_velvet_noise (18) |
| L1    | material (65) |
| L2    | hybrid (20), directivity (19), metamaterial (51), room (10), fdn (14), gfpe (23), diffusion (7), fdtd (39), outdoor (28), portal (14), udfa (15), underwater (36), vibroacoustics (25), bridge (29) |
| L3    | radiosity (7), image_source (30), ray (275, + benchmark) |
| pending | dwm (largest), logging (→ sakshi); L4 (diffuse, diffraction, analysis, beam), L5 (impulse, coupled), L6 (wav, binaural, integration ×3) — 13 modules |

Per-module detail in [`port-audit.md`](port-audit.md). Benchmarks in
[`../benchmarks/results.md`](../benchmarks/results.md). Toolchain: cyrius **6.3.14**.

## Tests

One `tests/<module>.tcyr` suite per ported module, each ported one-for-one
from that module's Rust `#[test]` blocks (serde round-trips dropped — no
serde). **23 suites, 1015 assertions, all green.** Run a suite with
`cyrius test tests/<module>.tcyr`. Two parallel workflows landed 19 of the 24
modules (L2 batch of 13 + wave-2 batch of 6); the rest were ported solo. Each
batch was independently re-verified in main (tests + canonical-fmt diff + lint).
`ray` also has a hot-path benchmark: `cyrius bench tests/ray.bcyr`.

## Dependencies

Direct (declared in `cyrius.cyml`, locked in `cyrius.lock`):

- **stdlib** — syscalls, string, alloc, str, fmt, vec, io, args, assert,
  **math** (F64_* constants, clamp/min/max, exp/ln polyfills),
  **ganita** (transcendentals: pow/exp/ln/sin/cos/sqrt/acos/…), **fnptr**
  (`fncallN` indirect calls for the closure→callback pattern), **bench**
  (`bench_new`/`bench_run`/`bench_report` for `.bcyr` benchmarks).
- **hisab** 2.6.7 — math/geometry; consumed as the single `dist/hisab.cyr`
  bundle (HVec3 and friends). Pulls transitive `sakshi`.

## Consumers

dhvani, shruti, kiran/joshua, aethersafha — *none consuming the Cyrius
port yet* (gated on the distlib bundle, roadmap M5).

## Next

See [`roadmap.md`](roadmap.md). 13 modules remain. Next candidates:
- **dwm** (L3, 1407 ln, largest) — all deps done (fdtd ✅); solo. Hot path →
  benchmark too.
- **L4 batch** — `diffuse`, `diffraction` (both need `ray` ✅ + `room`/`material`/
  `propagation`, all done) → parallel-workflow ready. `beam` needs `diffuse`;
  `analysis` needs `impulse` (L5) — those wait.
- **logging** (L0) — thin `sakshi` shim; solo.
- Then L5 (`impulse`, `coupled`), L6 (`wav`, `binaural`, integration ×3).

**Every language pattern is now proven** (f32→f64, hex literals, integer errors,
HVec3, manual layout, `Vec`, closures→fnptr+ctx, tuple→struct, bit-ops/xorshift,
caller-supplied randoms). Only **serde** is unhandled — and it's being dropped
(no consumer needs it yet).
