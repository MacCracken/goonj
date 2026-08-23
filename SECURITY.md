# Security Policy

## Supported Versions

| Version | Supported | Notes |
|---------|-----------|-------|
| 2.0.4   | ✅ Yes    | Current. |
| 2.0.3   | ✅ Yes    | First release free of the known P-1 defects below. |
| 2.0.2   | ⚠️ Upgrade | Fixed 11 P-1 defects but introduced a memory-exhaustion regression in `generate_ir` (a negative or NaN `max_time_seconds` allocates ~7.4 GB). Fixed in 2.0.3. |
| 2.0.0–2.0.1 | ❌ No | Carry 11 defects reachable through the public API — 4 out-of-bounds accesses (SIGSEGV), 3 process aborts, and 4 silently-wrong results. Fixed in 2.0.2. |
| 1.x (Rust) | ❌ No | The pre-port Rust line. Superseded by 2.0.0. |

**If you are on 2.0.0, 2.0.1 or 2.0.2, upgrade.** The 2.0.0/2.0.1 defects need
no unusual usage to trigger — a negative grid index passed to a solver
constructor is enough. Detail: the [2.0.2](CHANGELOG.md) and 2.0.3 changelog
entries; the regression suite that pins them is `tests/hardening.tcyr`.

## Reporting

Report security issues privately to the repository maintainer — do not open a
public issue, and do not include a working trigger in a public channel.

## Scope

goonj is a Cyrius computation library. It opens no sockets and does no file I/O
of its own: `wav` builds an in-memory byte buffer and hands it back, and it is
the consumer that writes it. The realistic attack surface is therefore **input
a consumer forwards from somewhere less trusted** — geometry, grid dimensions,
sample rates, indices, coefficients and durations.

What that means in practice, and what to expect from a report:

- **Memory safety is the top severity.** The library does manual memory
  management (`alloc` + `load64`/`store64`) with named offset constants. Raw
  `load64`/`store64` on a bad index corrupts memory silently; `vec_get`/`vec_set`
  bounds-check and abort the process instead. An unvalidated path reaching the
  former is a higher-severity finding than one reaching the latter, and both are
  in scope.
- **Rejecting bad input is a contract, not a nicety.** Degenerate values —
  negative counts and indices, zero or negative dimensions and sample rates,
  `NaN`/`±Inf`, out-of-domain coefficients — must produce an error code or a
  documented degenerate result, never a wild write, a hang, or a plausible-looking
  wrong answer.
- **Two porting hazards account for most defects found so far** and are the first
  place to look: the Rust original's `usize` indices became signed `i64` here, so
  a guard written as a single upper-bound test is only half a bound; and
  `f64_to` does not saturate the way Rust's `as` casts do. Both are documented in
  [ADR-0002](docs/adr/0002-signed-index-and-float-conversion-hazards.md).
- **Out of scope**: resource use on inputs a consumer legitimately chose (a large
  grid really does allocate a large grid), and the bump allocator never reclaiming
  within a run — both are documented behaviour, not vulnerabilities.
