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

**17 / 37 modules ported · 415 parity assertions green across 16 suites.**

| Layer | Modules (✅) | 
|-------|-------------|
| L0    | error, propagation (full, 44), resonance (15) |
| L1    | material (65) |
| L2    | hybrid (20), directivity (19), metamaterial (51), room (10), fdn (14), gfpe (23), diffusion (7), outdoor (28), portal (14), udfa (15), underwater (36), vibroacoustics (25), bridge (29) |
| pending | fdtd (waits on hybrid ✅ → next), + all of L3–L6 (20 modules) |

Per-module detail in [`port-audit.md`](port-audit.md).

## Tests

One `tests/<module>.tcyr` suite per ported module, each ported one-for-one
from that module's Rust `#[test]` blocks (serde round-trips dropped — no
serde). **16 suites, 415 assertions, all green.** Run a suite with
`cyrius test tests/<module>.tcyr`. The L0/L1 suites (propagation 44, material
65, resonance 15) plus the 13 L2 suites (291 assertions) make up the total.

## Dependencies

Direct (declared in `cyrius.cyml`, locked in `cyrius.lock`):

- **stdlib** — syscalls, string, alloc, str, fmt, vec, io, args, assert,
  **math** (F64_* constants, clamp/min/max, exp/ln polyfills),
  **ganita** (transcendentals: pow/exp/ln/sin/cos/sqrt/acos/…), **fnptr**
  (`fncallN` indirect calls for the closure→callback pattern).
- **hisab** 2.6.7 — math/geometry; consumed as the single `dist/hisab.cyr`
  bundle (HVec3 and friends). Pulls transitive `sakshi`.

## Consumers

dhvani, shruti, kiran/joshua, aethersafha — *none consuming the Cyrius
port yet* (gated on the distlib bundle, roadmap M5).

## Next

See [`roadmap.md`](roadmap.md). Next candidates:
- **fdtd** (L2) — now unblocked (`hybrid` done); then **dwm** (L3) which needs it.
- **L3 batch** — `ray` (hot path, benchmark), `radiosity`, `image_source`: all
  gated only on `material`/`room`/`propagation` (done) → parallel-workflow ready.
- **L0 leaves** — `ambisonics`; `scattering` + `dark_velvet_noise` need an **RNG**
  pattern (first unproven pattern remaining); `logging` → `sakshi`.

The closure/`Vec`/manual-layout/string/fnptr patterns are all proven; RNG and
serde (likely droppable) are the only unestablished patterns left.
