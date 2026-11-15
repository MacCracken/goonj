# rust-parity

Checks goonj's grid solvers (`solve_fdtd_2d`, `solve_dwm_3d`, `solve_diffusion_2d`) against
the Rust original, bit for bit. See [ADR-0010](../../docs/adr/0010-f32-grids-match-rust.md).

```sh
scripts/rust-parity/run.sh            # the fixed seed
scripts/rust-parity/run.sh 1 2 3      # more seeds
```

It extracts `rust-old` at tag 2.1.3 into `build/rust-parity/` and needs cargo. Run
`cyrius distlib` first: it tests `dist/goonj.cyr`.

- `gen.py` writes about 50 cases per seed, once as Rust (`cases_gen.rs`) and once as Cyrius.
  Every float is given as an f32 bit pattern, so both sides get the same input.
- `f32dump.rs` and `harness_*.cyr.in` run them and print every output as f32 bits.
- `compare.py` reports each case that differs, by how many ULPs, and in which arrays.
- `hsweeps.rs` runs `tests/hardening.tcyr`'s golden sweeps through Rust. Its hashes are the
  ones that test pins.

**What to expect:** every case bit-identical, except a rare case whose arrays include
`signal`. That is a Gaussian source sample where Rust's `expf` (glibc on Linux) is not
correctly rounded and goonj is, and the mesh carries the one-ULP difference.
