# FairBusts: the reset penalty of the busted timer

What happens to the busted timer when the player resets the car during a chase, what the stock game does, and the three
settings that change it. The busted timer itself is in [ai-pursuit.md](ai-pursuit.md) section 5; the values and the
binary reading are in [../provenance/fairbusts-reset.md](../provenance/fairbusts-reset.md).

- **Mod:** NFSMWFairBusts (MIT). Its key names are used here; its code is not, and its defaults are not all ours (see
  section 4).
- **Status:** built. The player's reset key starts the blink; the penalty is read from `NFSMWFairBustsSettings.ini`.

## 1. The stock behaviour

A manual reset (the reset key, `input.reset`) starts the post-reset blink. The blink lasts 2 s, the invulnerability timer
that a reset sets (`vehicle-rigid-body.md` section 7). While the player flashes, the busted timer's rules change:

| Rule | Normal | Flashing (stock) |
|---|---|---|
| Reach of a height-matched cop | 15 m | 90 m |
| Speed gate (the player must be slower than the heat's `BustSpeed`) | applies | skipped: a flashing player counts as slow |
| Fill rate | +1 per second | x4 (+4 per second) |

The effect is that a reset near a cop busts the player at once, whatever the speed. The mod calls this the instant bust.

## 2. The penalty

The busted timer takes a `ResetPenalty` of three values while the player flashes:

- `reach`: the flashing reach in metres.
- `speed_gate`: whether the speed gate still applies while flashing.
- `multiplier`: the flashing fill rate.

`ResetPenalty::STOCK` is 90 m, no gate, x4 (the install's values, see the provenance record). Normal reach is the
Bartender file's `Busted.Reach` (15 m vanilla).

## 3. The settings

File: `NFSMWFairBustsSettings.ini`, next to the config file. Section `[General]`. Read at the start of a track.

| Key | Default | Range | Meaning |
|---|---|---|---|
| `Enabled` | `1` | switch | `0` gives the stock penalty, whatever the other keys say. |
| `ResetAutoBust` | `0` | switch | `0`: the reset uses the normal reach (15 m) and the speed gate, so a reset does not bust by itself. `1`: the stock reach (90 m) and no gate, so the instant bust returns. |
| `ResetBustMultiplier` | `4.0` | 0 to 64 | The flashing fill rate. |

A missing file is the default. A malformed switch or number is its default, and a number outside its range is clamped.
Each is logged.

The mapping (`FairBustsSettings::penalty`):

| Settings | Reach | Gate | Multiplier |
|---|---|---|---|
| `Enabled = 0` | 90 m | no | 4.0 (stock, ignores the others) |
| `Enabled = 1`, `ResetAutoBust = 0` | normal (15 m) | yes | `ResetBustMultiplier` |
| `Enabled = 1`, `ResetAutoBust = 1` | 90 m | no | `ResetBustMultiplier` |

## 4. Differences from the mod

- **Default multiplier.** The mod's INI sets `ResetBustMultiplier = 1.0`, which its README describes as normal rules. The
  brief for this rewrite sets 4.0. At 4.0 the fair reset still fills four times as fast, but only within the normal reach
  and with the speed gate, so a reset does not bust a player who is moving away from cops.
- **Blink length.** The mod does not say how long a reset keeps the stock rules. This rewrite uses 2 s
  (`docs/specs/vehicle-rigid-body.md` section 7).
- **Compatibility.** The mod also patches other mods' code paths in the binary; this rewrite has no loader, so only the
  rules above apply.

## 5. Tests

`crates/nfsmw/src/settings/fairbusts_tests.rs` covers the defaults, the stock mapping, `ResetAutoBust`, the multiplier
clamp, malformed values and switches. `crates/nfsmw/src/scenes/world/ai/police_tests.rs` covers the penalty through the
busted timer: a fair reset does not bust a fast player 60 m away, the stock reset does, a fair reset still busts a slow
player close by, and a multiplier of 1 fills at the normal rate.
