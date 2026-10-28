# Police cars in mods

Part of the [modding catalogue](README.md). A mod makes a car a **police car** with `role = "cop"` (or `"both"`) in its car
manifest ([api-content.md](api-content.md#cars)) and says, level by level, how many cars of it the cops send and how likely
the chase is to pick it: the same two numbers the original gives its SUV. A mod cop car is a car of its own: nothing about it is
taken from a stock cop's record.

## How the original puts a cop car in a chase

(Sources: [pursuit-data.md](../formats/pursuit-data.md), [ai-pursuit-cops.md](../specs/ai-pursuit-cops.md),
[ai-pursuit-cop-cars.md](../specs/ai-pursuit-cop-cars.md), [ai-helicopter.md](../specs/ai-helicopter.md).)

- **The wave.** Each of the ten heat levels has a row (`pursuitlevels/heat_01..10`) with up to three records
  `{ cop type, Count, Chance }`: the 2005 game's heat 1 is `copmidsize x4`, heat 6 is `copsportghost x8 @80`, `copheli x1 @50`
  and `copsuvpatrol x2 @80`, heat 7 is `copsuvpatrol x6 @90`. **That is all a wave entry carries**; the SUV has no other
  per-level number. A chase picks the next cop at random, **weighted by `Chance`** (0 reads as 100), among the types whose
  `Count` the chase does not have yet; the nominal total is cut to the number of cops the chase still needs
  (`wave::adjusted_counts`), so a row with `Count` 6 asks for fewer when the chase is nearly won. A heat rise re-reads the row.
  So **`Count` is how many, `Chance` is the priority**.
- **The helicopter** is a record of the same row (`copheli x1 @50`), picked by the same weight, but it is spawned by its own
  rules: a timer (`TimeBetweenHeliActive`, `HeliFuelTime`, `SearchModeHeliSpawnChance` of the level's row), its own model
  without physics, and a flight goal. A mod cannot add a flying unit: a mod cop is a ground car.
- **Support units** are not wave rows: the heavy SUVs (`copsuv`, `copsuvl`) come from the `pursuitsupport` rows' heavy
  options (ram, roadblock; `ChanceBigSUV` picks the model) and the `copcross` car from the leader options. Their cars are fixed
  by the support code. A mod cop is a **wave** unit; it can be a heavy one by class (below) but does not replace the support
  cars.
- **Per car, not per level**, the original keeps in the car's `pvehicle` the records a cop needs: `aivehicle` (speed limits, the
  rep points for wrecking it), `damagespecs` (hit points), `collisionreactions` (how it hits the player), the siren and light
  groups of its `ecar`, and the chase radio's class is chosen by its name (`copsuv*` is a Rhino unit, `copsport*` a super
  pursuit unit).

## The manifest

```toml
role = "cop"                  # player, cop or both
cop_class = "heavy"           # patrol (default), pursuit or heavy: what kind of unit it is (below)

# Short form: one entry per level, the default numbers (2 cars, 10 percent).
heat = [2, 3]

# One table per entry, with its own numbers. heat is one level or a list of levels.
[[cop]]
heat = 6
count = 2                     # cars of this kind the wave wants: 1 to 8 (default 2)
chance = 80                   # the weight of the pick, percent: 1 to 100 (default 10)

[[cop]]
heat = [7, 8]
count = 6
chance = 90
```

- The two forms mix when they name different levels; a level named twice (in either form) refuses the car. Levels run from 1
  to 10. Unknown keys in a `[[cop]]` table refuse the car, and `heat` is required there.
- A cop car that names **no level** is in every level with the default numbers (as before).
- `heat`, `[[cop]]` and `cop_class` need `role = "cop"` or `"both"`.
- The host puts one record `{ car, count, chance }` in the wave of each level named, as the original's rows do
  (`car::mods::add_cop_rows`). The race waves (`racetable`) are not touched.

Code: `libs/blackbox-modding/src/content/cop.rs` (schema, checks, tests in `cop_tests.rs`),
`crates/nfsmw-data/src/car/mods/cop.rs` (what the game keeps).

## What makes it a police car, system by system

| System | What a mod cop car gets |
|---|---|
| Car table | usage type **1** (police), the type the stock cops have, for a car that is only a cop. A car with `role = "both"` stays usage 0: it has to be in the player's lists, which list racing cars, and it drives as a cop by its unit |
| Garage category | `Category::Police` for usage 1: the developer garage (F6, `garage` command) lists it under Police, like the stock cops; the player's lists (My Cars, the car lots) take racing cars only, so a cop-only car is not offered there |
| Cop waves | the records of its levels in `pursuit::heat_row` with its count and chance; patrols follow level 1's wave; every level's models are loaded when a chase starts |
| Physics and speed | built from its own `[stats]` and `BOUNDS.BIN` like a player car; the AI may use the car's whole top speed (`ai_vehicle_from_stats`) |
| Damage | a mod car's physics carry racing damage numbers with no hit points, which would make a cop that nothing can wreck, so a cop takes the numbers of its **class** (table below) |
| Rep for a wreck | `RepPointsForDestroying` of its class at every level (`pursuit::rep_table`) |
| Chase radio | the class: `heavy` is called as a Rhino (sport utility) unit, `pursuit` as a super pursuit unit, `patrol` as an ordinary car (`audio/chase/radio/tables.rs`, `vehicle_of`) |
| Cost to the state | a `heavy` car counts as a support car (450) like the SUVs; the others as patrol cars (250) (`ai/police/costs.rs`) |
| Light bar | the `[[light_bar]]` tables of its manifest: one siren lamp each, with a place, colour, sprite and a flash pattern, flashing while its emergency lights are on ([below](#the-light-bar)); a car without them is lit by its `COPLIGHT*` markers ([car-lamps.md](car-lamps.md)) with the shared red and blue pulses; the particle light groups of a stock cop are keyed by its `ecar` record, which a mod car does not have, so it gets none |
| Sirens, speech | the siren sounds and the dispatch speech are shared, not per car |

### Class defaults

The numbers are the generic ones the game gives a police car that has no record of its own, chosen by class. They are close
to the 2005 game's own range (its cop cars have 5 to 10 hit points), but they are the class's, not any car's.

| Class | Hit points | Threshold | Rep for a wreck | Radio |
|---|---|---|---|---|
| `patrol` | 5 | 0.75 | 250 | ordinary patrol car |
| `pursuit` | 8 | 1.5 | 5,000 | super pursuit unit |
| `heavy` | 10 | 2.0 | 25,000 | Rhino (sport utility) unit |

The camera distance beyond which hits are ignored is 35 m and the front and rear zones scale hits by 0.5, for all three.
The collision with the player uses the game's default reaction, not the cop sets (`collisionreactions` of the original).

## The light bar

The original flashes a cop's lights from two things: the `COPLIGHT*` markers of its model, which get the game's red and blue
flare profiles pulsed by one shared timing (`car_light_flares/emergency.rs`), and the particle groups of its `ecar` record. Both
run while the cop's emergency lights are on (`AiCar::emergency_lights`: a live cop or roadblock car that is not arrested and not
fleeing). A mod cop car can give its own bar with `[[light_bar]]` tables, one per siren lamp (the checks are in
`libs/blackbox-modding/src/content/lamp_flares.rs`):

```toml
[[light_bar]]
siren = "blue"                   # red, blue or white: which emergency light switches and breaks the lamp
pattern = "000090000900009000"   # one digit 0 to 9 per step: the brightness in ninths, played in a loop
step_seconds = 0.125             # how long a step lasts (0.005 to 2)
phase_seconds = 0.0              # where in the loop the lamp starts (default 0)
sprite = "MYCAR_CORONA0"         # a texture of the car's own TEXTURES.BIN, drawn additively
position = [-0.3, 0.45, 1.4]     # the car's space, metres
direction = [0.0, 0.0, 1.0]      # not used: a siren lamp is seen from every side (the key is required, as for a flare)
colour = [0.2, 0.55, 1.0]        # the tint, 0 to 1
alpha = [0.5, 1.0]               # the brightness at the near and the far distance of `range`
radius = [0.2, 0.9]              # metres at the near and the far distance
range = [8.0, 100.0]             # metres
falloff = 0.0
```

The look keys are those of [`[[lamp_flare]]`](car-lamps.md#lamp-flares-and-glows). The lamp's brightness is the pattern's step at
the car's lamp clock (`LampState::time`, the same clock the marker flares use, offset by the car's own phase) times the look; it is
dark when the emergency lights are off or the lamp's colour bit is broken (a blue lamp is broken with the blue bit, white with
white). Up to 32 siren lamps a car. A car that gives any `[[light_bar]]` is not drawn with the markers' siren flares
(`COPLIGHTRED`, `COPLIGHTBLUE`, `COPLIGHTWHITE`, the bright and the orange ones): the bar is the car's whole siren. A player-driven
mod car does not flash: only the cops the game drives have emergency lights.

**The 2012 cars.** The five police vehicles of the [2012 mod](../../mods/nfsmw-2012-content/README.md#police-cars) get a bar from
their `<Car>_Cop_Coronas` records: 8 lamps on the Charger, the Corvette and the SWAT van, 19 on the Crown Victoria, 20 on the
Explorer. Each lamp's place is its locator group's, its colour the glow of its corona object (blue or red), and its pattern the
record's flash pattern baked at its shortest step: three long-pattern lamps of each colour flash in triples that run along the bar
(0.125 s steps over five 16-second cycles) and two small lamps strobe in alternation (0.034 s steps); the data is in
[chameleon-corona.md](../formats/chameleon-corona.md#flash-patterns-verified).

## The developer tools

- `garage` (world and car viewers) lists every car folder by category: racing cars, police cars, **mod player cars**, **mod police
  cars (cop only)** with their class and levels, and mod cars that are both:
  `mod police cars (cop only): V12_SWAT [heavy, heat 8 x1 @30%, 9 x2 @40%]`.
- `pursuit cops` prints the wave of every heat level (car, count, weight; the mods' cars marked `(mod, <class>)`) and each mod
  cop car with its entries. `pursuit cop <car>` asks for any car of that list (a mod's too), and `pursuit <heat>` starts a chase
  with the level's wave.
- The F6 garage lists a cop-only mod car under Police.
- Code: `crates/nfsmw/src/devtools/car_list.rs`, `scenes/world/ai/commands.rs`.

## What is not done

- **A flying unit** (a mod helicopter) and a **replacement for the support cars** (the heavy SUVs, the cross car).
- **Per-car light bar particle groups.** A mod cop has the sprites of its `[[light_bar]]` tables (or the flares of its `COPLIGHT*`
  markers), not the particle groups of a stock cop's `ecar` record. The 2012 bars are an approximation of the 2012 look and were not
  seen in a window ([the light bar](#the-light-bar)).
- **Collision reactions** (pit and roadblock weights) are the default ones.
- **The race waves**: a mod cop is not in `racetable` rows.
- Nothing was driven or listened to: the checks are unit tests of the manifest, the wave rows and the class tables, and the
  game's parser accepting the 2012 mod's manifests.
