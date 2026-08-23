# goonj — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**2.0.4** — closes out the `rust-old/` parity sweep: 7 behavioural divergences
fixed (DVN unsigned modulo, radiosity result aliasing, saturating step counts in
fdtd/dwm/diffusion, gfpe cap + negative index, `GOONJ_LOG`), 6 vacuous tests made
falsifiable, 3 dropped tests restored, and `error` given a suite.

**2.0.3** — fixes a regression 2.0.2 introduced in `generate_ir` (a negative or
NaN `max_time_seconds` allocated ~7.4 GB where Rust yields an empty IR). Found
by the `rust-old/` parity sweep, not by the hardening sweep that caused it.

**2.0.2** — **security / hardening**: 11 reachable P-1 defects fixed (4 SIGSEGV,
3 process aborts, 4 silently-wrong results), pinned by `tests/hardening.tcyr`.

**2.0.1** — toolchain + dependency refresh (pin 6.3.14 → 6.5.35, hisab → 2.11.2).

**2.0.0** (2026-06-30) was the port itself — goonj's Rust line shipped through
1.4.3 and the Cyrius rewrite is a major break.

### The Rust oracle

`rust-old/` (14,630 lines, 37 modules) served as the parity oracle through the
port and three follow-up sweeps. It is **cleared for deletion**: nothing in the
build, test, bench or distlib path reads it — verified by building and running
all 41 suites in a tree with it removed. Two things were extracted first:

- `rust-old/bench-history.csv` → [`../benchmarks/rust-bench-history.csv`](../benchmarks/rust-bench-history.csv),
  the frozen Rust baseline behind every Rust figure in the comparison doc. It
  cannot be regenerated once the tree is gone (no `Cargo.toml`, no Rust toolchain).
- `rust-old/examples/basic.rs` → [`../../examples/basic.cyr`](../../examples/basic.cyr),
  the port's only runnable demo.

After removal the oracle stays readable at tag `2.0.2`:
`git show 2.0.2:rust-old/src/<module>.rs`. The `Ports rust-old/src/x.rs`
provenance comments in `src/` and `tests/` are deliberately kept — they are the
port's audit trail and still resolve against that tag.

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
| docs  | ⚠ **55 undocumented public fns** — pre-existing, unchanged by 2.0.1/2.0.2 |
| tests | ✅ 41 suites green (37 parity + hardening + dwm_modal + 2 smoke) |
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

**37 / 37 modules ported — PORT COMPLETE · 3624 parity assertions green across 37 suites.**

| Layer | Modules (✅) |
|-------|-------------|
| L0    | error (27), propagation (full, 44), resonance (15), ambisonics (20), scattering (211), dark_velvet_noise (18), logging (20, real sakshi + `GOONJ_LOG`) |
| L1    | material (65) |
| L2    | hybrid (20), directivity (19), metamaterial (51), room (10), fdn (14), gfpe (23), diffusion (7), fdtd (39), outdoor (28), portal (14), udfa (15), underwater (36), vibroacoustics (25), bridge (29) |
| L3    | radiosity (7), image_source (30), ray (277, +bench), dwm (54, +bench, +`dwm_modal`) — **L3 complete** |
| L4    | diffuse (2306), diffraction (17), beam (43), analysis (48) — **L4 complete** |
| L5    | impulse (30), coupled (7) — **L5 complete** |
| L6    | wav (16), binaural (12), integration: dhvani (5), kiran (6), soorat (16) — **L6 complete** |
| pending | none — all 37 modules ported |

Per-module detail in [`port-audit.md`](port-audit.md). Benchmarks in
[`../benchmarks/results.md`](../benchmarks/results.md). Toolchain: cyrius **6.5.35**.

## Tests

One `tests/<module>.tcyr` suite per ported module — including `error` as of
2.0.4, whose Rust tests had covered only the `Display` impl and the `Result`
alias (both dropped by design), leaving it the one module without a suite. Each
suite is ported one-for-one from its module's Rust `#[test]` blocks, minus serde
round-trips (no serde in Cyrius). **37 suites, 3624 assertions, all green.** Run
one with `cyrius test tests/<module>.tcyr`.

Four suites sit outside that per-module set, bringing the totals to
**41 suites / 3667 assertions**:

- **`hardening.tcyr`** (26) — every P-1 defect from the 2.0.2 sweep plus the
  2.0.3 IR sample-count contract. Runs against the shipped `dist/goonj.cyr`, so
  it also proves each repair reached the bundle. A real regression suite: it
  SIGSEGVs or aborts against 2.0.1.
- **`dwm_modal.tcyr`** (4) — the DWM solver's only physics-correctness check
  (first axial mode dominates). Split out because it runs ~15 s.
- **`bundle.tcyr`**, **`goonj.tcyr`** — cross-layer and smoke.

Six parallel workflows landed 30 of the 37 modules; the rest were ported solo.
Each batch was independently re-verified in main (tests + canonical-fmt diff +
lint) — that integration gate is the real one, not the agents' self-checks.
`logging` is real sakshi-backed logging, honouring `GOONJ_LOG` as the Rust
original did. `ray` and `dwm` carry hot-path benchmarks; 16 `.bcyr` suites exist
in total, recorded by `scripts/bench-history.sh`.

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
port yet*. The bundle is not the gate — `dist/goonj.cyr` has shipped since
2.0.0, is validated by `tests/bundle.tcyr` and gated in CI by
`cyrius distlib --check`. The gate is the consumers themselves being ported;
see [`roadmap.md`](roadmap.md#open-gates).

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
- ✅ **2.0.2** — P-1 hardening sweep: 11 reachable defects fixed across `fdtd`,
  `dwm`, `material`, `metamaterial`, `analysis`, `propagation`, `impulse`,
  `binaural`, `ambisonics`, `radiosity`, `wav`; `tests/hardening.tcyr` added.
  Rust-era `scripts/{version-bump,bench-history}.sh` ported to Cyrius; CI now
  gates `cyrius distlib --check`.
- ⏳ **Consumer-green** — deferred until a downstream consumer (dhvani/shruti/
  kiran) is itself ported (working up the stack).

**Every language pattern is now proven** (f32→f64, hex literals, integer errors,
HVec3, manual layout, `Vec`, closures→fnptr+ctx, tuple→struct, bit-ops/xorshift,
caller-supplied randoms). Only **serde** is unhandled — and it's being dropped
(no consumer needs it yet).
