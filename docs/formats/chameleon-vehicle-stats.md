# Chameleon vehicle stats: what a 2012 car is, in numbers

Where NFS: Most Wanted (2012) keeps the base stats of each of its 93 cars, and how
[`libs/chameleon-vehicle`](../../libs/chameleon-vehicle/src/stats.rs) reads them. The containers are in
[chameleon-bundle.md](chameleon-bundle.md), the graphics side of a car in [chameleon-vehicle.md](chameleon-vehicle.md).
Evidence tags: **[verified]** is checked on the install (v1.4.0.0) with the Rust reader or a scratch script, **[exe]** is
a string or table of `NFS13.exe`, **[unconfirmed]** is read off the data and not proven by code.

## Short answer

Every car bundle (`VEHICLES/VEH_<n>_HI.BNDL`) holds its own handling definition: one big Genesys object (`GenesysObject`,
type 0x15) with the engine, gearbox, tyres, brakes, aerodynamics, steering and nitrous of the car, one with its rigid
bodies (mass, centre of gravity, box, wheels) and one with its axles. **93 of 93 cars** have all three **[verified]**.
The numbers are in SI units, except speeds (mph) **[verified by comparison with the real cars, below]**. The shared
`VEHICLELIST.BNDL` holds only the 110-row list of the UI (id, three fractions, the stat bars) and the nitrous and upgrade
types; it has no physics.

The numbers are the 2012 game's gameplay numbers, not always the real car's: the Porsche 911 Carrera S has the real
wheelbase (2.45 m), torque (440 N m at 5,600 rpm) and limiter (7,800 rpm), the Bugatti Veyron the real 268 mph, but the
1970 Dodge Charger R/T has a curve that peaks at 1,300 N m.

## How the data is laid out

### Genesys types and objects **[verified]**

`GenesysType` (0x14) resources describe structures; `GenesysObject` (0x15) resources hold them. A type's main block is
laid out as:

| Offset | Content |
|---|---|
| 0x10 | u32 type hash (also the third word of every instance, at +8) |
| 0x18 | u32 size of one instance |
| 0x20 | u32 `n`; the type's name is the zero-terminated string at `n + 8` |
| 0x28 | field records, 0x1C bytes each, until `n` |

A field record is `u32 type hash, u32 0, u32 name hash, u32 size, u32 offset, u16 count, u16 alignment, u32 0`. The ids of
the base types are the CRC-32 of their names (`float32_t` is 0x530A6D87, `bool8_t`, `uint32_t`, `Genesys.Object`,
all checked). **Field names are hashed with a function that is not the CRC-32 of the name**: a dictionary of 27,000
identifiers of `NFS13.exe` matched none of the 1,208 field hashes of a car bundle. The reader therefore addresses fields by
offset, and the meaning of each offset below was inferred from the values.

An object is one block of memory with every nested structure inline. Its import table has a *type import* for each
structure: the slot offset has bit 31 set and the low bits are the structure's offset in the block, the id is the type.
Structure offsets are listed in the import table in the order of memory. A structure starts with 12 bytes (two zero words
and the type hash); its fields follow at the offsets of its type. Arrays are an offset to the array and a count: a
structure array is a table of offsets to the structures (the torque curve), a small structure array is inline and
contiguous (the gears). The 352 types of a bundle repeat in the sound half of the bundle; only the first copy matters.

### The three objects

| Object | Found by | Holds |
|---|---|---|
| Handling definition (6,832 bytes for the Porsche, 116 type imports) | a `Genesys.Gen.HandlingBehaviour` | everything below except bodies and axles |
| Physical definition (3,360 bytes, 32 imports) | a `Genesys.Gen.RaceCarPhysicalDefinition` | box, rigid bodies, wheel cylinders |
| Vehicle info (432 bytes, 9 imports) | a `Genesys.Gen.VehicleInfo` | the two axles |

## Fields

`+x` is the offset in the structure (counting the 12-byte head). Units are those of the file.

### Engine **[verified]** unless tagged

`Genesys.Gen.Engine.Power_curve` has a pointer table at `+0x4C` and a count (one byte) at `+0x50`; each table entry is the
offset of a `Genesys.Gen.Point2DWithTangents` (72 bytes):

