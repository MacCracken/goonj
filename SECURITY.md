# Security Policy

## Supported Versions

| Version | Supported | Notes |
|---------|-----------|-------|
| 2.3.0   | ✅ Yes    | Current. See **Known limits**. |
| 2.2.1   | ✅ Yes    | |
| 2.2.0   | ✅ Yes    | Upgrade for a long-lived process: temporaries are never reclaimed, about 3.6 MB per 1 s impulse response, and a long `solve_radiosity` can run out of memory. |
| 2.1.3   | ✅ Yes    | Closes the 2.0.6 limit down to an address-space requirement. |
| 2.1.0–2.1.2 | ⚠️ Upgrade | Public band accessors, including `mbr_set_energy_band`, read and write out of bounds on a bad index, and carry the 2.0.6 limit. Fixed in 2.1.3. |
| 2.0.6   | ⚠️ Upgrade | A non-finite or overflowing room dimension reaching `fdn_new` costs about 9.3 GiB and aborts where that is unavailable. Carries the 2.1.3 band accessor defects too. |
| 2.0.3–2.0.5 | ⚠️ Upgrade if built on cyrius ≥ 6.6.8 | `+Inf` or overflowing input to the FDTD/DWM gaussian pulses or the FDN exhausts memory without limit. Safe below 6.6.8. Fixed for the pulses and bounded for the FDN in 2.0.6. |
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
  `f64_to` does not clamp the way Rust's `as usize` casts do. Both are documented in
  [ADR-0002](docs/adr/0002-signed-index-and-float-conversion-hazards.md), and
  [ADR-0003](docs/adr/0003-f64-to-saturates-keep-bounding-in-f64.md) covers what
  changed at cyrius 6.6.8.
- **A refused allocation ends the process**, as it does in Rust, by
  `goonj: allocation refused` or a fault at the null page
  ([ADR-0004](docs/adr/0004-allocation-refusal-ends-the-process.md)). It never gives a
  partial result. Only the WAV writers report it, as `ERR_COMPUTATION`.
- **A 0 handle faults at the null page.** Handles come from goonj's own constructors,
  so this is a caller bug, not input. Two constructors do return 0 for rejected input,
  `acoustic_material_new` and `metamaterial_to_acoustic_material`, and that 0 must be
  checked before use.
- **Where the port deliberately differs from Rust on hostile input** is listed in
  [ADR-0005](docs/adr/0005-hostile-input-parity-and-recorded-gaps.md).
- **Out of scope**: resource use on inputs a consumer legitimately chose (a large
  grid really does allocate a large grid), and the bump allocator never reclaiming
  within a run — both are documented behaviour, not vulnerabilities.

## Known limits

- **`fdn_new` on a non-finite or overflowing room dimension needs about 5.5 GiB of
  address space**, and takes under 25 ms and 18 MiB of it resident, because a delay line
  costs nothing until it is used, and `fdn_reset` clears only what was written. Under a
  tighter address-space limit it stops with
  `goonj: allocation refused`. Reject non-finite dimensions first if a consumer forwards
  them from untrusted input.
