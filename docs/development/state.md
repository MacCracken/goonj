# goonj — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**2.3.0** — SIMD on the grid solvers: the DWM solver goes from about 15× behind Rust to
about 2×, FDTD from about 17× to 6×, and ray traversal gets faster too. No result moves
([ADR-0008](../adr/0008-row-kernels-for-grid-solvers.md)).

**2.2.1** — the allocation strategy: hot paths work in stack buffers, so a 1 s impulse
response leaves 1.1 MB behind where it left 3.6 MB, and ray paths run about 35% faster.
No result moves, across 172,180 compared cases
([ADR-0007](../adr/0007-stack-buffers-over-an-arena.md)).

**2.2.0** — retires `rust-old/`. No library code changed. A last comparison on valid
input agreed with Rust, six integration tests that were never ported joined the suites,
and every gate was proven in a tree without the oracle before it was deleted.

**2.1.3** — the P(-1) sweep: 172,076 hostile cases differenced against `rust-old/`, all
37 modules read against it, real fuzz targets, allocation fault injection. Fixes 21
out-of-bounds public accessors, a refused BVH build that silently removed every wall,
process aborts on non-finite input, and NaN and negative-count parity. `fdn_new`'s
9.3 GiB limit is down to an address-space requirement. Results on valid input do not
move. See [ADR-0004](../adr/0004-allocation-refusal-ends-the-process.md) and
[ADR-0005](../adr/0005-hostile-input-parity-and-recorded-gaps.md).

**2.1.2** — `cyrius audit` exits 0 for the first time: 217 public fns documented and
29 long lines wrapped, with no code change. The roadmap is rewritten forward.

**2.1.1** — toolchain 6.7.9 → **6.7.18**, hisab 3.4.0 → **3.4.1**, sakshi 2.5.8 →
**2.5.10**, each its project's newest tag. No source or API change, and no result
moved at value level. The consumer floor stays cyrius 6.7.2.

**2.1.0** — toolchain 6.6.20 → **6.7.9**, hisab 3.3.7 → **3.4.0**, sakshi 2.5.7 →
**2.5.8**. No source or API change, and no result moved at value level (5,808
primitive probes and five whole pipelines are bit-identical). It is a minor because
consumers now need **cyrius ≥ 6.7.2**.

**2.0.6** — toolchain 6.6.2 → **6.6.20**, hisab 2.22.1 → **3.3.7**, sakshi 2.5.1 →
**2.5.7**. Fixes the memory exhaustion that `f64_to` saturating at cyrius 6.6.8
opened in the FDTD/DWM gaussian pulses and bounds it in the FDN, with every
assertion green throughout
([ADR-0003](../adr/0003-f64-to-saturates-keep-bounding-in-f64.md)). A non-finite room
dimension through `fdn_new` cost about 9.3 GiB until 2.1.3; the 11.52M alternative is
measured there.

**2.0.5** — toolchain 6.5.35 → 6.6.2, hisab 2.11.2 → 2.22.1; documentation sweep.

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

`rust-old/` (14,630 lines, 37 modules) served as the parity oracle through the port and
four follow-up sweeps, and left the tree at 2.2.0
([ADR-0006](../adr/0006-retire-rust-old-keep-its-bar.md)). The source stays readable at
tag `2.1.3`: `git show 2.1.3:rust-old/src/<module>.rs`. The `rust-old/src/x.rs`
citations in `src/` and `tests/` are deliberately kept and resolve at that tag.

Its other artifacts moved first:

- `rust-old/bench-history.csv` → [`../benchmarks/rust-bench-history.csv`](../benchmarks/rust-bench-history.csv),
  the frozen Rust baseline behind every Rust figure in the comparison doc.
- `rust-old/examples/basic.rs` → [`../../examples/basic.cyr`](../../examples/basic.cyr),
  the port's only runnable demo.
- `rust-old/tests/integration.rs` → [`../../tests/integration.tcyr`](../../tests/integration.tcyr).

## Toolchain

- **Cyrius pin**: `6.7.18` (in `cyrius.cyml [package].cyrius`) — moved
  deliberately at 2.1.1 (was `6.7.9`). The pin is still set on purpose: bump it
  as a decision, don't auto-chase `cycc` wrapper drift. As of 2026-11-06 this pin,
  hisab 3.4.1 and sakshi 2.5.10 are each their project's newest tag.
- **A toolchain bump can change results with the suite still green.** 2.0.6 is the
  worked case: run a value-level probe of the primitives on both pins, not only
  the tests. `tests/hardening.tcyr` now trips if `f64_to` moves again.
- **Bumping the pin needs a clean re-resolve.** `cyrius lib sync` re-locks at the
  new pin but skips undeclared leaves, so `cyrius deps` then refuses `lib/atomic.cyr`
  and `lib/result.cyr` as DISAGREE. It also leaves old commit rows in the lock, and
  `cyrius deps --verify` checks `lib/` against the lock, not against the pin. Delete
  `cyrius.lock` and any leaf a clean resolve drops, re-run `cyrius deps`, and diff
  `lib/` against `~/.cyrius/versions/<pin>/lib`. At 2.1.0 and 2.1.1 that was just
  `rm lib/*.cyr cyrius.lock; cyrius deps`, with no refusal and no `--relock`, and
  `cyrius deps --locked` (6.7.6) then confirmed `lib/` and the lock are what the tags
  resolve.
