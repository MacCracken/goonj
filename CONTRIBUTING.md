# Contributing to Goonj

Goonj is a [Cyrius](https://github.com/MacCracken/cyrius) library (v2.0.2),
compiled by `cycc` (toolchain pinned in `cyrius.cyml`). There is no cargo/Rust
toolchain — the Rust source at `rust-old/` is a frozen parity oracle only.

## Workflow

1. Fork and clone
2. Create a feature branch
3. Make changes following the development process in CLAUDE.md
4. `cyrius deps` to resolve dependencies, then run the affected suites:
   `cyrius test tests/<suite>.tcyr` (one `.tcyr` per module; 36 suites, 3,585
   parity assertions, all green). Security-relevant changes should also keep
   `cyrius test tests/hardening.tcyr` green — it pins the 2.0.2 P-1 fixes
   (negative indices, overflow caps, NaN validation) against the shipped bundle.
5. Add tests (`tests/<module>.tcyr`) and, for hot paths, benchmarks
   (`tests/<module>.bcyr`) for new code; cross-check behavior against `rust-old/`
6. Submit PR (the maintainer handles all git — commit, push, tag)

## Code style

- Format: the tree must match `cyrfmt` canonical output. Check one file with
  `diff <(cyrfmt src/x.cyr) src/x.cyr` — `cyrfmt` prints the canonical form to
  stdout and never modifies the file. **Do not** use `cyrius fmt <file>` to
  inspect: on 6.5.x it prints nothing and rewrites the file *in place*. **Do
  not** trust `cyrius fmt --check` or `--dry` either: both report a
  non-canonical file as clean, so neither can gate. The project-wide gate is
  `cyrius audit`, whose fmt section does list drifted files.
- Lint: `cyrius lint <file.cyr>` — 0 warnings, 0 untracked deferrals; no line >120 chars
- `#must_use` on pure functions; `#derive(accessors)` for struct field accessors
- Integer error codes (see `src/error.cyr`) — no exceptions/panics
- Never `unwrap`/`panic`; f32 → f64 throughout (hisab's `HVec3` is f64-only)
- All physics implementations must include correctness tests with known values

## Testing & benchmarks

- Tests: `cyrius test tests/<suite>.tcyr` (`tests/*.tcyr`, ported one-for-one
  from the Rust `#[test]` blocks)
- Example: `cyrius build examples/basic.cyr build/basic && ./build/basic` — it
  compiles against `dist/goonj.cyr`, so it also catches a bundle that has drifted
  from `src/` (run `cyrius distlib` first if you changed a module)
- Benchmarks: `cyrius bench tests/<suite>.bcyr` (`tests/*.bcyr`, 16 suites); see
  [`docs/benchmarks-rust-vs-cyrius.md`](docs/benchmarks-rust-vs-cyrius.md)
- Distlib: after touching `src/*.cyr` or the `[lib]` list, regenerate the bundle
  with `cyrius distlib` and keep it collision-free

The correctness bar is the parity assertions in `tests/*.tcyr` reproducing the
`rust-old/` oracle's behavior.
