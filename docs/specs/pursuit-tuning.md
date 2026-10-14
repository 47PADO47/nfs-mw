# Pursuit tuning: the Bartender file

The pursuit numbers the police read from `NFSMWBartenderSettings.ini`: how many cops chase, how fast heat grows, the speed
under which the busted timer fills, the escape's heat loss and the fines. The chase itself is in
[ai-pursuit.md](ai-pursuit.md) and [ai-pursuit-heat.md](ai-pursuit-heat.md). The reset penalty is in
[fairbusts-reset.md](fairbusts-reset.md). Provenance of the defaults: [../provenance/pursuit-tuning.md](../provenance/pursuit-tuning.md).

- **Mod:** NFSMWBartender (CC0). The mod's code is not used. Its key names and defaults were not read (see
  [../mods/nfsmwbartender.md](../mods/nfsmwbartender.md) section 4), so this file has its own keys and does not load a
  Bartender file.
- **Status:** the readers and the wiring below are built. The rest of the Bartender feature list is not, because the
  systems it tunes do not exist yet (section 4).

## 1. The file

File: `NFSMWBartenderSettings.ini`, next to the config file, read at the start of a track. Keys follow `[Section]` and
`Key = value`; `//` starts a comment.

| Section | Key | Default (vanilla) | Range | Used by |
|---|---|---|---|---|
| `[Cops]` | `MaxCops` | 8 | 1 to 64 | The most cop cars chasing at once (`Pursuit::wanted`). |
| `[Heat1]` to `[Heat10]` | `CopCap` | none: the install's `FullEngagementCopCount` of the level | 1 to 64 | The cops the wave wants at that level, before the cap of `MaxCops`. |
| same | `TimePerLevel` | 600 s (levels 1 to 7), 780, 820, 2000 | 1 to 100000 s | Seconds of heat growth per level (`Heat::grow`). |
| same | `BustSpeed` | 9, 12, 19, 20, 25, 25, 19, 5, 5, 5 km/h | 0 to 200 km/h | The speed under which the busted timer fills (`bust_speed`). |
| `[Heat]` | `EvadeAdjust` | 0.95 | 0 to 2 | The share of the heat a chase keeps after an escape (`carried_heat`). |
| `[Fines]` | `Speeding`, `Racing`, `RecklessDriving`, `Assault`, `HitAndRun`, `DamageToProperty`, `ResistingArrest`, `OffRoad` | 150, 350, 1000, 350, 300, 100, 300, 75 | 0 to 1000000 | The fine of each infraction, once per chase. |
| `[Busted]` | `Timeout` | 5 s | 0.1 to 60 s | Seconds of the busted timer past which the player is busted. |
| same | `Reach` | 15 m | 1 to 500 m | Horizontal reach of a height-matched cop while not flashing. |
| same | `HeightGate` | 1.5 m | 0.1 to 10 m | Maximum height difference for a cop to count. |

Rules: a missing file is the vanilla tuning. A value outside its range is clamped to the nearest bound. A malformed value
is its default. Each is logged, so the game still starts. A level without its section keeps the vanilla value for that
level. A `CopCap` of none uses the install's row, so the install's values still apply without a `CopCap` key.

The vanilla values are in `crates/nfsmw/src/settings/bartender/vanilla.rs`; a test checks that a file of them reads back
as the defaults.

## 2. What reads them

- `Police::set_settings` takes the Bartender file and the FairBusts file. The track's loader (`ai/data.rs`) calls it next to
  the helicopter's settings.
- The heat growth, the escape's heat loss and the fines are read at the moment they apply (`police.rs`).
- The busted timer's constants come from `Police::bust_tuning()` (`police/busted.rs`). The fill and drain rates
  (+1 and -2 per second) and the height gate's place in the rules are not in the file.
- The cop cap is read by `Pursuit::wanted` together with the install's wave. The helicopter's own tuning is in
  `NFSMWHelicopterSettings.ini` and is not part of this file.

## 3. Not in the file

The escape's required time (`evadetimeout`, per heat) and the hiding multiplier are install values in `blackbox_pursuit::evade`.
They are read by the HUD and the escape, and moving them into this file means threading the settings through the pursuit
feed (`TrafficWorld::pursuit_feed`) and the HUD, which was not done in this pass.

## 4. Not built

These are the Bartender feature list's items that have no system in the rewrite to tune yet. Each needs its system first.

- Bounty and combo multipliers, rep points (`RepPointsPerMinute`, `RepPointsForDestroying`): the economy is not built
  (`Police::record_wreck` is the hook).
- NOS and Speedbreaker charge gained from tagging, assaulting or destroying cops: no such gain exists yet.
- Roadblock and ramming strategy settings (`roadblockprobability`, `roadblockspikechance`, the `CopFormations` records):
  the support roll reads the install's values; overriding them needs a settings path into `support::heavy_options`.
- Bug-fix toggles: the Bartender mod's list of fixes is not known, so none are listed.
- Helicopter spawn and respawn rules beyond `NFSMWHelicopterSettings.ini`.
