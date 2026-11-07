# Blacklist event launch identity

## Event selection

The Race Events page lists only `WorldRaces`. Keep `BossRaces` exclusively in the
Challenge Rival flow. A boss race can use an ordinary mode icon and route name;
those do not make it a normal race. In particular, a zero-reward boss speed trap
must not appear after the final normal event in the last Blacklist bin.

Blacklist bins reference race collections by key. Convert each referenced race
with its stored event identifier, falling back to its collection name only when
that identifier is absent. Preserve suffixes such as `.r`: a reused route in a
later bin can have different opponents, rewards and story activity from its
earlier event. Never shorten that identifier to three numeric components.

The race sheet, forced player car lookup and world queue must resolve the same
event. Career launches use that collection's laps, difficulty, catch-up, traffic,
police flags, opponents and story settings, without custom-race overrides.
Ordinary events retain the player's career car. A forced player preset uses its
own car folder, appearance and installed performance. Rival presets retain their
appearance and equipment rather than resolving to a stock model of the same type.
Clamp preset levels to each vehicle's available category maximum, as the native
preset application does. A high preset level is valid even on a car with fewer
available upgrades. These records do not grant Junkman parts.
Use the preset's vehicle collection key for physics and sound. Its geometry
folder can be different: a story-start preset can share a body with another
car while naming its own vehicle record. Keep model and vehicle identities
separate when assembling and driving that preset.

## Validation

For every installed bin, follow both world and boss race keys through conversion
and lookup and require the original collection key. Cover reused routes whose
numeric stem also names a rival duel. Verify ordinary versus boss settings and
forced versus owned player cars. Exercise preset loading and equipment through
the world loader. Synthetic tests cover suffix identity without game data.

See [provenance](../provenance/blacklist-event-launch.md).
