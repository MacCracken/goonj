# 0001 — Rust → Cyrius port conventions

**Status**: Accepted
**Date**: 2026-06-30

## Context

goonj shipped through 1.4.3 as a Rust library (14,630 lines, 37 modules,
heavy f32 acoustics math, `hisab::Vec3` for geometry). It is being rewritten
in Cyrius (sovereign systems language, compiled by `cycc`) and released as
**2.0.0**. Cyrius differs from Rust in ways that force several cross-cutting
choices *before* the bulk of modules are ported — getting them right once
avoids re-porting 35 modules later. The dependency hisab has already been
ported to Cyrius and is the de-facto reference for idiom.

Forcing constraints:

- Cyrius has no generics, no traits, no `enum` with payloads, no closures as
  values, and no `serde`. Floats are f64 bit patterns; there is no f32 math.
- hisab's `HVec3` is **f64-only**. goonj's geometry types come from hisab.
- The Rust source is the correctness oracle, frozen at `rust-old/`.

## Decision

Adopt these conventions for the whole port (recorded in
`docs/development/port-audit.md` as the working reference):

1. **f32 → f64 everywhere.** Forced by hisab; also improves precision.
2. **`hisab::Vec3` → hisab `HVec3`**, consumed via the single `dist/hisab.cyr`
   bundle (`[deps.hisab]`, tag-pinned, `cyrius.lock`-locked) — the same way
   hisab consumes its own deps.
3. **Error enums → integer codes** (`src/error.cyr`): functions return `0`
   (`ERR_NONE`) or a negative `ERR_*`. String payloads are dropped; static
   names come from `goonj_err_name`.
4. **Float literals**: integers via `f64_from(n)`; non-integers as named
   module-top `var` constants holding the IEEE-754 hex bit pattern with the
   decimal in a comment.
5. **Structs** via `#derive(accessors)` + `alloc(sizeof(T))`; methods become
   free functions `type_verb(self, …)`. Module files don't `include` each
   other — the build/test entry orders includes.
6. **Validation by `tests/*.tcyr`** ported one-for-one from each module's Rust
   `#[test]` blocks; the smoke `src/main.cyr` does not include the library
   (hisab's layout). Tolerances loosen vs the f32 oracle where bit-exactness
   isn't meaningful.

Deferred to per-occurrence ADRs (not settled here): **closures** (`speed_fn`,
ray-tracer callbacks) → fnptr/callback pattern; **`Vec<T>`** return values →
`vec.cyr` vs length-prefixed `alloc` buffers; **RNG** for stochastic modules.

## Consequences

- **Positive** — One consistent idiom across 37 modules; the recipe is proven
  (error + propagation, 35 green tests) before fan-out. Owning the math stack
  end-to-end (hisab + cyrius stdlib, no external deps).
- **Negative** — Verbose float literals and manual struct plumbing vs Rust's
  ergonomics. Lost: serde, payload-carrying error context, compile-time
  generics. f64 widening means tests can't assert f32-bit-exactness.
- **Neutral** — A python one-liner generates hex bit patterns during porting.
  The closure/`Vec<T>`/RNG decisions still need their own ADRs as they arise.

## Alternatives considered

- **Keep f32** — rejected: hisab is f64-only; mixing would mean reimplementing
  vector math or constant f32↔f64 conversion at every hisab boundary.
- **Vendor `dist/hisab.cyr` into the repo** instead of a git dep — rejected:
  the tag-pinned + locked git dep matches the ecosystem and tracks hisab
  releases; vendoring would drift silently. (`cyrius deps` resolves it.)
- **Error structs carrying a heap string** — rejected: every call site would
  allocate/free; integer codes match hisab and the stdlib `result` idiom.
- **One monolithic `src/main.cyr`** (the port tool's suggested path) —
  rejected: per-module files mirror `rust-old/` for diffable parity and keep
  the distlib bundle layout open.
