# Provenance: the busted meter

What the busted timer uses, where each value came from, and what is still a guess. The rules are in
`libs/blackbox-pursuit/src/busted.rs`; the wiring is in `crates/nfsmw/src/scenes/world/ai/police/busted.rs`.
Spec: [ai-pursuit.md](../specs/ai-pursuit.md) sections 4.4 and 5.

## Values

| Value | Used as | Source |
|---|---|---|
| Timeout 5.0 s | `BUST_TUNING.timeout` (busted when the timer passes it) | decomp, `kBustedTimeout` (spec 5.2) |
| Cop reach 15 m, horizontal | `BUST_TUNING.reach` | decomp, `kBustedCopDistance` (spec 5.1) |
| Flashing reach 90 m | `BUST_TUNING.flashing_reach` | decomp (spec 5.1) |
| Height gate 1.5 m | `BUST_TUNING.height_gate` | decomp (spec 5.1: cop height differs by less than 1.5 m) |
| Fill +1.0 per second | `BUST_TUNING.fill_rate` | decomp, spec 5.1 net effect |
| Drain -2.0 per second | `BUST_TUNING.drain_rate` | decomp, spec 5.1 net effect |
| Flashing fill x4 | `BUST_TUNING.flashing_multiplier` | decomp, spec 5.1 and table (x4) |
| Bust speed 9, 12, 19, 20, 25, 25, 19, 5, 5, 5 km/h (heat 1 to 10) | `BUST_SPEED_KPH`, converted to m/s | decomp, `BustSpeed` (spec 5.1) |

## Discrepancies and guesses

- The task brief said the flashing multiplier is x6. The spec, from the decomp, says x4 (also in its constants
  table). The code follows the spec. If a later reversing pass finds x6, change `flashing_multiplier`.
- Spec 5.1 writes the per-tick update as `+0.25 * dt` (x4 flashing) and then says the net effect is +1.0 per second
  filling and -2.0 per second draining. The code uses the net rates. The exact tick scale is not confirmed.
- The meter shown to the HUD is `timer / timeout` (0 to 1). The decomp's `PursuitMeter` (spec 4.4, -1 to 1) is a
  different value that drives the five-segment bar. It is not implemented, and the bar object is not located
  (see [hud-pursuit.md](../specs/hud-pursuit.md)).
- `flashing` is always false for now. The post-reset blink is the FairBusts (E12) area.
- Whether a cop whose height matches counts for the whole 1.5 m gate, or only at the cop's own position, is taken as
  the spec states it.

## Ghidra pass (`/speed.exe`, base 0x400000)

The binary is stripped (names are `FUN_*`), so the search went through string references. Findings:

| Identifier | String address | Referenced by | Note |
|---|---|---|---|
| `Hud_BustedMeter` | 0x008a2878 | 0x008738f0 (in a function labelled `Unwind@008738ee`) | HUD element name; the reference looks like a name table, not the meter update |
| `PreventPlayerBeingBusted` | 0x008a4994 | 0x0061f0d1 (in `FUN_0061e750`) | a flag name; not yet read |
| `MPerpBusted` | 0x00890d98 | 0x004080fa (in `FUN_004080d0`) | the message is hashed at a static getter; the sender is not located |
| `eNIS_Busted`, `BUSTED_OVERLAY.fng`, `PostBusted.fng` | 0x008a3e20, 0x008948f8, 0x0089f7dc | not traced | the arrest and overlay side (E8f) |

No function was found that updates the busted timer, so the constants above are not confirmed by Ghidra. They come
from the decomp reading already in the spec. The name search returns nothing (`find_functions` with `busted` finds no
named function). The next step is to find the per-tick pursuit update that compares against a 5.0 float and to read
the 15.0 and 90.0 float constants from it.