- **Read `docs/development/ecosystem-migration.md` at the new tag before bumping.**
  It lists what changes meaning at a pin. Dep guides list consumer-visible changes
  too, but check their floors against what goonj pulls in: hisab 3.4.x states 6.6.3
  for its own consumers, which never need sakshi, while goonj's real floor is 6.7.2.
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
| lint  | ✅ clean (was 29 line-length warnings in `tests/` before 2.1.2) |
| docs  | ✅ complete (was 217 undocumented before 2.1.2) |
| tests | ✅ 42 suites green (37 parity + hardening + integration + dwm_modal + 2 smoke) |
| bench | ✅ 16 suites green |

`cyrius audit` **exits 0** as of 2.1.2, for the first time. It fails on any lint
warning or undocumented public fn, so a non-zero exit is now a real signal. Note
that `tests/goonj.fcyr` counts toward the docs gate, so a glob over `.tcyr` and
`.bcyr` alone misses it. (Note the exit code is easy to lose: `cyrius audit > log; echo $?`
in a compound command reports the `echo`, not the audit.)

## Source

- Rust reference: 14,630 lines across 37 modules, readable at tag `2.1.3`.
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
[`../benchmarks/results.md`](../benchmarks/results.md). Toolchain: cyrius **6.7.18**.

## Tests

One `tests/<module>.tcyr` suite per ported module — including `error` as of
2.0.4, whose Rust tests had covered only the `Display` impl and the `Result`
alias (both dropped by design), leaving it the one module without a suite. Each
suite is ported one-for-one from its module's Rust `#[test]` blocks, minus serde
round-trips (no serde in Cyrius). **37 suites, 3624 assertions, all green.** Run
one with `cyrius test tests/<module>.tcyr`.

Four suites sit outside that per-module set, bringing the totals to
**42 suites / 3801 assertions**:

- **`hardening.tcyr`** (151) — every P-1 defect from the 2.0.2 sweep, the 2.0.3 IR
  sample-count contract, the 2.0.6 `f64_to` and NaN-vertex cases, the 2.1.3 sweep, the
  2.2.1 stack-escape checks and allocation budgets, and the 2.3.0 golden hashes.
  Runs against the shipped `dist/goonj.cyr`, so it also proves each repair reached the
  bundle. A real regression suite: it SIGSEGVs or aborts against 2.0.1 and 2.1.2.
- **`integration.tcyr`** (9) — the six cross-module tests from `rust-old/tests/integration.rs`,
  run against the shipped bundle.
- **`dwm_modal.tcyr`** (4) — the DWM solver's only physics-correctness check
  (first axial mode dominates). Split out because it runs ~15 s.
- **`bundle.tcyr`**, **`goonj.tcyr`** — cross-layer and smoke.

- **`tests/goonj.fcyr`** — 2,000 hostile cases across 13 target groups, with a fixed seed.
  It checks survival, not results, and fails any batch over 2 GiB resident. Run it with
  `--poison`, as CI does: about 15 s, peaking near 0.6 GiB.

The differential and fault-injection harnesses from 2.1.3 needed the Rust oracle and a
patched `alloc`, so they were not kept. ADR-0004 and ADR-0005 record what they found.

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
  (`bench_new`/`bench_run`/`bench_report` for `.bcyr` benchmarks), **simd**
  (`simd_has_avx2`, for the row kernels' 256-bit path).
- **hisab** 3.4.1 — math/geometry; consumed as the single `dist/hisab.cyr`
  bundle (HVec3 and friends). Pulls transitive **sakshi** 2.5.10. goonj calls 19
  hisab functions, none of them `Result`-returning.

`cyrius.lock` pins 31 entries (2 commit-pinned deps + the vendored `lib/`
hashes), down from 32 at 2.0.5: `boxed`, `callback` and `tagged` left with hisab's
old 16-leaf sidecar, and `alloc_cx` and `poison` arrived with 6.6.20.

`dist/goonj.deps` names one leaf, `io`. That is correct, not a loss: from cyrius
6.6.18 the sidecar holds only what the compile-verify proves, and everything else
reaches a consumer through hisab's and sakshi's own `# Requires` blocks.

## Consumers

**A consumer of goonj 2.1.x needs cyrius ≥ 6.7.2.** sakshi declares its syscall
numbers `const` from 2.5.8, and goonj's `logging` pulls sakshi in. Measured on 13
pins from 6.6.3 to 6.7.18 at 2.1.1: below 6.7.2 the build fails with `undefined
variable '_SK_SYS_OPEN'`, and every pin from 6.7.2 builds and runs.

Six repos declare `[deps.goonj]`, all pinned behind 2.1.1 and below that floor (as
of 2026-11-06):

| Repo | goonj | hisab | cyrius |
|------|-------|-------|--------|
| dhvani | 2.0.5 | 2.11.2 | 6.6.10 |
| garjan, ghurni, prani | 2.0.4 | 2.11.2 | 6.6.3 |
| naad, svara | 2.0.4 | 2.22.1 | 6.6.3 |

⚠ **dhvani builds 2.0.5 on cyrius ≥ 6.6.8**, so the memory exhaustion fixed in 2.0.6
is live there until it bumps. A from-scratch consumer builds and runs on 2.1.1, but
whether these six repos' own code does has not been checked from here; see
[`roadmap.md`](roadmap.md#consumer-green). aethersafha does not declare it, and
shruti and kiran/joshua have no `cyrius.cyml` alongside these repos.

## Next

See [`roadmap.md`](roadmap.md). What remains in 2.3.x is f32.

**Every language pattern is now proven** (f32→f64, hex literals, integer errors,
HVec3, manual layout, `Vec`, closures→fnptr+ctx, tuple→struct, bit-ops/xorshift,
caller-supplied randoms). Only **serde** is unhandled — and it's being dropped
(no consumer needs it yet).
