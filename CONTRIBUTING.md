# Contributing to Goonj

Goonj is a [Cyrius](https://github.com/MacCracken/cyrius) library (v2.0.0),
compiled by `cycc`. There is no cargo/Rust toolchain — the Rust source at
`rust-old/` is a frozen parity oracle only.

## Workflow

1. Fork and clone
2. Create a feature branch
3. Make changes following the development process in CLAUDE.md
4. `cyrius deps` to resolve dependencies, then run the affected suites:
   `cyrius test tests/<suite>.tcyr` (one `.tcyr` per module; 36 suites, 3,585
   parity assertions, all green)
5. Add tests (`tests/<module>.tcyr`) and, for hot paths, benchmarks
   (`tests/<module>.bcyr`) for new code; cross-check behavior against `rust-old/`
6. Submit PR (the maintainer handles all git — commit, push, tag)

## Code style

- Format: `cyrius fmt <file.cyr> --check` must be clean (`--check` exits 1 with
  empty output on 6.3.x — verify canonically by diffing `cyrius fmt <file>`
  against the file)
- Lint: `cyrius lint <file.cyr>` — 0 warnings, 0 untracked deferrals; no line >120 chars
- `#must_use` on pure functions; `#derive(accessors)` for struct field accessors
- Integer error codes (see `src/error.cyr`) — no exceptions/panics
- Never `unwrap`/`panic`; f32 → f64 throughout (hisab's `HVec3` is f64-only)
- All physics implementations must include correctness tests with known values

## Testing & benchmarks

- Tests: `cyrius test tests/<suite>.tcyr` (`tests/*.tcyr`, ported one-for-one
  from the Rust `#[test]` blocks)
- Benchmarks: `cyrius bench tests/<suite>.bcyr` (`tests/*.bcyr`, 15 modules); see
  [`docs/benchmarks-rust-vs-cyrius.md`](docs/benchmarks-rust-vs-cyrius.md)
- Distlib: after touching `src/*.cyr` or the `[lib]` list, regenerate the bundle
  with `cyrius distlib` and keep it collision-free

The correctness bar is the parity assertions in `tests/*.tcyr` reproducing the
`rust-old/` oracle's behavior.
