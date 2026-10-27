# Chameleon traffic and low-detail bundles

What the `_TR` and `_LO` bundles of `VEHICLES/` in NFS: Most Wanted (2012) hold, and what
[`libs/chameleon-vehicle`](../../libs/chameleon-vehicle) and the content mod do with them. Containers are in
[chameleon-bundle.md](chameleon-bundle.md), the car bundle in [chameleon-vehicle.md](chameleon-vehicle.md). Tags as in the
[README](../README.md#evidence-tags): **[verified]** is measured on the install (v1.4.0.0) with the Rust reader,
**[unconfirmed]** is a reading.

## The three kinds of vehicle bundle

| Files | Count | What |
|---|---|---|
| `VEH_<n>_HI.BNDL` | 93 | the cars the player drives ([chameleon-vehicle.md](chameleon-vehicle.md)) |
| `VEH_<n>_LO.BNDL` | 93, the same numbers | the same car for the distance: body, wheels and lamps again, no sounds or handling |
| `VEH_<n>_TR.BNDL` | 11 | the **traffic cars**: eleven road cars that are not in the vehicle list |
| `TRAFFICATTRIBS.BNDL` | 1 | the handling, camera and sound objects of the eleven traffic cars (not decoded per car) |

## Traffic bundles (`_TR`)

A traffic bundle is a car bundle with the player's parts taken out **[verified]**: one `VehicleGraphicsSpec`, three models
(the body and two wheels; five for the Topkick), 13 to 21 renderables, 16 to 19 materials, 13 to 18 textures, one
`PolySoupList` of 186 to 268 triangles, 11 to 14 Genesys objects (the lamp object `Traffic_<Make>_<Model>_Coronas` among them), and
744 to 1,109 KB on disk (a `_HI` bundle is 1.5 to 5 MB). It has **no** `HandlingBehaviour`, no paint record, no skeleton, no driver
setup, no sound and no hinged part. `chameleon_vehicle::load`, `read_lamps` and `load_poly_soup` read it unchanged; `read_stats`,
`paints`, `Skeleton::read` and `driver_setup` find nothing. The name of the car is the one thing the bundle states: the lamp
object's name, read by `chameleon_vehicle::traffic_name`.

| File | Object name | Wheelbase | Length x width x height (m) | Tyre radius | Body, finest level |
|---|---|---|---|---|---|
| 122816 | `Audi_A3` | 2.58 | 4.20 x 1.96 x 1.29 | 0.313 | 9,680 triangles |
| 122818 | `Cadillac_CTSV` | 2.88 | 4.82 x 2.06 x 1.26 | 0.337 | 11,618 |
| 122819 | `Chevrolet_ExpressVan` | 3.44 | 5.69 x 2.40 x 1.85 | 0.385 | 9,823 |
| 122821 | `Dodge_Caliber` | 2.63 | 4.41 x 2.08 x 1.30 | 0.327 | 10,177 |
| 122822 | `Dodge_GrandCaravan` | 3.08 | 5.14 x 2.21 x 1.99 | 0.345 | 9,964 |
| 122824 | `Dodge_Ram1500` | 3.57 | 5.82 x 2.40 x 1.62 | 0.421 | 9,368 |
| 122827 | `GMchevrolet_Cobalt` | 2.62 | 4.58 x 1.99 x 1.23 | 0.330 | 9,677 |
| 122828 | `Infiniti_G35` | 2.86 | 4.63 x 1.93 x 1.23 | 0.333 | 8,945 |
| 122829 | `Nissan_Frontier` | 3.20 | 5.22 x 2.15 x 1.53 | 0.388 | 9,671 |
| 122830 | `Nissan_Versa` | 2.61 | 4.30 x 1.96 x 1.55 | 0.310 | 9,907 |
| 392322 | `GMC_Topkick` | 5.74 | 8.21 x 2.90 x 2.92 | 0.432 | 10,735 |

(Widths include the mirrors. The wheelbases are those of the real cars, so the hubs and the body agree: no axle needs the arch test
of [chameleon-vehicle.md](chameleon-vehicle.md#hubs-and-the-bodys-arches); the overhangs are 11 to 23 percent of the length.) The
classes are the eleven: four compacts (A3, Caliber, Cobalt, Versa), two saloons (CTS-V, G35), two vans (Express, Grand
Caravan), two pickups (Ram, Frontier) and one truck (Topkick) **[verified by the names]**.

* **Levels of detail [verified].** The body has **five levels** (the Versa has a sixth of 240 triangles): the A3's are 9,680,
  5,142, 2,862, 1,558 and 100 triangles, falling the same way in the others (the fourth is 649 to 1,625, the fifth 96 to 100). They
  are in order, unlike the `_HI` bundle's. The wheel (tyre, rim, disc, caliper) has four levels (the A3's: 1,152, 567, 240 and 90
  triangles). The surfaces are the ones of the player cars: `BodyPaint` (1 or 2 meshes), `Body`, `Glass`, the `EmissiveFourChannel`
  lamp circuits (head, brake, tail) and a `Licenseplate`, so the paint, lamp and plate conversions of the content mod apply.
* **Paint [verified].** No bundle has a paint record; the body's paint meshes are drawn with the 2012 `BodyPaint` shader, which
  takes its colour from the game at run time. The content mod therefore gives each car one colour of its own choosing.
* **Handling [unconfirmed].** `TRAFFICATTRIBS.BNDL` holds 282 Genesys objects and 279 types, 11 `VehicleSound` and 11 `ConvexHull`
  resources and 55 waves: eleven sets of `Genesys.Gen` handling, camera, engine and tyre objects (the types include a
  `HandlingBehaviour`, a `Gearbox`, an `Engine` and `Tyres`) and the wheel rigid-body links. Which set is which car, and the
  units, were not decoded: its camera containers are read by [chameleon-camera.md](chameleon-camera.md). The content mod gives the
  traffic cars the stats of their class (design values), not these.

## Low-detail bundles (`_LO`)

`VEH_<n>_LO.BNDL` has the number of a `_HI` bundle (93 for 93) and the same spec, body, wheels, lamps and collision soup, with more
levels of detail and without the car's sounds, handling and paints **[verified]**. Its body has **five or six levels**; for the
Porsche 911 Carrera S they are 16,870, 16,870, 5,617, 827, 95 and 436 triangles, against the `_HI` bundle's 30,908, 827 and 436.

* The `_HI` bundle's second level is one of the `_LO` bundle's (the same triangle count) in 87 of the 92 cars that have a second level, and
  its third is the `_LO` bundle's sixth in 91 of 92: the `_HI` bundle's coarse levels are mostly the `_LO` bundle's own, so importing
  them again adds nothing. The test is `chameleon_vehicle::traffic::low_detail_tests` (ignored, needs the install).
* What the `_LO` bundle adds is the **mid-detail level**: 4,765 to 11,366 triangles (5,000 to 6,100 in 81 cars), between the
  finest (about 30,000) and the 800 to 1,400 triangle model, which the `_HI` bundle lacks; its first two levels are an
  11,000 to 18,000 triangle model, no gain over `_HI`. 92 of 93 cars have one; the BMW 1M's body (Lightning McQueen's, 5,672
  triangles) is too small to have one. Two cars have a body of five levels; in one of them (2196000) the mid level is the second.
* The content mod writes `A` and `B` from the finest level (the hinged parts are cut out of it), `C` from this mid level and `D`
  from the `_HI` bundle's 800 to 1,400 triangle level, for the cars that have no livery (`low_detail.rs`). A car with liveries
  keeps its own levels: a livery changes the texture coordinates of the paint meshes, which the `_LO` bundle's do not follow.
  The game picks the level from the car's size on the screen (`scenes/world/drive/car_detail.rs`: the main view's levels change at
  120, 25, 20 and 10 pixels, a traffic car's at 80, 40, 20 and 10).

## The reader and the converter

`chameleon_vehicle::traffic_name(bundle) -> Option<TrafficName { make, model }>` reads the lamp object's name (`None` for every
`_HI` and `_LO` bundle, tested on the install). `mods/nfsmw-2012-content` converts the eleven bundles to traffic cars
(`src/traffic/`) and adds the mid level to the player cars (`src/low_detail.rs`); see its
[README](../../mods/nfsmw-2012-content/README.md) and [api-traffic.md](../modding/api-traffic.md).
