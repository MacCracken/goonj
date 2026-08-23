# goonj — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**2.0.1** — maintenance release: toolchain + dependency refresh on top of the
completed port. No source-behaviour changes (the only `src/` edits were
canonical-fmt whitespace). **2.0.0** (released 2026-06-30) was the port itself —
goonj's Rust line shipped through 1.4.3 and the Cyrius rewrite is a major break.
The 14,630-line Rust source is frozen at `rust-old/` as the parity oracle.

## Toolchain

- **Cyrius pin**: `6.5.35` (in `cyrius.cyml [package].cyrius`) — moved
  deliberately at 2.0.1 (was `6.3.14`). The pin is still set on purpose: bump it
  as a decision, don't auto-chase `cycc` wrapper drift.
- The 6.5.35 formatter indents continuation lines deeper than 6.3.x did, so the
  whole tree was re-canonicalised at 2.0.1 (whitespace only). **Three of the four
  obvious ways to check formatting are traps** (all verified against 6.5.35):
  - `cyrius fmt <file>` **rewrites the file in place** and prints nothing. It is
    not a dry run — do not reach for it to inspect.
  - `cyrius fmt <file> --check` is non-destructive but exits 0 with empty output
    even on a file that is *not* canonical. It cannot gate anything.
  - `cyrius fmt <file> --dry` likewise prints "already canonically formatted"
    for a non-canonical file.
  - `cyrfmt <file>` is the honest one: prints canonical output to stdout, never
    touches the file, works on any path. **Definitive per-file check:**
    `diff <(cyrfmt src/x.cyr) src/x.cyr`.
  Project-wide, `cyrius audit`'s fmt section is the real gate — it correctly
  listed all 23 drifted `src/` files when `--check` claimed clean.
- Build: `cyrius build src/main.cyr build/goonj`
- Test: `cyrius test tests/<suite>.tcyr`

### Quality gates (`cyrius audit`)

| Gate | Status |
|------|--------|
| fmt   | ✅ clean (was FAIL / 23 files before the 2.0.1 re-canonicalisation) |
| lint  | ✅ clean |
| docs  | ⚠ **55 undocumented public fns** — pre-existing, unchanged by 2.0.1 |
| tests | ✅ 38 suites green |
| bench | ✅ 16 suites green |

`cyrius audit` **exits 1**, and did so at 2.0.0 as well — the docs gate is the
only non-green one. Don't read the exit status as a regression signal; read the
gate list. (Note the exit code is easy to lose: `cyrius audit > log; echo $?`
in a compound command reports the `echo`, not the audit.)

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
[`../benchmarks/results.md`](../benchmarks/results.md). Toolchain: cyrius **6.5.35**.

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
- **hisab** 2.11.2 — math/geometry; consumed as the single `dist/hisab.cyr`
  bundle (HVec3 and friends). Pulls transitive **sakshi** 2.4.11.

`cyrius.lock` pins 31 entries (2 commit-pinned deps + the vendored `lib/`
hashes) — up from 29 at 2.0.0, because 6.5.35's stdlib snapshot adds
`lib/callback.cyr` and `lib/tagged.cyr` as transitive leaves.

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
- ✅ **2.0.1** — toolchain pin 6.3.14 → **6.5.35**, hisab 2.6.7 → **2.11.2**,
  transitive sakshi 2.4.2 → **2.4.11**, stdlib re-vendored, tree re-canonicalised
  to the 6.5.35 formatter (23 `src/` + 28 `tests/` files, whitespace only). All
  3585 parity assertions stay green; `dist/goonj.cyr` is byte-identical apart
  from the version stamp and the fmt whitespace.
- ⏳ **Consumer-green** — deferred until a downstream consumer (dhvani/shruti/
  kiran) is itself ported (working up the stack).

**Every language pattern is now proven** (f32→f64, hex literals, integer errors,
HVec3, manual layout, `Vec`, closures→fnptr+ctx, tuple→struct, bit-ops/xorshift,
caller-supplied randoms). Only **serde** is unhandled — and it's being dropped
(no consumer needs it yet).
