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

**29 / 37 modules ported · 3464 parity assertions green across 28 suites.**

| Layer | Modules (✅) |
|-------|-------------|
| L0    | error, propagation (full, 44), resonance (15), ambisonics (20), scattering (211), dark_velvet_noise (18) |
| L1    | material (65) |
| L2    | hybrid (20), directivity (19), metamaterial (51), room (10), fdn (14), gfpe (23), diffusion (7), fdtd (39), outdoor (28), portal (14), udfa (15), underwater (36), vibroacoustics (25), bridge (29) |
| L3    | radiosity (7), image_source (30), ray (275, +bench), dwm (53, +bench) — **L3 complete** |
| L4    | diffuse (2306), diffraction (17), beam (43) |
| L5    | impulse (30) |
| pending | analysis (unblocked), coupled (unblocked), wav (unblocked), logging (→ sakshi); binaural (needs wav), integration ×3 — 8 modules |

Per-module detail in [`port-audit.md`](port-audit.md). Benchmarks in
[`../benchmarks/results.md`](../benchmarks/results.md). Toolchain: cyrius **6.3.14**.

## Tests

One `tests/<module>.tcyr` suite per ported module, each ported one-for-one
from that module's Rust `#[test]` blocks (serde round-trips dropped — no
serde). **28 suites, 3464 assertions, all green.** Run a suite with
`cyrius test tests/<module>.tcyr`. Four parallel workflows landed 23 of the 29
modules (L2 ×13 + wave-2 ×6 + L4 ×2 + impulse/beam ×2); the rest were ported solo. Each batch was independently re-verified in main (tests +
canonical-fmt diff + lint). `ray` and `dwm` have hot-path benchmarks
(`cyrius bench tests/{ray,dwm}.bcyr`).

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

See [`roadmap.md`](roadmap.md). 8 modules remain. `impulse` ✅ unblocked a wave:
- **Batch (unblocked, independent):** `analysis` (C50/C80/D50/EDT/STI…),
  `coupled` (multi-room energy exchange), `wav` (16-bit PCM export), `logging`
  (→ `sakshi`) — a 4-agent parallel workflow.
- Then **`binaural`** (needs `wav` — HRTF spatialization) and the three
  `integration/*` consumer APIs (dhvani, kiran, soorat) — final batch.
- After those: `dist/goonj.cyr` distlib bundle, green a consumer, tag **2.0.0**.

**Every language pattern is now proven** (f32→f64, hex literals, integer errors,
HVec3, manual layout, `Vec`, closures→fnptr+ctx, tuple→struct, bit-ops/xorshift,
caller-supplied randoms). Only **serde** is unhandled — and it's being dropped
(no consumer needs it yet).
