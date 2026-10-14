# Police helicopter tuning file and hidden-zone types (epic E9, tuning part)

- **Restricted source read:** [docs/mods/nfsmwhelicopteroptions.md](../mods/nfsmwhelicopteroptions.md), a behaviour summary of
  the NFSMWHelicopterOptions mod (MIT). Used for the file's key names (`[Helicopter:<Name>]`, `default`, `heatNN`,
  `raceNN`) and the list of tunables. **Its numbers are not used as defaults.** Its FuelTime defaults for heat 7 to 10
  are 180, 180, 180, 180 s, while the install-verified table in [specs/ai-helicopter.md](../specs/ai-helicopter.md)
  section 2.3 gives 10, 75, 120, 400 s. Heat 4 to 6 agree (60, 90, 180 s). The install-verified table is the default.
- **Defaults and rules:** [specs/ai-helicopter.md](../specs/ai-helicopter.md) section 2.3 (`pursuitlevels` per heat and race
  level, tagged [verified] there), section 3 (the hidden-zone rule), section 8 (constants). Also the format record of HIDDEN
  zones in [formats/pursuit-data.md](../formats/pursuit-data.md) section 1, which states that every installed HIDDEN zone has
  `Data[0] = 0` (so the install's zones hide from both).
- **Mod numbers not used:** the mod's FuelTime row (10, 10, 10, 60, 90, 180, 180, 180, 180, 180) and its FlySpeed, SearchSpeed,
  Leading, ClosingSpeed, SearchRadius, Turning, Acceleration and Brake defaults were not taken as game truth. Where the spec
  has a value (fly and search speed, attack duration, visual sphere), the spec's is the default.
- **Ghidra:** not run in this pass. The `ghidra-mcp` tools were deferred and not loaded, so `CanSeeTarget`, the per-level
  `heliLOSdistance` and `HeliFuelTime` reads, `MAX_SPEED_MPS` and the hidden-zone data words were **not** re-checked in
  `speed.exe`. The values are the spec's, as tagged there. The install reader `nfsmw-data` has no field reader for
  `pursuitlevels` helicopter fields yet, so there is no install-gated test of them either.
- **Clean-room statement:** the file's schema, reader and clamping are our own code (`crates/nfsmw/src/settings/helicopter.rs`,
  built on `libs/ini-tuning`). No code of the mod was read or copied.
- **Hidden-zone reversal:** the perpetrator's test, as recorded in [formats/pursuit-data.md](../formats/pursuit-data.md): a
  HIDDEN zone whose first data word is the name hash of `"Car"` hides the target from cars only; `"Heli"` from helicopters
  only; any other word from both. Implemented in `crates/nfsmw/src/scenes/world/ai/police/world.rs`
  (`hidden_from_cars`, `hidden_from_helicopters`).
- **Tunnel test:** the spec's `CanSeeTarget` checks the main game view for a tunnel. We test the player's place against the
  TUNNEL zones (kind 3). The camera is not in the police state, so this is a proxy and is noted as such in the code.
