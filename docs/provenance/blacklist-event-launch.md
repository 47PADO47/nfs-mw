# Blacklist event launch evidence

Installed PC gameplay data links each bin's world and boss races by collection
key. Several reused routes have stored identifiers with `.r`, while the first
three numeric components of their collection names also name earlier events.
The host discarded the suffix when converting bins, then launched the earlier
race. This was reproduced by comparing all bin references with the host's
converted event identifiers. Race settings and character preset references come
from those installed collections. Preset assembly and performance use the
existing car readers. The CC0 reference's generated `presetride` field table
identifies ordinary levels for tires, brakes, chassis, transmission, engine,
induction and nitrous; installed records supply their values. No game data or
translated implementation is embedded.

The PC `Physics::Upgrades::ApplyPreset` path was inspected after the installed
SLR preset requested tire level 3 on a car whose available maximum is 1.
It takes the minimum of the preset's category level and the vehicle's maximum
before applying the package. Program identity matched the existing PC research
image. This extends the strict public package-selection API with a bounded
conversion for installed presets; it does not weaken user-input validation.

The native front-end preset constructor selects the `pvehicle` by the preset's
vehicle key, separately from its model folder. Installed story-start and rival
BMW presets share the E46 model but reference different vehicle records. The
host now retains that distinction for physical setup and player engine audio.

The follow-up race-events screenshot exposed a separate presentation error:
the page appended the bin's boss events. The CC0 reference's
`UISafehouseRaceSheet::Setup` (`uiRepSheetRaceEvents.cpp`) enumerates only
`GetWorldRaceCount` / `GetWorldRaceHash` in both current-bin and older-bin modes.
This corrects the previous spec's free-plus-boss interpretation. Event identity
and boss launch remain unchanged; only the ordinary race page's membership changes.
