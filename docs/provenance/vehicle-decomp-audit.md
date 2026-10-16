# Provenance: vehicle decomp audit

Companion of [../specs/vehicle-decomp-audit.md](../specs/vehicle-decomp-audit.md).

## Sources

- PC decompilation, Most-Wanted-Vehicles-Decomp (Brawltendo, no licence, EA code). Read through the raw GitHub
  URLs under `Speed/Indep/Src/Physics/` via summary fetches:
  - `Behaviors/SuspensionRacer.cpp` (tire model, suspension, steering, drift, wheel forces). Three fetches.
  - `Behaviors/Chassis.cpp` (aero, grip scales, Ackermann). Fetched; the summary did not cover wheel radius,
    contact geometry, the step rate or puncture logic.
  - `Behaviors/EngineRacer.cpp` (torque lookups, engine braking, speed limiter, launch window). One fetch.
  - `PhysicsTunings.h` (empty tuning struct only).
  - Directory listings of `Speed/Indep/Src/Physics/Behaviors` and `Speed/Indep/Src/Physics`.
  - Verbatim requests were refused by the fetch tool; no source text is copied into the repo.
- `speed.exe` (PC v1.3) in Ghidra at `/speed.exe`. Stripped (no function names). Image base 0x00400000, 26083
  functions. Used for string search and byte search.
- Existing vehicle specs and provenance in the repo: `vehicle-calibration.md` (PC capture), the suspension,
  engine, steering, brakes and rigid-body specs, and their provenance notes (Q1..Q8).
- Port code read (read only): `libs/blackbox-vehicle/src/{tires,suspension,steering,ground,engine,drivetrain,
  brakes,induction,nos,aero,input,vehicle}`, `libs/blackbox-vehicle/src/lib.rs`, `crates/nfsmw/src/scenes/world/
  drive/sim/mod.rs`, `crates/nfsmw/src/scenes/world/events.rs`, `crates/nfsmw-data/src/car/damage.rs`,
  `crates/nfsmw-data/src/car/physics/chassis.rs`, `crates/nfsmw/src/scenes/world/drive/sim/ai.rs`.
- No copy of the decompiled source is in the repo. No local clone of the decomp repository exists under
  `C:\Users\PADO\github` (the folder `nfsmw-decomp` is a Ghidra scripts skeleton, not the decomp).

## Method

1. Listed the port's vehicle source files and their line counts. Read the tire, suspension, steering, brake,
   aero, nos, engine, drivetrain, input and vehicle modules.
2. Asked the decomp summary fetch targeted questions for each area (formulas, constants, units), in prose.
3. Compared each port constant with the decomp answer, with the existing specs and with the PC capture where
   one exists.
4. Used Ghidra byte search on float tables and on candidate constants, then looked for xrefs and nearby code.
5. Wrote the spec with a tag on every number.

## Lessons

- Summary fetches paraphrase and sometimes refuse verbatim text. Ask for numbers and units in prose, one area
  per question. Ask for the formula, not the code.
- Two fetches disagreed on the same steering table; the existing steering provenance agreed with the newer read.
  Settle a disputed table in bytes before writing a fix.
- The binary is stripped. Data tables found by byte search have no xrefs, even when the decomp names them.
  Immediate-address operand search (`search_instructions`) also found nothing for the two range/rate table
  addresses. Do not treat "no xref" as "unused".
- Float bytes of a six-point table can be searched as a consecutive run of little-endian floats. The range
  table at 0x008f8ae4 and the rate table at 0x008f8b64 both matched, confirming the two tables are distinct
  in the binary.
- Common constants (0.05 as `CDCC4C3D`) return dozens of hits. Byte search only works for multi-float runs.
- Ghidra tool names drift: `search_byte_patterns` loaded only after the analysis group was loaded and a
  ToolSearch for the exact name. Call `search_tools` first if a name is refused.
- The shipped data (`chassis.rs:35`) is the authority for game values. Library defaults can differ (the
  airborne downforce default 0.1 versus shipped 0.8). Check the data crate before reporting an aero mismatch.

## Dead ends

- Verbatim fetch of `SuspensionRacer.cpp`, `Chassis.cpp` and `EngineRacer.cpp` refused. Summaries used
  instead. Not a lead to retry.
- The raw GitHub `Speed/` paths under the decomp root returned 404 at first; the folder is `Speed/Indep/Src`.
- GitHub contents API returned the directory listings for `Physics` and `Behaviors`, but the repository tree
  endpoint was too large to read in one fetch.
- Ghidra function names: `find_functions` with name patterns "Steer" and "Brak" returned nothing (stripped).
- Ghidra xrefs to the puncture event strings `ETirePunctured` (0x008a7678) and `ETireBlown` (0x008a7664):
  none. The event names are probably hashed into the VLT table, not referenced by address.
- Ghidra byte search for the pad dead zone 0.05 (`CDCC4C3D`): 60+ hits, not useful alone.
- `search_instructions` for the operand `8f8ae4` and `8f8b64`: no code references. The table addresses are
  likely reached through a struct or a pointer table, not an immediate.
- Ghidra `search_byte_patterns` for the range table (1, 1, 1.1, 1.2, 1.25, 1.35): one hit at 0x008f8ae4. For
  the rate table (1, 1.05, 1.1, 1.2, 1.3, 1.4): one hit at 0x008f8b64.

## Results from the bytes (2026-10-16)

- Range-table bytes present at 0x008f8ae4: [1, 1, 1.1, 1.2, 1.25, 1.35] as six consecutive floats. [bytes]
- Rate-table bytes present at 0x008f8b64: [1, 1.05, 1.1, 1.2, 1.3, 1.4] as six consecutive floats. [bytes]
- Both were printed in a 192-byte memory read at 0x008f8ac0. The block also holds the input-speed table
  [1, 1.05, 1.1, 1.5, 2.2, 3.1] just before 0x008f8b64. [bytes]
- Use sites of these tables are not yet established. The spec marks section 2 as blocked on that.

## Next leads

1. Find the functions that read the table at 0x008f8ae4 (range) and 0x008f8b64 (rate). Options: search
   instructions for memory operands with base registers near 0x8f8a/0x8f8b; find the steering module by its
   constants (the 180 deg rate, the 45 deg maximum) as float immediates; then follow the reads.
2. Pad dead zone: search for a float 0.05 read next to the snap comparisons. Restrict to functions that also
   touch a gear field or the handbrake.
3. Tire puncture on impact: find the collision callback that writes tire state, then check whether a severity
   threshold exists in the original. Start from the damage zone code, not the event strings.
4. First-gear launch window: read the EngineRacer window routine in bytes and tie its output to the converter.
5. Wheel radius and body dimension: read the body position update call site and the tire diameter routine.
6. Brake scales (4 and 10): search for float 4.0 and 10.0 near the brake torque code; check against the spec
   Q8 note about stopping distances.
7. Confirm the ETirePunctured event by hash: compute the VLT hash of the event name and search for it as a
   32-bit immediate in code.
