# `DH_SIZE` collides with yukti's, and `generate_dhvani_ir` then overruns its allocation by 64 bytes

**Status:** ✅ **FIXED in 2.3.2** (2026-11-15). See [Resolution](#resolution).
**Placement:** 2.3.2.
**Discovered:** 2026-11-08, during dhvani 2.3.1's dependency audit, from the compiler's duplicate-symbol warning.
**Severity:** High. Silent heap corruption on valid input, in any unit that includes goonj before yukti. dhvani is such a unit.
**Affects:** goonj 2.3.1 (tag `2.3.1`, `3de2e3c`, also `main`) with yukti 2.3.18 (`3efbba9`), built with cyrius 6.7.18. goonj 2.0.5 and yukti 2.3.11 carry the same two values.
**Companion filing:** yukti `docs/development/issues/2026-11-08-dh-size-collides-with-goonj.md`. One rename closes both.

## Summary

goonj's `DhvaniLayout` sets `DH_SIZE = 88`. yukti's `DeviceHealthField` sets `DH_SIZE = 24`. Cyrius bundles share one flat namespace, and a redefined enum member takes the last definition at every use, including uses compiled before it. The compiler only warns:

```
warning:lib/yukti.cyr:713:17: duplicate symbol 'DH_SIZE' redefined with conflicting value (last definition wins)
```

So with goonj included before yukti, `generate_dhvani_ir` runs `alloc(DH_SIZE)` as `alloc(24)`, then writes the eight rt60 values at offsets 24..87. Those 64 bytes are outside the allocation and overlap whatever the bump allocator hands out next.

## Where

- `src/dhvani.cyr:30`: `DH_SIZE = 88` in `enum DhvaniLayout`.
- `src/dhvani.cyr:77`: `var d = alloc(DH_SIZE);` in `generate_dhvani_ir`.
- yukti `src/core.cyr:207`: `DH_SIZE = 24` in `enum DeviceHealthField`.

## Measured

| Include order | Bytes reserved for the `DhvaniIr` (gap to the next `alloc`) | Unrelated write changes `rt60[7]` |
|---|---|---|
| dhvani 2.3.1 unit (all deps) | **24** | **yes** |
| goonj, then yukti | **24** | **yes** |
| goonj alone | 88 | no |
| yukti, then goonj | 88 | no |

In the two bad rows, writing 64 bytes to the allocation that follows the `DhvaniIr` changes what `dhvani_ir_rt60_band(d, 7)` returns.

## Repro

From a dhvani 2.3.1 checkout, which has both bundles in `lib/`:

```cyrius
include "lib/sakshi.cyr"
include "lib/hisab.cyr"
include "lib/goonj.cyr"
include "lib/freelist.cyr"
include "lib/process.cyr"
include "lib/fs.cyr"
include "lib/patra.cyr"
include "lib/yukti.cyr"

alloc_init();
var room = acoustic_room_shoebox(f64_from(5), f64_from(4), f64_from(3), acoustic_material_wood());
var cfg = ir_config_default();
var d = generate_dhvani_ir(hvec3_new(f64_from(2), 1.5, f64_from(2)),
                           hvec3_new(f64_from(3), 1.5, f64_from(2)), room, cfg);
var n = alloc(64);
println_int(n - d);                      # 24 here; 88 is correct
var before = dhvani_ir_rt60_band(d, 7);
var i = 0;
while (i < 8) { store64(n + i * 8, 0x4141414141414141); i = i + 1; }
var after = dhvani_ir_rt60_band(d, 7);
if (before != after) { println("rt60 corrupted by an unrelated allocation"); }
syscall(60, 0);
```

```sh
cyrius build --no-deps repro.cyr repro && ./repro
```

Delete the five lines from `lib/freelist.cyr` to `lib/yukti.cyr` and it prints 88, with no corruption.

## Reach

dhvani's manifest and include order put goonj before yukti, and `cyrius test` compiles every declared dep into every unit. `dhvani_acoustics_generate_ir` (`src/acoustics.cyr:22`) calls `generate_dhvani_ir` directly, and `tests/acoustics.tcyr` reaches this path and passes. dhvani itself names no `DH_*` member of this enum.

## Fix direction

Rename the members of `DhvaniLayout` here, for example to `DHVANI_IR_*`. Only `DH_SIZE` collides, but the whole family has 15 uses, all in `src/dhvani.cyr`, and a module-specific prefix prevents the next one.

Doing it in goonj is the shorter path. yukti also ships inside the cyrius stdlib snapshot, so a rename there reaches consumers only after a toolchain re-vendor.

## Regression

A test that includes yukti after goonj and asserts the gap from a `generate_dhvani_ir` result to the next `alloc` is at least 88 bytes.

## Resolution

Fixed in goonj 2.3.2 as the fix direction proposed. yukti needs no change.

- **The rename.** `DhvaniLayout`'s members are `DHVANI_IR_IR`, `DHVANI_IR_MULTIBAND`,
  `DHVANI_IR_VOLUME`, `DHVANI_IR_RT60` and `DHVANI_IR_SIZE`, with the same values. All 15
  uses are in `src/dhvani.cyr`, and nothing outside it named them.
- **A second collision of the same kind.** A scan of every top-level goonj name against
  the cyrius 6.7.18 stdlib found `LogLevel`'s `LOG_FATAL = 0` … `LOG_TRACE = 5` against
  stdlib `log`'s `LOG_TRACE = 0` … `LOG_FATAL = 5`, in the opposite order. With `log`
  after goonj, `logging_init_verbose()` set sakshi to 0, fatal-only, and `GOONJ_LOG`
  parsed backwards. They are `GOONJ_LOG_*` in `enum GoonjLogLevel` now. This one changes a
  public name. None of the six consumers names one.
- **One overlap stays.** `ERR_NONE` is also an enum member in shravan, which dhvani links.
  Both are 0, so nothing changes meaning. naad's suite reads goonj's `ERR_*` codes by name,
  so the family keeps its names.

**Regression.** `tests/namespace.tcyr` includes the bundle, then yukti's
`DeviceHealthField` and stdlib `log`'s `LogLevel` copied verbatim from the 6.7.18
snapshot. It checks that the next `alloc` starts at least 88 bytes past the `DhvaniIr`,
that writing that allocation leaves every rt60 band alone, and that the logging levels
reach sakshi as sakshi's numbers. Against 2.3.1's bundle, with 2.3.1's names, 5 of its
11 assertions fail: the gap is 24, rt60 is overwritten, and verbose init reads 0.

**The general gate.** `scripts/check-namespace.sh` fails if any top-level name in
`dist/goonj.cyr` is also defined in the pinned toolchain's stdlib. CI runs it after the
bundle-freshness check. On the 2.3.1 bundle it reports `DH_SIZE` (yukti), the six
`LOG_*` and `LogLevel` (log), and `LogLevel` (sakshi). On 2.3.2 it reports none. Pass a
consumer's `lib/` to check that consumer's bundles as well:
`scripts/check-namespace.sh ~/Repos/dhvani/lib` reports only `ERR_NONE` (shravan).
