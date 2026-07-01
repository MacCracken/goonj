# Getting started with goonj

## Build

```sh
cyrius deps                              # resolve hisab (+ transitive sakshi) into lib/
cyrius build src/main.cyr build/goonj    # compile the smoke binary
cyrius test tests/propagation.tcyr       # run ONE parity suite (explicit path)
cyrius bench tests/dwm.bcyr             # run a benchmark
cyrius distlib                           # regenerate dist/goonj.cyr (the bundle)
```

`cyrius test` takes an explicit suite path; there is no auto-discovery. Run each
`tests/<module>.tcyr` by name (36 suites, 3,585 parity assertions).

## Layout

- `src/main.cyr` — smoke binary only (prints and exits; does **not** include the
  library). Top-level `var r = main(); syscall(SYS_EXIT, r);`.
- `src/*.cyr` — the 37 acoustics modules (the library). Self-contained (no
  `include` lines); stdlib + hisab resolve from `cyrius.cyml`. Validated by
  `tests/*.tcyr`, not by the smoke binary.
- `tests/*.tcyr` — one parity suite per module; `tests/*.bcyr` — benchmarks.
- `dist/goonj.cyr` — the distlib bundle (all 37 modules concatenated; see
  `[lib]` in `cyrius.cyml`). Regenerate with `cyrius distlib`.
- `rust-old/` — the original Rust source, frozen as the parity oracle. Do not
  modify.

## Adding / changing a module

1. Edit `src/<module>.cyr` (or add a new module file).
2. Cross-check parity against `rust-old/src/<module>.rs`.
3. Add/extend `tests/<module>.tcyr` (ported from the Rust `#[test]` blocks) and
   run `cyrius test tests/<module>.tcyr` — it must print `N passed, 0 failed`.
4. `cyrius fmt` canonical + `cyrius lint` clean; if you touched a bundled module,
   `cyrius distlib` and confirm the bundle stays collision-free.
5. Bump `VERSION` + add a CHANGELOG entry before tagging (the maintainer tags).

See [`../adr/template.md`](../adr/template.md) when a non-trivial design choice
deserves an ADR, and [`../development/state.md`](../development/state.md) for live state.
