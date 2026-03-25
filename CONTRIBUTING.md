# Contributing to Goonj

## Workflow

1. Fork and clone
2. Create a feature branch
3. Make changes following the development process in CLAUDE.md
4. Run `make check` (fmt + clippy + test + audit)
5. Add tests and benchmarks for new code
6. Submit PR

## Code Style

- `cargo fmt` enforced
- `cargo clippy -- -D warnings` enforced
- `#[non_exhaustive]` on public enums
- `#[must_use]` on pure functions
- No `unwrap()` or `panic!()` in library code
- All physics implementations must include correctness tests with known values

## Testing

- `cargo test --all-features`
- Benchmarks: `./scripts/bench-history.sh`
- Coverage: `make coverage`
