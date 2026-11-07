# 0006 — Retire `rust-old/`, and keep its bar

**Status**: Accepted
**Date**: 2026-11-06

## Context

`rust-old/` held the original Rust engine, 49 tracked files plus 3.5 GB of ignored
build output, as the port's parity oracle. 2.1.3 was its last planned use. Nothing in
the build, tests, benches or bundle reads it, but "nothing reads it" was also true of
six tests nobody had ported, so the deletion was checked rather than assumed:

| Check | Result |
|---|---|
| Every gate in a clean tree without it | build, 42 suites, `cyrius audit`, `distlib --check`, fuzz and the example all pass |
| Oracle rebuilds from the tag | `cargo test --all-features` passes all 518 unit and 6 integration tests |
| Files found only there | `benches/benchmarks.rs` and the Rust-only tool configs, all kept by the tag |
| Tests never ported | `tests/integration.rs`: 6 tests, now `tests/integration.tcyr` |
| Final comparison on valid input | below |

The 2.1.3 differential fed mostly hostile values, and the solvers reject most of those,
so their maths on valid input had barely been compared. Before deleting, 26 realistic
scenes ran on both sides with 4 configurations each: the FDTD, DWM and GFPE solvers,
diffusion, whole impulse responses, analysis metrics, binaural, the integration modules,
radiosity, FDN reset, and more.

Every value agreed to within 1e-4 relative, except analysis outputs fed by three
impulse responses. Dumping those in full showed 6 of 16,000 samples differ, each a
single reflection whose arrival rounds to the adjacent sample in f32. That is the same
f32 versus f64 difference [ADR-0005](0005-hostile-input-parity-and-recorded-gaps.md)
already records, not a defect.

## Decision

**Delete the tree from `main`. Keep its bar.** The port's correctness bar is still
"matches what Rust did, and diverge only with an ADR". What changes is how to check it.

- **The Rust source stays readable at tag `2.1.3`**, the last tag that has it:
  `git show 2.1.3:rust-old/src/<module>.rs`.
- **The `rust-old/src/x.rs:NN` citations in `src/` and `tests/` stay.** They are the
  port's audit trail, and they resolve at that tag.
- **Day to day, the bar is what the oracle left behind:** the parity assertions in
  `tests/*.tcyr`, `tests/hardening.tcyr`, and the rules in ADR-0002 and ADR-0005.
- **To run the oracle again:**

  ```sh
  mkdir /tmp/oracle
  git archive 2.1.3 rust-old | tar -x -C /tmp/oracle
  cd /tmp/oracle/rust-old && cargo test --all-features
  ```

## Consequences

- **Positive** — The tree is one language, and a full clone no longer carries Rust that
  nothing builds.
- **Positive** — The six cross-module tests now run on every change. In the Rust crate
  they sat outside `src/`, which is how the module-by-module port missed them.
- **Negative** — A parity question now costs a tag extraction, so it will get asked less.
  New features have no oracle at all, which is true of anything past the Rust surface.
- **Negative** — `Cargo.lock` was git-ignored and went with the tree. The tested build
  used hisab 1.4.0 and glam 0.29.3. A rebuild resolves from the crate registry again, so
  it depends on those versions staying available.
- **Neutral** — The Rust benchmark figures stay a frozen record in
  [`rust-bench-history.csv`](../benchmarks/rust-bench-history.csv). They can be re-run
  only from the tag.
- **Neutral** — The 2.1.3 differential harness was not kept, and depends on the oracle.
  ADR-0005 records what it found, and the recipe above restores what it needs.

## Alternatives considered

- **Keep it in the tree** — rejected. It was only ever scheduled to stay until the sweeps
  finished, and a frozen second language invites edits and stale reading.
- **Move it to a branch or a separate repo** — rejected. The tag already does that, with
  nothing to maintain.
- **Commit golden outputs from the final comparison** — rejected. The suites already pin
  what matters, and raw f32 outputs would be re-litigated against every f64 rounding.
