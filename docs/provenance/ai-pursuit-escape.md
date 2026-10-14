# Provenance: the escape cool-down and the end-of-pursuit card

What the escape uses, where each value came from, and what is still a guess. The rules are in
`libs/blackbox-pursuit/src/cooldown.rs` (with the per-heat tables in `evade.rs` and the outcome in `end.rs`); the wiring
is in `crates/nfsmw/src/scenes/world/ai/police/escape.rs` and the card in `crates/nfsmw/src/hud/pursuit_end.rs`.
Spec: [ai-pursuit.md](../specs/ai-pursuit.md) sections 4.3, 6 and 7.

## Values

| Value | Used as | Source |
|---|---|---|
| Grace 7.0 s | `TUNING.grace` (lost after 7 s without a sighting) | decomp, `mIsPerpInSight` (spec 4.3) |
| Sight timer starts at -5 s | `SIGHT_AT_START` | decomp, `time_since_any_saw` start (spec 4.3) |
| Sighted value -0.25 s | `SIGHTED` (the cops' visibility test leaves this) | decomp (spec 4.1) |
| Mode gate 2.5 s | `TUNING.mode_gate` (each way between chase and cool-down display) | decomp, `PursuitMeterModeTimer` (spec 6) |
| Decay 0.93 per update | `TUNING.decay` (shown meter while a cop sees the player) | decomp (spec 6); the float 0.93 is in `speed.exe` (see Ghidra pass) |
| Floor 0.05 | `TUNING.floor` | decomp (spec 6) |
| Required seconds per heat | `evade::TIMEOUT` (20, 45, 75, 90, 120, 120, 120, 220, 60, 220) | install-verified `evadetimeout` (spec 6) |
| Hiding multiplier per heat | `evade::HIDDEN_MULTIPLIER` (4, 5, 6, 7, 8, 8, 8, 4, 2, 4) | install-verified `HiddenZoneTimeMultiplier` (spec 4.3) |
| EVADED at meter 1 while shown | `Status::Evaded`, terminal | decomp (spec 6) |

## Discrepancies and guesses

- The hiding latch (0.05 s, or effectively infinite while seen) is not implemented: `hidden` is the world's per-frame
  zone test, as it was before this change.
- The roadblock sight (60 m) in spec 4.3 is not wired: it needs the roadblock cops, which are not built.
- The spec says the cool-down entry also infracts "resisting arrest" and sets the heli search. The code still records
  resisting arrest when sight is lost (`rules::lost_sight`), not at the cool-down entry. Unchanged here; to be aligned
  with the heat spec.
- The decay runs once per world update (the frame), as the spec's "per pursuit update" reads. The spec's update
  rate is 4 Hz (section 1.1), which would make the per-second drain differ with frame rate. Not resolved.
- `busted` skips the escape on its frame and the world ends the chase busted first. The spec says a busted chase
  sets the meter to 0; the chase is over on that frame, so the effect is the same.
- The card's wording ("You escaped", "Busted", "Cops wrecked", "Time") is ours, not the game's. The original text
  is in the package the Ghidra pass did not locate (see below).

## Ghidra pass (`/speed.exe`, base 0x400000)

The pass was partial. The binary is stripped (names are `FUN_*`), so the search went through strings and constants.

| Identifier or value | Address | Referenced by | Note |
|---|---|---|---|
| `MPerpEscaped` | 0x00890cb0 | 0x00405c6a in `FUN_00405c40` | the evade message name; confirms the event exists |
| `EVADED_%d` | 0x0089d470 | 0x00546da6 in `FUN_00546a50`, 0x0052d875 in `FUN_0052d640` | a format string for the evaded state; the status logic is not decoded |
| float 0.93 (`7B 14 6E 3F`) | 0x00892fac | 0x0071b56c in `FUN_0071b4e0` (a read) | consistent with the decay constant; the function is not decoded, so this is a match, not a proof |
| `InGameBusted.fng` package | not found as a string | none | the EVADED/BUSTED overlay package was not located; the card is built from the feed, not the package |
| 2.5 s gate, 7.0 s grace | not searched | none | unverified in the binary |

Not done in this pass: `PursuitIsEvaded`, `ShouldEnd`, and the cool-down state machine's decode. The spec values above
still rest on the decompiled source the spec cites.
