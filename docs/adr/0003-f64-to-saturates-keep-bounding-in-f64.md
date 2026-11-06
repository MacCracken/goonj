# 0003 — `f64_to` saturates from cyrius 6.6.8; keep bounding in f64, and cap where Rust had no ceiling

**Status**: Accepted
**Date**: 2026-11-06

> **Note, 2.1.3.** The cap and the decision stand, but the cost below is out of date.
> `fdn_new` now takes each line from `alloc`, whose bytes are zero, instead of filling a
> vec. `+Inf` through it takes under 25 ms, 18 MiB resident and about 5.5 GiB of address
> space, which is what Rust's `vec![0.0; n]` does with calloc. A process limited below
> that stops with `goonj: allocation refused`
> ([ADR-0004](0004-allocation-refusal-ends-the-process.md)).

## Context

[ADR-0002](0002-signed-index-and-float-conversion-hazards.md) was written against
a `f64_to` that returned `INT64_MIN` for `NaN`, `±Inf` and anything outside `i64`.
**cyrius 6.6.8 changed that.** Measured on both pins:

| Input | 6.6.2 | 6.6.20 | Rust `as i64` |
|---|---|---|---|
| `NaN` | `INT64_MIN` | `0` | `0` |
| `+Inf`, ≥ 2⁶³ | `INT64_MIN` | `INT64_MAX` | `i64::MAX` |
| `-Inf`, ≤ −2⁶³ | `INT64_MIN` | `INT64_MIN` | `i64::MIN` |
| `-1.5` | `-1` | `-1` | `-1` |

So `f64_to` now matches Rust's signed casts. It does not match the `as usize` and
`as u32` casts the oracle mostly used, which also send negatives to 0, so
ADR-0002's first hazard stands: `f64_to` keeps the sign.

The change was not a free win. Three sites had broken ADR-0002's rule 2 and got
away with it, because they wrote only a floor:

```
var n = f64_to(unbounded);
if (n < 1) { n = 1; }
```

`+Inf` arrived as `INT64_MIN`, hit the floor, and became a harmless 1. On 6.6.20
it arrives as `INT64_MAX`, passes the floor, and drives a `vec_push` loop until
the process is out of memory:

| Site | Input | 2.0.5 on 6.6.2 | Same source on 6.6.20 |
|---|---|---|---|
| `fdtd_source_gaussian_pulse` | `sigma_steps = +Inf` | exit 0, instant | `vec: alloc failed` |
| `dwm_source_gaussian_pulse` | `sigma_steps = +Inf` | exit 0, instant | `vec: alloc failed` |
| `fdn_new` | room dimension `+Inf` or `1e30` | exit 0, instant | `vec: alloc failed` |

**All 3667 assertions stayed green**: nothing exercised these inputs. Any consumer
that builds a goonj older than 2.0.6 on cyrius ≥ 6.6.8 has this path live.

## Decision

**ADR-0002 rule 2 stands unchanged: bound in f64 before calling `f64_to`.**
Saturation does not relax it. A saturated `INT64_MAX` is no safer than the old
sentinel, it only fails in the opposite direction. The saturation target is still
call-site-specific (0, a cap, or an error return), and the sign still survives.

The three sites now follow that rule. Each maps NaN and anything below 1 to 1, as
Rust's `.max(1)` does, and maps `+Inf` and overflow to a ceiling:

- **Gaussian pulses cap at the solver's step limit** (`MAX_TIME_STEPS`,
  `DWM_MAX_TIME_STEPS`, both 1,000,000). Rust has no ceiling here and panics on
  capacity overflow. The cap is unobservable through a solver, which runs at most
  that many steps and reads `signal` only at `step < num_steps`. It is observable
  through `vec_len(signal)` for a span past the cap.
- **FDN delay lines cap at `FDN_MAX_DELAY_SAMPLES = 115,200,000`**, the oracle's own
  `MAX_IR_SAMPLES` (192 kHz × 600 s). This is a real divergence from the oracle,
  recorded here because the port's rule requires it. Rust's `as u32` saturates at
  `u32::MAX`, a 4.29-billion-sample line. goonj clamps instead, so any delay line
  past the cap differs from Rust: at 48 kHz that is a dimension over about 412 km,
  at 192 kHz about 103 km. Results below the cap are bit-identical to the previous
  behaviour.

  **The value favours Rust parity over memory, so the FDN case is bounded, not
  closed.** A non-finite dimension caps five of the eight lines, and `fdn_new` then
  takes about 8 s and peaks at 9.3 GiB. Where that much memory is available it
  completes. Where it is not, it still ends in `vec: alloc failed`, measured under
  4 GB and 8 GB address-space limits. The two gaussian pulses have no such residue.

`tests/hardening.tcyr` pins the three sites and carries a tripwire on `f64_to`'s
semantics, so that a toolchain moving them again fails a test rather than passing
silently. **On any such failure, audit every `f64_to` site.** A green suite is no
evidence: this is exactly how the regression got through.

## Consequences

- **Positive** — Rust parity improves for free at the other sites: `NaN` delays
  now land at index 0, as `NaN as usize` does, and `dark_velvet_noise` keeps one
  pulse for an absurd cell size where it used to fall back to one per sample.
- **Positive** — The hazard is pinned, not just documented.
- **Negative** — `+Inf` or an overflowing dimension reaching `fdn_new` still costs
  9.3 GiB, and still aborts a process that cannot have it. A consumer forwarding
  untrusted room dimensions must reject non-finite values before calling it.
- **Negative** — That allocation is too heavy to run on every change, so
  `tests/hardening.tcyr` pins the delay length in the config and not the call to
  `fdn_new`. An uncapped length still fails that assertion.
- **Negative** — A second number now has to be remembered at the FDN, and its value
  is a judgement.
- **Neutral** — Comments across `src/` that stated the old behaviour in the
  present tense were rewritten. The guards they explain all stay.
- **Neutral** — `dark_velvet_noise` still reaches `MAX_IR_SAMPLES` (115.2M samples)
  for `duration_seconds = +Inf`. That matches the oracle's `.min(MAX_IR_SAMPLES)`,
  so it is left alone, and SECURITY.md already treats large allocations at
  documented caps as out of scope.

## Alternatives considered

- **FDN cap at 1,000,000, the oracle's solver step limit** — built and measured,
  then rejected at maintainer review. It closes the regression on any machine
  (`+Inf` through `fdn_new` takes 0.065 s and 82 MiB), but it diverges from Rust
  from a 3.6 km dimension at 48 kHz and 890 m at 192 kHz, and the port's bar is
  matching Rust.
- **FDN cap at 11,520,000 (192 kHz × 60 s)** — measured, not adopted. It is the
  natural middle: 0.82 s and 1.06 GiB on the same input, with Rust parity to about
  41 km at 48 kHz and 10 km at 192 kHz. It is the first value to reach for if the
  9.3 GiB peak proves a problem in practice, and only the constant would change.
- **FDN cap at `u32::MAX`, matching Rust exactly** — rejected. Through `vec_push`
  that is about 34 GB per line, so it reproduces the regression.
- **Restore the old result, mapping non-finite lengths to 1** — rejected. It
  reproduces an accident: nothing but the old sentinel ever chose 1, and finite
  values just inside `i64` would still run away.
- **Wrap `f64_to` in a saturating helper** — rejected for the reason ADR-0002
  gave: the right target differs per site, which is the decision that matters.
- **Supersede ADR-0002** — rejected. Its rules are all still correct and only one
  stated fact moved, so it stays Accepted with a pointer here.
