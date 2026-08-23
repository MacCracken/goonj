# Architecture notes

Non-obvious constraints, quirks, and invariants that a reader cannot derive from the code alone. Numbered chronologically — never renumber.

Not decisions (those live in [`../adr/`](../adr/)) and not guides (those live in [`../guides/`](../guides/)). An item here describes *how the world is*, not *what we chose* or *how to do something*.

## Items

- [`overview.md`](overview.md) — module map by dependency layer, the flat-namespace
  and bundle constraints, and what `cyrius distlib` concatenates.

Add a numbered entry (`001-kebab-case-title.md`) the next time the code has a
non-obvious invariant a reader can't derive. Do not write entries for decisions —
those are ADRs.
