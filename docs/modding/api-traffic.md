# Mod API catalogue: traffic cars

Part of the [modding catalogue](README.md). Conventions are in [api.md](api.md#how-to-read-an-entry). **Works now** in NFS:MW: a
mod adds cars to the roads with a car manifest, and `bb.traffic` lists them and asks for one at a point.

A **traffic car** is a mod car that nobody drives: the game's traffic manager puts it on the roads among its own cars. It is a car of
its own like any [mod car](api-content.md#cars) (its geometry, textures, collision bounds and `[stats]`), with one more thing: how
it joins the traffic. The 2012 content mod's eleven traffic cars are the first users ([the section below](#the-2012-mods-traffic-cars)).

## The manifest

```toml
# cars/hatchback_tr.toml
role = "traffic"                 # player, cop, both or traffic; a [traffic] table next to another role makes that car traffic too
name = "Hatchback (traffic)"
[stats]                          # as for any car (car-stats.md); the AI drives the car with them
mass_kg = 1300.0
[customization]                  # one paint is the colour the car wears on the road (the first, or default_paint)
paints = [{ name = "Silver", color = [196, 199, 203], finish = "metallic" }]
[traffic]                        # every key optional; the class gives the defaults
class = "compact"                # compact, sedan, coupe, suv, pickup, van, truck, bus or taxi (default sedan)
weight = 10                      # 1 to 100 percent: how often it is picked, and the share of the slots it may take
max_instances = 0                # 0 to 16: at most this many on the road at once (0: no limit)
speed_factor = 1.0               # 0.5 to 1.5: times the pattern's street and highway speeds
patterns = ["downtown"]          # the districts it appears in, lower case; left out or empty: every pattern
```

| Key | Default by class | Meaning |
|---|---|---|
| `class` | `sedan` | The kind of road car. Sets the defaults below, the sounds the car uses and the devtools' label |
| `weight` | compact, sedan 20; suv, pickup 15; coupe, van 12; taxi 10; truck 6; bus 5 | The spawn weight in percent. The game's `traftaxi` is 20 in its default pattern |
| `max_instances` | truck and bus 1, others 0 | The most cars of the kind on the road at once |
| `speed_factor` | coupe 1.05; pickup 0.97; van 0.95; truck 0.85; bus 0.8; others 1.0 | The car cruises at this much of the pattern's speeds (35 mph on streets, 55 mph where there are four or more traffic lanes) |
| `patterns` | every pattern | The traffic patterns the car appears in. The game's are `default`, `downtown`, `collegesouth`, `collegenorth`, `collegehighway`, `cityhighway`, `coastalsouth`, `coastalnorth`, `industrial` and `drag`; a name the game does not have is never selected |

`role = "traffic"` is traffic only: no heat, `[[cop]]` or `cop_class` key (`HeatWithoutCop`). A key that is not listed, a number outside
its range, a class that is not one of the nine, or a pattern name that is not letters, digits and `_` (1 to 32 bytes, once) refuses the
car with the key named in the log. The pure checks are `libs/blackbox-modding/src/content/traffic.rs` (`TrafficClass`, `Traffic`,
`CarManifest::traffic`, `Role::Traffic`); the tests are next to it.

## What the game does with it

* **The car table.** A car that is traffic only is a traffic type (usage 2, the type the game's own traffic cars have), so it is not in the
  garage or any player list, and its models draw with the traffic thresholds (levels change at 80, 40, 20 and 10 pixels) and the
  traffic brake lamps. A traffic car that is also a player or cop car keeps that type (`car::mods::car_type_of`).
* **The patterns.** A traffic car gets one rule in the list of each pattern it appears in, **after** the game's own records of that
  pattern. A rule is the game's record (`rate`, `max_instances`, `percent`, [ai-traffic-spawning.md](../specs/ai-traffic-spawning.md#4-the-trafficpattern-data-verified)):
  `rate` is 120 divided by the weight, between 2 and 60 (a weight of 20 is rate 6, the game's taxi), `percent` is the weight, and
  `max_instances` is the manifest's. The manager looked at ten records per pattern; it now looks at those and up to 64 more.
* **The settings still rule.** The rules are the manager's, so the traffic density (`traffic` setting and the context density), the
  population budget (20, 40 or 64 slots), the common-slot filling and replenishment, the cop share and the pursuit scale act on
  mod cars exactly as on the game's own: a mod car never adds to the budget, it takes slots from the same pool, and a density of 0
  stops it. A weight relative to the others is a share of the picks: with the game's four default-pattern rules (rate 3, 3, 6, 10)
  and eleven mod cars of weight 10 (rate 12), about half of the new cars are mod cars.
* **Models.** The manager loads the model of every rule of the pattern in force when it starts the pattern, on the loader thread, a
  few a frame. A mod that adds cars while the game runs (`bb.content` `rescan`) makes the manager read the rules again at its next
  tick; the pattern restarts and the new cars load.
* **Speed.** A car is spawned with the pattern's cruise speeds times its `speed_factor`.
* **Physics, damage and collision.** From the car's `[stats]` and `BOUNDS.BIN` like any mod car; a hit shocks it as it shocks the game's
  traffic.
* **Sound.** A mod car has no `pvehicle` record, so the car's class names the game's traffic car whose horn, engine bank and drive-by
  sound it uses: compact `trafha`, sedan `traf4dseda`, coupe `trafficcoup`, suv `trafstwag`, pickup `trafpickupa`, van `trafminivan`,
  truck and bus `trafdmptr`, taxi `traftaxi` (`TrafficClass::sound_record`). Nothing else is taken from those records.
* **Paint.** The car wears its default paint; every car of a model shares one rig, so all the cars of a model have one colour.
* **Not offered.** A rule of time of day (the game's traffic has none), districts below a pattern, trailers, per-car horn or engine
  sound, a colour per spawn.

## Console

| Command | Does |
|---|---|
| `traffic cars` | Lists the traffic cars mods added: id, class, weight, limit, speed factor, patterns, and whether the model is loaded |
| `traffic spawn <car> [metres] [oncoming]` | Asks for the traffic car `car` (a mod's id, any case, or a `pvehicle` name of the game's) that many metres ahead of the player (30), heading the way the player faces, or towards the player with `oncoming`. It is placed on the nearest lane that can take a car when its model is loaded, at any density |
| `garage` (devtools) | Lists the folders with a section `mod traffic cars` |

## `bb.traffic` (1.0)

Ask `"bb.traffic" = "1.0"` under `[optional.interfaces]` of `mod.toml`. `Ctx::traffic()` answers the table, or `None` on a host without it.

| Call | Does |
|---|---|
| `list(sink, user)` | Writes the traffic pool, one line per car in registration order, to the sink during the call: `id`, `class`, `weight`, `max_instances`, `speed_factor` and `patterns` (joined by `,`), separated by a tab. No car is an empty text. A later minor may append fields to a line |
| `spawn(car, position, heading)` | Asks for the car (an id of the pool, any case) on the lane nearest `position` (render space; the height is not used), heading `heading` radians from +x. Queued: the scene places it at its next tick after the mods ran, when the car's model is loaded, and drops the ask after 30 seconds. `BB_ERR_NOT_FOUND` for a car not in the pool, `BB_ERR_ARG` for a name of 0 or more than 128 bytes or a number that is not finite, `BB_ERR_NOT_READY` while no world is loaded, `BB_ERR_HOST` when 32 asks wait already |

The SDK form is `Traffic::cars() -> ModResult<Vec<TrafficCar>>` and `Traffic::spawn(car, [x, y, z], heading)`. The 2005 interface
`bb.vehicles` `spawn_traffic` is unchanged: it places a **random loaded** traffic model, which now includes the mod cars.

| Entry | Implemented in |
|---|---|
| ABI table `BbTraffic`, layout test | `libs/blackbox-modding/src/abi/iface/traffic.rs`, `traffic_tests.rs` |
| Host trampolines, `TrafficBackend`, tests | `libs/blackbox-modding/src/host/table/traffic.rs`, `traffic_tests.rs`, `host/registry.rs` (`add_traffic`) |
| SDK | `libs/blackbox-modding/src/sdk/traffic.rs`, `sdk/ffi/traffic.rs`, `sdk/ctx.rs` (`Ctx::traffic`) |
| The game's port, the ask queue | `crates/nfsmw/src/modding/backends/traffic.rs`, `crates/nfsmw/src/traffic_requests.rs` |
| Pattern rules, speed, asked cars, console | `crates/nfsmw/src/scenes/world/ai/{mod_traffic,pattern,population,spawn,commands}.rs` |
| The pool and its numbers | `crates/nfsmw-data/src/car/mods/traffic.rs` |

## Compatibility

Append-only: new `[traffic]` keys and new fields on a line of `list` come with a minor and are ignored by mods built for an earlier
one; a key the game does not know refuses the car, so a manifest asks only for what its game has. The checks cover only the schema: that
a pattern name is one the game has is not checked, because the install's patterns are read when a world loads.

## The 2012 mod's traffic cars

[`mods/nfsmw-2012-content`](../../mods/nfsmw-2012-content/README.md) converts the 11 traffic bundles of the 2012 install
([chameleon-traffic.md](../formats/chameleon-traffic.md)) into traffic cars, unless its setting `traffic = false` (default `true`). The
ids are the make's first three letters and the model, cut to 9, then `_TR`; the display names end in `(traffic)`.

| Id | Car | Class | Drive | Weight | Patterns |
|---|---|---|---|---|---|
| `AUDA3_TR` | Audi A3 | compact | front | 10 | all |
| `DODCALIBE_TR` | Dodge Caliber | compact | front | 10 | all |
| `GMCCOBALT_TR` | Chevrolet Cobalt | compact | front | 10 | all |
| `NISVERSA_TR` | Nissan Versa | compact | front | 10 | all |
| `CADCTSV_TR` | Cadillac CTS-V | sedan | rear | 10 | all |
| `INFG35_TR` | Infiniti G35 | sedan | rear | 10 | all |
| `CHEEXPRES_TR` | Chevrolet Express Van | van | rear | 8 | all |
| `DODGRANDC_TR` | Dodge Grand Caravan | van | front | 8 | all |
| `DODRAM150_TR` | Dodge Ram 1500 | pickup | rear | 8 | all |
| `NISFRONTI_TR` | Nissan Frontier | pickup | rear | 8 | all |
| `GMCTOPKIC_TR` | GMC Topkick | truck | rear | 4 (one at a time) | `cityhighway`, `collegehighway`, `industrial` |

What is the 2012 game's and what is made up: the **body, wheels, lamps and collision** are the bundle's (the body at its levels 1, 2
and 4 of 5 as `A`/`B`, `C` and `D`; the wheels and the tyre sizes measured from the wheel model; the collision fitted to the bundle's
soup); the **class and the drive** are of the real cars, by name; the **stats** (mass, engine, gearbox, brakes, grip: `class.rs`
of the mod's `src/traffic/`) are the numbers of a road car of the class, **design values** (the bundles hold no handling, and the
set in `TRAFFICATTRIBS.BNDL` was not matched to its car); the **colour** is one of eight traffic paints chosen per car; the
**weights and patterns** are the mod's design, chosen so the 2005 cars keep about half of the road (a calculation from the rules, not a count in a running game). `cars` and `exclude` of the mod's
settings pick them by id or by a part of the name (`traffic` is in every name); `[performance]` does not apply. A car is about 2 MB of
geometry and 1 to 3 MB of textures (an estimate: not measured on a build), and converts in the background with the others.

To see them: `traffic cars` lists them, and `traffic spawn AUDA3_TR 30` puts one 30 m ahead. With the setting on and the game's
traffic at its default, the rules give the 2012 cars about 45 percent of the new cars in the default pattern (fewer in the others, which have more of the game's own records, and where the slot shares cap a type); this is a calculation, not a count in a running game.

## Looking at them without a window

```
nfsmw view-world --drive --exec "traffic spawn AUDA3_TR 25" --exec "traffic spawn GMCTOPKIC_TR 45 oncoming" \
  --exec "traffic warmup 8" --screenshot traffic.png --muted
```

`traffic warmup` simulates the seconds first (a screenshot has no frame time), which is also when the models load and the asks are
placed; `--exec "traffic 24"` fills the road from the patterns, with the mod cars among the others. The tests of the pure parts
(manifest checks, the pool's rules, the pattern merge, the ask queue, the ABI layout and trampolines, the SDK line parser, the mod's
ids, classes, stats and manifest) run with `cargo test -p blackbox-modding -p nfsmw-data -p nfsmw -p nfsmw-2012-content traffic`;
the readers on the install with `CHAMELEON_GAME_DIR` and `MW2012_GAME_DIR` set and `-- --ignored`.