| Offset in the knot | Field |
|---|---|
| +0x0C | engine speed in rpm |
| +0x10 | torque in N m |
| +0x18 | in tangent: a `Point2D` (x at +0x0C, y at +0x10 of it), the vector the curve arrives with |
| +0x30 | out tangent, the vector it leaves with |

The curve has 3 knots in 69 cars, 4 in 16 and 5 in 8. **The first knot's rpm is the lowest speed the engine has and the
last knot's rpm is its limiter**: the Porsche is 1,000 and 7,800 rpm, the Lexus LFA 1,000 and 9,000 (the real LFA turns
9,000), the Lamborghini Aventador 1,000 and 8,250 (real: 8,250), the Veyron 1,000 and 6,750 (real: 6,700). The Tesla
Roadster, an electric motor, starts at 0 rpm with a flat 400 N m and ends at 14,000. The Porsche peaks at 440 N m at
5,600 rpm (real: 440 N m at 5,600). The tangents are vectors whose y over x is a slope in N m per rpm, and whose x is a
share of the segment's width (1.0 and 0.625 of 4,600 rpm on the Porsche's first segment); the reader builds a cubic
Hermite spline with those slopes **[unconfirmed: the engine may evaluate the vectors as Bezier handles]**.

The sixteen floats of the `Power_curve` structure (+0x0C to +0x48) look like a summary of the curve (lowest rpm at +0x20,
limiter at +0x2C, the rpm of the peak at +0x34, the peak at +0x44, and three negative numbers at +0x0C, +0x10 and +0x14
that may be engine braking in N m) but the peak is **stale on 32 of 93 cars** (the 1970 Charger says 637 N m and its curve peaks
at 1,300; the Porsche 911 GT2 says 440 and its curve peaks at 685), so the reader uses the knots only. The limiter at +0x2C does equal the last knot on all 93 cars and the lowest speed at +0x20 the first knot on 90 **[verified]**.

### Gearbox **[verified]**

