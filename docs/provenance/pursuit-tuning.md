# Provenance: the pursuit tuning file and its defaults

Where each default of `NFSMWBartenderSettings.ini` came from, and what was not read. The schema and the rules are in
[pursuit-tuning.md](../specs/pursuit-tuning.md); the code is `crates/nfsmw/src/settings/bartender/`.

## Sources

| Default | Value | Source |
|---|---|---|
| `TimePerLevel` | 600 (levels 1 to 7), 780, 820, 2000 s | `docs/specs/ai-pursuit-heat.md` section 2.3 (the bin-independent values the rewrite already used) |
| `BustSpeed` | 9, 12, 19, 20, 25, 25, 19, 5, 5, 5 km/h | `docs/specs/ai-pursuit-heat.md` section 2.2 (`pursuitlevels.BustSpeed`, install-verified) |
| `Fines` | 150, 350, 1000, 350, 300, 100, 300, 75 | `docs/specs/ai-pursuit-heat.md` section 4 (`infractions`, install-verified; the constant the rewrite already used) |
| `EvadeAdjust` | 0.95 | `docs/specs/ai-pursuit-heat.md` section 2.3 (`EvadeSuccessHeatAdjust` of the default collection) |
| `CopCap` | none (the install's `FullEngagementCopCount`) | `docs/specs/ai-pursuit-heat.md` section 2.1 (install row, read by `nfsmw-data::pursuit`) |
| `MaxCops` | 8 | the rewrite's existing cap (`pursuit.rs`); the stock car loader's limit in the Bartender notes (`docs/mods/nfsmwbartender.md` section 4), not measured here |
| `Busted.Timeout` | 5 s | `docs/specs/ai-pursuit.md` section 5.1 (`kBustedTimeout`) |
| `Busted.Reach` | 15 m | `docs/specs/ai-pursuit.md` section 5.1 (`kBustedCopDistance`); confirmed by Ghidra in `docs/provenance/fairbusts-reset.md` (the 15.0 float) |
| `Busted.HeightGate` | 1.5 m | `docs/specs/ai-pursuit.md` section 5.1 |

## Not read

- The Bartender mod's own INI keys and defaults. Its README and code were summarised, not read verbatim
  (`docs/mods/nfsmwbartender.md`), so this file uses its own keys and does not claim to match the mod.
- A Ghidra pass over the AttribSys tables. The per-heat values above are the install-verified ones in the specs; they were
  not re-read from `attributes.bin` in this pass.
