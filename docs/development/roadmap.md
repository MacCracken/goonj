# goonj — Roadmap

> **Forward plan.** The 2.0.0 Rust → Cyrius port is complete; its milestone
> detail is compressed into [Shipped](#shipped) at the bottom rather than
> deleted. Live state lives in [`state.md`](state.md); per-module parity in
> [`port-audit.md`](port-audit.md); release detail in
> [`../../CHANGELOG.md`](../../CHANGELOG.md).
>
> This file is the sequencing — what ships next, in what order, against what
> dependency gates.

## Open gates

- [ ] **Consumer-green** — at least one downstream consumer compiles and passes
      against `dist/goonj.cyr`. The bundle itself has shipped since 2.0.0
      (validated by `tests/bundle.tcyr`, gated in CI by `cyrius distlib --check`);
      what is missing is a consumer. **Gate: dhvani / shruti / kiran being
      ported to Cyrius** — goonj is being worked up the stack, so this is not
      goonj's to close alone.
- [ ] **Retire `rust-old/`** — the frozen oracle is cleared for deletion
      (verified: build, all suites and the bundle check pass with it removed).
      Its benchmark baseline is already archived to
      [`../benchmarks/rust-bench-history.csv`](../benchmarks/rust-bench-history.csv)
      and its only runnable demo ported to [`../../examples/basic.cyr`](../../examples/basic.cyr).
      Deleting it also drops ~3.3 GB of untracked `target/`.
- [ ] **55 undocumented public fns** — the one non-green `cyrius audit` gate,
      and the sole reason `audit` exits 1. It did so at 2.0.0 too, so this is a
      standing debt rather than a regression, but it is closable.

## Post-2.0.0 backlog

- **Review optimizing for various number types.** The port is f64-throughout and
  otherwise unoptimized (naive per-call `alloc`, no arena/pooling, no SIMD). The
  Rust→Cyrius benchmark comparison ([`../benchmarks-rust-vs-cyrius.md`](../benchmarks-rust-vs-cyrius.md))
  is deliberately an honest first pass so it points at exactly this. Cyrius is
  adding **more diverse float/integer types + related SIMD** (driven by model
  training/inference use); once those land, revisit goonj's hot paths (dwm solver,
  ray traversal, wall intersection) to pick appropriate widths (f32 where hisab
  allows, narrower ints for grid indices) and vectorize the inner loops. Expected
  to close much of the current codegen/precision gap. **Gate: Cyrius number-type +
  SIMD support shipping.**
- An arena/pool allocator (or out-param returns) for the alloc-heavy tiny ops
  (`wall_intersection`, per-ray/per-reflection temporaries) — independent of the
  above and doable sooner. Note the alloc-heavy band already improved from
  ~22–130× to ~11–44× between 2.0.0 and 2.0.1, but that came from the Cyrius
  stdlib allocator's single-threaded fast path and a bench-harness rewrite, not
  from goonj changing its allocation strategy. This item is still fully open.

## Out of scope

- New acoustics features beyond the Rust surface (defer to 2.1+).
- Re-architecting module boundaries — parity first; refactor only when the
  ported code demands it (third-instance rule).

## Shipped

### 2.0.x maintenance line

| Release | Date | What |
|---------|------|------|
| **2.0.4** | 2026-08-23 | `rust-old/` parity close-out: 7 behavioural divergences fixed (DVN unsigned modulo, radiosity result aliasing, saturating step counts, gfpe cap + negative index, `GOONJ_LOG`), 6 vacuous tests made falsifiable, `error` given a suite |
| **2.0.3** | 2026-08-23 | Fixed a 2.0.2 regression in `generate_ir` (a negative/NaN `max_time_seconds` allocated ~7.4 GB where Rust yields an empty IR) |
| **2.0.2** | 2026-08-23 | Security/hardening: 11 reachable P-1 defects (4 SIGSEGV, 3 process aborts, 4 silently-wrong results), pinned by `tests/hardening.tcyr` |
| **2.0.1** | 2026-08-22 | Toolchain pin 6.3.14 → 6.5.35, hisab 2.6.7 → 2.11.2, stdlib re-vendored, tree re-canonicalised |
| **2.0.0** | 2026-06-30 | The port itself — all 37 modules, tagged |

### 2.0.0 — the port (M0–M6, complete)

All six release criteria were met except the consumer gate, which is still open
above. Ported bottom-up through the `use crate::X` dependency graph — L0 leaves
(error, propagation, resonance, ambisonics, dark_velvet_noise, scattering,
logging) → L1 `material` (the spine) → L2 geometric/wave (14) → L3 acceleration
and sources (ray, radiosity, image_source, dwm) → L4 energy and metrics
(diffuse, diffraction, analysis, beam) → L5 impulse, coupled → L6 wav, binaural
and the dhvani/kiran/soorat integration APIs. Six parallel worktree-isolated
workflows landed 30 of the 37 modules (the 13-agent L2 batch alone contributed
291 assertions); `ray` and `dwm` were ported solo as the hot paths. Every batch
was re-verified in main — that integration gate, not the agents' self-checks,
is what caught the drift each time.

Close-out: the `dist/goonj.cyr` distlib bundle with a flat-namespace collision
audit (2 clashes de-collided, zero remaining), the two Rust `tracing::warn!`
sites threaded to `goonj_log_warn`, `cyrius vet` clean, and the full 34-benchmark
suite ported to `.bcyr` with a Rust→Cyrius comparison. Per-module detail:
[`port-audit.md`](port-audit.md). Conventions:
[`../adr/0001-rust-to-cyrius-port-conventions.md`](../adr/0001-rust-to-cyrius-port-conventions.md).

### Port challenges, all resolved

Closures → fn-pointer + context (`fncall2` + `TraceCtx`, in `propagation`);
`Vec<T>` → stdlib `vec` (in `resonance`); fixed arrays and String struct fields →
manual `alloc` + offset layout (in `material`); f32 → f64 widening forced by
hisab; `serde` dropped with no consumer needing it; String-payload errors →
integer codes. The signed-integer and float-conversion hazards this port
inherited from Rust's `usize`/`as` semantics — which caused most of the 2.0.2
and 2.0.4 defects — are recorded in
[`../adr/0002-signed-index-and-float-conversion-hazards.md`](../adr/0002-signed-index-and-float-conversion-hazards.md).