A car has two `Genesys.Gen.Gearbox` structures (87 cars) or one (6 cars). They are the stock gearbox and the upgraded one:
the byte at `+0x52` is 0 for the first and 1 for the second, whose shift points differ and which has one more gear on
most cars (the Countach's stock box has five gears, the upgraded one six). The reader takes the one with the byte at 0.

| Offset | Field |
|---|---|
| +0x0C | the first gear, inline (reverse); the 28-byte gear structure is below |
| +0x38 | final drive ratio (2.37 to 4.92 over the cars; the Porsche 3.44) |
| +0x4C | offset of an array of the other gears (the first is neutral, ratio 0) |
| +0x50 | u16 number of gears in the array |
| +0x52 | u8 0 stock, 1 upgraded |

A gear (`Genesys.Gen.Gearbox.Gear`, 28 bytes): `+0x0C` lowest engine speed, **`+0x10` gear ratio** (negative for
reverse), `+0x14` the engine speed it shifts up at, `+0x18` the speed in mph it shifts up at. The Porsche's stock ratios
are 3.91, 2.29, 1.55, 1.30, 1.08, 0.88 and 0.71 and reverse 2.68; the Countach's 2.232, 1.625, 1.088, 0.858 and 0.707,
and the 930 Turbo's 2.0, 1.304, 0.893 and 0.70, the ratios of those real cars' boxes. Over the 93 cars: 4 to 7 forward
gears (4: 2 cars, 5: 13, 6: 49, 7: 28; the Tesla has 2), and in every car the forward ratios fall and are positive.

### Drive

`Genesys.Gen.Engine.Differentials` has four floats from +0x0C. **[unconfirmed meaning, consistent over 93 cars]**: the third
is the share of the torque sent to the front axle and the second and fourth are the lock of the front and the rear
differential.

| Third float | Cars | Drive |
|---|---|---|
| 0 | 63 | rear (the Porsche: 0, 0, 0, 0.75) |
| 1 | 4 | front (Focus ST, Fiesta ST, Alfa Mito and the Focus RS500: 0, 0.75, 1.0, 0) |
| 0.15 to 0.6 | 26 | all-wheel (the Gallardo: 0, 0.5, 0.3, 0.5; the R34 GT-R the same) |

### Mass, weight distribution and body **[verified]**

In the physical definition, `RaceCarPhysicalDefinition.PhysicalDefinition.RigidBody` appears five times: the body (the
heaviest) and the four wheels. A rigid body: **`+0xA8` mass in kg**, `+0x40` position (x left, y up, z forward; for the
body that is its centre of gravity above the ground, 0.45 m for the Porsche). The wheel order differs between cars (a
Caterham has two builds that list them in opposite order), so the reader sorts them by z and then x.

The car's mass is the body plus four wheels: the Porsche 1,295 + 24 + 24 + 27 + 27 = 1,397 kg, the Veyron 16.4 Super Sport
1,728 + 110 = 1,838 kg (the real car: 1,838), the SWAT van 8,000 + 140 kg. Over the cars 500 to 8,140 kg.

`PhysicalDefinition`: `+0x20` half size of the body box (x, y, z; the Porsche 0.9115, 0.5473, 2.2115 m: a car 4.42 m
long), `+0x10` the centre of the box. `…RigidBody.CylinderVolume` (four, in the order of the wheel bodies): `+0x58`
radius and `+0x54` half width of the tyre (the Porsche 0.34 and 0.123 m front, 0.343 and 0.148 m rear: 246 and 296 mm wide). `VehicleInfo.Axle` (two): `+0x20` the axle's z (1.245 and -1.205 for the Porsche: wheelbase 2.45 m). The wheel
bodies of the Hennessey Venom GT sit at z = 0.125 while its axle is at 1.3, so the **wheelbase and the weight distribution
use the axles**, the track (twice the mean |x| of a pair) uses the wheels. The Porsche's weight on the front axle comes out
at 38.8 percent; from the body's centre alone the Gallardo is at about 43, the BMW M3 at 51 and a Focus ST at 63.

### Tyres **[verified layout, meaning unconfirmed]**

Two `Genesys.Gen.Tyre` structures (front, rear; 416 bytes each) hold three lateral and four longitudinal grip curves, two
lateral and two longitudinal slip factors, and a few numbers (28,000 and 35,000 at +0x178 on the Porsche). A
`Tyre.Long_Grip_Curve` has four floats from +0x10. **[unconfirmed]** The first, of the first long-grip curve in each tyre,
is the tyre's peak longitudinal friction coefficient: it runs from 1.1 (F-150 Raptor) and 1.25 (Focus ST) over 1.5
(Porsche, most sports cars) to 1.75 (Ariel Atom) and 2.1 (Hennessey Venom GT), in the order a driver expects. What the
other curves are (surfaces, the sliding branch) is not decoded.

### Brakes **[verified layout]**

`Genesys.Gen.BrakeBehaviour`: `+0x10` front and `+0x14` rear, in N m at each wheel. The Porsche 1,300 and 1,350 (a
rear-engine car brakes harder at the rear), the Evoque 2,000 and 1,200, a front-drive Focus ST 2,000 and 1,000, the SWAT
van 6,000 and 5,000. A wheel torque of 1,300 N m at the 0.34 m tyre is 3.8 kN, 1.1 g on the Porsche's 1,397 kg when it
brakes with four wheels, which is what a road car does; the numbers scale with the car's weight over the 93 cars.

### Aerodynamics and top speed

`Genesys.Gen.AerodynamicBehaviour` (108 bytes): **`+0x60` is a speed in mph, the car's design top speed**, and `+0x44` is a
second speed, 40 mph above it in 83 cars. The Porsche 189 and 229, the Veyron 268, the Lexus LFA 202, the Gallardo 201, the
Tesla Roadster 125 (its electronic limit), the SWAT van 80. The numbers match the real cars' top speeds for the cars that
reach them. **[unconfirmed]** `+0x44` is the top speed with nitrous. `+0x10` (1.0 for sports cars, 2.5 for the Charger and
the SWAT van) looks like a drag multiplier. The remaining floats are not decoded; `Engine +0x10C` is another speed (189 on
the Porsche, 185 on the Countach whose `+0x60` says 170), also not understood.

### Nitrous

`Genesys.Gen.NitrousStrength` has a float at **`+0x3C`**: the strength, 1.15 (Veyron, Ariel Atom) to 2.5 (Delta
Integrale), 1.9 for the Porsche; its `Jump` structure holds the value two thirds of the way to 1 (1.6 for 1.9) and its
`Punch` structure the same strength again. **[unconfirmed]** It is the factor the car's force is multiplied by while
nitrous burns; the strongest cars have the weakest nitrous. The tank is in `Genesys.Gen.NitrousParameters`
(usage and recharge rules, five objects per car) and is not decoded.

## What is not decoded

| Data | Where | Why not |
|---|---|---|
| Suspension (`SuspensionWheel`, `WheelSuspensionConstraint`, `…SpringConstraint`) | handling definition | values like 5, 0.09, -1.5 and 0.35, 2, 0.08, whose units are not known |
| Steering curve (`SteeringBehaviour`, 28 floats) | handling definition | no field names |
| Drift, donut, handbrake abilities; hard-driving assists | handling definition | gameplay layers, not base stats |
| Tyre slip factors and the other grip curves | `Tyre` | meaning of the curves |
| Rigid-body inertia, damping | physical definition | the mod uses the engine's own |
| Upgrades (`PerformanceUpgrades`, 8,304 bytes, 192 imports in the Porsche's bundle; `VehicleCategoryInfo`) | handling definition, `VEHICLELIST.BNDL` | the shop of this game builds its steps from the car's own stats instead |

## NFS13.exe

**[exe]** The UI stat names are strings in the executable: `VehicleStats`, `NitrousStat`, `DriftStat`, `OffroadStat`,
`StrengthStat`, `ControlStat`, `TopSpeedStat`, `AccelerationStat` and the same with `Mod` and `Modded` (the seven bars of the
2012 garage and their upgrade previews), plus `GetPlayerTopSpeedMPH`, `GetPlayerTopSpeedRatio`, `PowerTrainUpgrade` and the
upgrade categories `PowerTrain`, `Nitrous`, `Body`, `Chassis`, `Tyre`. Their only references are Lua/UI binding tables
(for instance the property descriptors at 0xF58760 to 0xF58840), not the code that computes them. **The code that reads the
Genesys handling objects was not found or read**: the layout above is from the data. Nothing is copied from the executable.

## What the mod does with it

[`mods/nfsmw-2012-content`](../../mods/nfsmw-2012-content) reads these numbers with
`chameleon_vehicle::read_stats` and writes them to the `[stats]` table of each car's manifest
([car-customization.md](../modding/car-customization.md#stats-stats-in-the-car-manifest)):

| `[stats]` key | From |
|---|---|
| `mass_kg` | body + four wheels |
| `front_weight_pct` | body mass at its centre of gravity and the wheels at their axles, over the axles' z |
| `cg_height_m` | the same mass-weighted average of y |
| `wheelbase_m`, `front_track_m`, `rear_track_m` | axle z and wheel x |
| `front_tire_radius_m` … `rear_tire_width_m` | wheel cylinders (radius; width = 2 x half width) |
| `ride_height_m` | box centre y minus its half height |
| `idle_rpm`, `redline_rpm` | first knot (at least 500) and last knot |
| `torque_curve` | the spline sampled to at most 16 points that keep the knots |
| `gear_ratios`, `reverse_ratio`, `final_drive` | the stock gearbox |
| `drive`, `front_torque_share`, `front_diff_lock`, `rear_diff_lock` | the differential structure |
| `top_speed_kph` | `Aero +0x60`, mph to km/h; the game solves the drag from it |
| `front_tire_grip`, `rear_tire_grip` | the peak of the long-grip curves |
| `front_brake_torque_nm`, `rear_brake_torque_nm` | `BrakeBehaviour` |
| `nitrous`, `nitrous_boost` | strength - 1 |

Left out, so the game's generic values apply: springs and dampers (derived from the mass on each wheel), steering, downforce,
the handbrake, gear efficiency and engine braking.

## Checks

`cargo test -p chameleon-vehicle` has the spline tests. With `CHAMELEON_GAME_DIR=<2012 install>`,
`cargo test -p chameleon-vehicle -- --ignored` reads the Porsche's values listed above and the stats of all 93 cars, and
`cargo test -p nfsmw-2012-content stats -- --ignored` turns all 93 into manifests that the game's parser accepts.
