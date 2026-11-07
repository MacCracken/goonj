# Architecture Decision Records

Decisions about goonj — what we chose, the context, and the consequences we accept. Use these when a future reader would reasonably ask *"why did we do it this way?"*

## Conventions

- **Filename**: `NNNN-kebab-case-title.md`, zero-padded to four digits. Never renumber.
- **One decision per ADR.** If a decision supersedes a prior one, add a new ADR and set the old one's status to `Superseded by NNNN`.
- **Status lifecycle**: `Proposed` → `Accepted` → (optionally) `Superseded` or `Deprecated`.
- Use [`template.md`](template.md) as the starting point.

## ADR vs. architecture note vs. guide

| Kind | Lives in | Answers |
|---|---|---|
| ADR | `docs/adr/` | *Why did we choose X over Y?* |
| Architecture note | `docs/architecture/` | *What non-obvious constraint is true about the code?* |
| Guide | `docs/guides/` | *How do I do X?* |

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [0001](0001-rust-to-cyrius-port-conventions.md) | Rust → Cyrius port conventions | Accepted |
| [0002](0002-signed-index-and-float-conversion-hazards.md) | Signed-index and float-conversion hazards inherited from the Rust port | Accepted |
| [0003](0003-f64-to-saturates-keep-bounding-in-f64.md) | `f64_to` saturates from cyrius 6.6.8; keep bounding in f64, and cap where Rust had no ceiling | Accepted |
| [0004](0004-allocation-refusal-ends-the-process.md) | A refused allocation ends the process | Accepted |
| [0005](0005-hostile-input-parity-and-recorded-gaps.md) | What hostile input should do, and where the port stays different | Accepted |
| [0006](0006-retire-rust-old-keep-its-bar.md) | Retire `rust-old/`, and keep its bar | Accepted |
| [0007](0007-stack-buffers-over-an-arena.md) | Hot paths use stack buffers, not an arena | Accepted |
| [0008](0008-row-kernels-for-grid-solvers.md) | Grid solvers run as row kernels over `f64v_*` | Accepted |
