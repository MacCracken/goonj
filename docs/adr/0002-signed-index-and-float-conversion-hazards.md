# 0002 — Signed-index and float-conversion hazards inherited from the Rust port

**Status**: Accepted
**Date**: 2026-08-23

## Context

[ADR-0001](0001-rust-to-cyrius-port-conventions.md) settled the *shape* of the
Rust → Cyrius port. It did not anticipate a hazard class that only surfaced once
the ported code was audited against hostile input, in the 2.0.2 and 2.0.4 sweeps.
Between them those two releases fixed **18 defects, of which 13 share this single
root cause** — four SIGSEGVs, three process aborts, and the rest silently-wrong
results, all reachable through the public API with no unsafe usage by the caller.

Rust encodes two guarantees in its type system that Cyrius has no equivalent for.
Porting the *code* faithfully therefore dropped them silently, because the ported
line looks correct:

1. **`usize` / `u32` cannot be negative.** So Rust idiomatically writes a bounds
   check as a SINGLE upper-bound test — `if source.ix >= config.nx`, or
   `if rx.ix < nx && rx.iy < ny`. That is a *complete* check in Rust. Ported to
   signed `i64` it keeps only half the bound, and the index then reaches raw
   `load64`/`store64`. `fdtd_receiver_new(-4000000, 0)` → `solve_fdtd_2d` was a
   reproducible SIGSEGV.
2. **Rust's float → int `as` casts SATURATE** (since Rust 1.45): `NaN` → 0,
   negative → 0, overflow → `MAX`. **Cyrius `f64_to` does not** — it returns
   `INT64_MIN` for `NaN`, `±Inf` and anything outside `i64`, and otherwise
   truncates toward zero, keeping the sign. So
   `let i = (x * rate) as usize; if i < len` is a complete bound in Rust and an
   incomplete one here.

Two narrower members of the same family:

3. **`saturating_mul` for memory caps.** `nx.saturating_mul(ny) > MAX_CELLS`
   became a plain wrapping `nx * ny`, so the cap was bypassed by overflow.
4. **Unsigned modulo.** `(r as usize) % n` on a `u64` draw is an unsigned
   modulo. Applying C truncated modulo to the same bits as a signed `i64` and
   correcting negatives computes `(r − 2⁶⁴) mod n`, which equals the unsigned
   result only when `n` is a power of two. `dark_velvet_noise` used `n` = 24 and
   12, so every draw with the top bit set landed on the wrong pulse position —
   wrong output for years, with every statistical test still passing.

## Decision

Treat these as standing conventions, checked in review and in new code:

1. **Every index guard checks BOTH bounds.** Where the Rust oracle had one
   `>=` or `<` test on a `usize`, the Cyrius port needs two. Cite the Rust
   line in a comment so the reason survives (`rust-old/src/fdtd.rs:77-79` —
   both `usize`). The house idiom is a flat guard, not deep nesting:
   `var ok = 1; if (i < 0) { ok = 0; } if (i >= n) { ok = 0; }`.
2. **Never feed `f64_to` a value you have not already bounded in f64.** Clamp
   on the float side first, so `NaN`/`±Inf`/out-of-range are handled explicitly
   and Rust's saturating semantics are reproduced:

   ```
   var q = f64_div(a, b);
   var n = 0;                                          # NaN and negatives -> 0
   if (f64_ge(q, f64_from(CAP)) == 1) { n = CAP; }     # +Inf and overflow -> CAP
   if (f64_gt(q, 0) == 1) {
       if (f64_lt(q, f64_from(CAP)) == 1) { n = f64_to(q); }
   }
   ```

   Getting the sentinel mapping *backwards* is expensive in the other
   direction: 2.0.2 mapped it to the cap instead of zero and turned a Rust
   no-op into a 7.4 GB allocation (fixed in 2.0.3).
3. **Bound each operand before multiplying** anything that gates an allocation,
   so the product cannot wrap past its own cap.
4. **NaN must be rejected explicitly, not by negating a range test.** Rust's
   `!(0.0..=1.0).contains(&x)` is TRUE for `NaN`, so Rust *rejects* it.
   Rewriting that as two one-sided rejections (`if x < 0 reject; if x > 1
   reject`) inverts it, because `f64_lt`/`f64_gt` are both false for `NaN`.
   Use the NaN-correct `f64_ge`/`f64_le` and test `== 0`.

## Consequences

- **Positive** — The class is now named, so it is greppable and reviewable
  rather than rediscovered per module. `tests/hardening.tcyr` pins every
  instance found so far and fails loudly against the versions that had them.
  Severity is bounded by knowing which primitive is in play: `vec_get`/`vec_set`
  bounds-check and abort (fail-fast), whereas raw `load64`/`store64` corrupt
  memory silently.
- **Negative** — Cyrius guards are wordier than the Rust they replace, and the
  duplication is unavoidable without an unsigned type or a saturating
  conversion in the language. Some guards are strictly *stricter* than the
  oracle, which is a deliberate, documented divergence.
- **Neutral** — The oracle that made this class discoverable (`rust-old/`) is
  being retired. That is precisely why it is written down here: after deletion
  the Rust signatures are only reachable as `git show 2.0.2:rust-old/src/<m>.rs`.

## Alternatives considered

- **Audit once and move on, without an ADR** — rejected: 2.0.2 swept `fdtd` and
  `dwm` for exactly this and missed `gfpe`, which 2.0.4 then had to fix. An
  unnamed class does not get swept completely.
- **A shared bounds-check helper** — rejected for now: the flat namespace makes
  one generic helper awkward across 37 modules, the guards differ in arity and
  in what they return on failure (empty result vs sentinel vs skip), and the
  explicit form keeps the Rust citation next to the check.
- **Wrap `f64_to` in a saturating `f64_to_sat`** — attractive, and worth
  revisiting if a fourth site appears; deferred because the correct saturation
  target is call-site-specific (0, a cap, or an error return), so a single
  helper would hide the decision that actually matters.
