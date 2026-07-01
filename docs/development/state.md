# goonj — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**2.0.0** — Cyrius port **complete** (released 2026-06-30). goonj's Rust line
shipped through 1.4.3; the Cyrius rewrite is a major break, so it lands as
**2.0.0**. The 14,630-line Rust source is frozen at `rust-old/` as the parity
oracle.

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

**37 / 37 modules ported — PORT COMPLETE · 3585 parity assertions green across 36 suites.**

| Layer | Modules (✅) |
|-------|-------------|
| L0    | error, propagation (full, 44), resonance (15), ambisonics (20), scattering (211), dark_velvet_noise (18), logging (11, real sakshi) |
| L1    | material (65) |
| L2    | hybrid (20), directivity (19), metamaterial (51), room (10), fdn (14), gfpe (23), diffusion (7), fdtd (39), outdoor (28), portal (14), udfa (15), underwater (36), vibroacoustics (25), bridge (29) |
| L3    | radiosity (7), image_source (30), ray (275, +bench), dwm (53, +bench) — **L3 complete** |
| L4    | diffuse (2306), diffraction (17), beam (43), analysis (48) — **L4 complete** |
| L5    | impulse (30), coupled (7) — **L5 complete** |
| L6    | wav (16), binaural (12), integration: dhvani (5), kiran (6), soorat (16) — **L6 complete** |
| pending | none — all 37 modules ported |

Per-module detail in [`port-audit.md`](port-audit.md). Benchmarks in
[`../benchmarks/results.md`](../benchmarks/results.md). Toolchain: cyrius **6.3.14**.

## Tests

One `tests/<module>.tcyr` suite per ported module, each ported one-for-one
from that module's Rust `#[test]` blocks (serde round-trips dropped — no
serde). **36 suites, 3585 assertions, all green.** Run a suite with
`cyrius test tests/<module>.tcyr`. Six parallel workflows landed 30 of the 37
modules (L2 ×13 + wave-2 ×6 + L4 ×2 + impulse/beam ×2 + analysis/coupled/wav ×3 +
final ×4); the rest were ported solo — each batch independently re-verified in
main (tests + canonical-fmt diff + lint). `logging` is real sakshi-backed
logging with a verbose mode (`logging_init_verbose`) for diagnosis. Hot-path
benchmarks: `ray`, `dwm` (`cyrius bench tests/{ray,dwm}.bcyr`). Each batch was independently re-verified in main (tests +
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

See [`roadmap.md`](roadmap.md). **All 37 modules ported + release close-out
mostly done:**
- ✅ Cross-module collision audit (2 clashes de-collided; zero remaining).
- ✅ `dist/goonj.cyr` distlib bundle (`[lib]` in `cyrius.cyml`; `tests/bundle.tcyr`
  cross-layer smoke, 11 green).
- ✅ Diagnostics: the 2 `tracing::warn!` sites (dwm) → `goonj_log_warn` + sakshi
  verbose mode.
- ✅ `cyrius vet` clean; benchmarks captured (ray + dwm).
- ✅ **2.0.0 released** — tagged 2026-06-30.
- ⏳ **Consumer-green** — deferred until a downstream consumer (dhvani/shruti/
  kiran) is itself ported (working up the stack).

**Every language pattern is now proven** (f32→f64, hex literals, integer errors,
HVec3, manual layout, `Vec`, closures→fnptr+ctx, tuple→struct, bit-ops/xorshift,
caller-supplied randoms). Only **serde** is unhandled — and it's being dropped
(no consumer needs it yet).
