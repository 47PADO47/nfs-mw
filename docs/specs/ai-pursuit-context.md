# Pursuit event context

Sources read: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw)
(CC0-1.0, decompiled), `AI/Common/AIPursuit.cpp` constructor and heat update,
`Gameplay/GRaceStatus.h` bin accessors, `AI/Common/AIVehicle.cpp` heat-table
selection, and generated `gameplay` field declarations.
Read for facts; no code copied. Complements [ai-pursuit-heat.md](ai-pursuit-heat.md).

## Context selection **[decomp]**

Outside a race, select `pursuitescalation/heattable`. While racing an event whose
`PursuitRace` is false, select `racetable`; pursuit events retain `heattable`.
Loading, intro and result phases also retain `heattable` until actual racing.
Keep the same heat index for wave composition, backup, growth and rep rates.
Selection must happen before pursuit construction, including event
starts which run while the world temporarily takes its race session out.

Career events use the current career bin's heat bounds, except the epic pursuit,
which fixes both bounds at six. Quick races other than challenges use one to five.
Other contexts use their event bin's `BaseOpenWorldHeat` and `MaxOpenWorldHeat`.
With no bin these default to zero and ten, respectively. `UseWorldHeat` replaces
these with `ForceHeatLevel` and `MaxRaceHeatLevel`. Otherwise `MaxHeatLevel` can
lower the maximum. The generated PC class lacks `MaxRaceHeatLevel`; absence must
preserve the bin maximum instead of inventing a special event mapping.

Growth uses the selected row's `TimePerHeatLevel`. A loaded career profile adds
the bucket multiplier: challenges use index fourteen, other contexts use the
current career bin capped at fourteen. Without a profile the base row time
applies. `ScaleOpenWorldHeat` is constructor state in the reference; the inspected
growth code does not consume it, so it is not applied speculatively.

The epic pursuit resets unseen time every update; ordinary cooldown cannot end
it. The authored win trigger or a bust decides the event. This does not create
false ground sightings or reveal a hidden target to cop drivers.

## Integration policy **[ours]**

Console heat starts remain explicit development overrides: they may raise the
career maximum to the requested level. Automatic patrol and event starts clamp
to their context bounds. Explicit mod heat ranges retain priority except the
authored final-pursuit rule. A forced console heat is not an event identifier.
Special rows eight through ten remain available without assigning guessed event
semantics. A row's wave size, required evasion budget and active cop cap remain
separate values.

Leaving an event restores the open-world table and bounds for a continuing
pursuit. Restarting reconstructs its context from the same event parameters.
New original-game recordings are not required to test these state transitions;
their visual and route-level parity remains unverified.

## Installed field types **[decomp]**

The generated `pursuitlevels` schema stores `CollapseInnerRadius` and
`MaxCopsCollapsing` as `Int32`. Other formation scalar fields and the level
tuning fields consumed here are `Float`. Readers must decode the stored type
before converting an integer radius to runtime metres; accepting a float or
falling back to a default would hide an invalid or missing field.
