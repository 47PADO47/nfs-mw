# Collision and impact geometry of a mod car

A car a mod adds is an entry in the car list with files of its own. Its collision is its own too: the shape it hits the
world with, the box its rigid body is built from, and the zones its damage is counted in come from one file in its folder,
`BOUNDS.BIN`. Nothing is taken from a stock car's bounds set or damage record. A car without a valid `BOUNDS.BIN` cannot
be driven: the game refuses it and says why.

```
mods/example.pack/files/CARS/FASTCOUPE/
  GEOMETRY.BIN      the body and wheels
  TEXTURES.BIN      the textures
  BOUNDS.BIN        the collision bounds (this page)
```

## The file

`BOUNDS.BIN` is the game's own bounds format: a `BoundsPack` chunk (`0x8003B900`) holding the car's set as a `0x3B901` chunk
([formats/collision.md](../formats/collision.md#bounds-8003b900-boundspack-decomp--verified)). The set is keyed by the hash of the
folder name in upper case (`vlt_hash("FASTCOUPE")`); a file with a single set is accepted under any key.

The set is a tree of boxes and spheres in the car's own space: **x across (right is +x), y up, z along the car (forward is
+z), metres, origin on the ground between the wheels**. The units in the file are millimetres. Positions of the nodes below
the root are relative to the **root's pivot**, and so is the point cloud.

| Node | Flags | Is |
|---|---|---|
| 0, the root | `0x40` box | The extents of the car. **Half dimensions** are the size the rigid body is built from (they also decide where a hit counts as front, rear, side, roof or underbody); the **pivot** is the centre of the box in the car's space |
| lower body box | `0x46` box, vs world and objects | From the ground to about 0.75 m, as wide and long as the car |
| cabin box | `0x46` | Over the high part of the body, on top of the lower box |
| 4 front corner spheres | `0x82` sphere, vs world | Radius 0.33 m, at the nose, low and high on each side |
| 2 roof spheres | `0x82` | Radius 0.24 m, at the front top corners of the cabin |
| ground mesh | `0x50` box with a point cloud | The side silhouette of the body, 8 corners a side, mirrored: the convex shape the car slides and rolls on |

That is the layout of the commonest car set of the original game (36 of its 86 have exactly these ten nodes; the others
have fewer, see [formats/collision.md](../formats/collision.md#the-layout-of-the-car-sets-verified)), and it is what the game
builds the body from. A set may differ, within what the game checks:

- the root is a box with half dimensions between 0.25 and 3 m (across), 0.15 and 3.5 m (high), 0.5 and 12 m (long);
- the nodes form a tree from the root, each reached once, with children inside the node array;
- every shape has a volume, every mesh for the ground names a point cloud that exists, and no cloud is empty or has a point
  more than 24 m away;
- at least one node below the root collides (a box or sphere vs world or objects, or a ground mesh).

A node's `surface` is the road surface key of what
it is made of (`carbody` is `0x99DA6531`); 0 is the object's default.

## What uses it

| Part | By |
|---|---|
| Root half dimensions and pivot | the rigid body (`VehicleSpec::dimension`, where the box sits against the model's origin), the damage zones |
| Boxes and spheres | contacts with the world, other cars and props |
| Point cloud | contact with the ground and the roadside |

## Damage

Visible damage is counted in ten zones, `DZ_FRONT`, `DZ_REAR`, `DZ_LEFT`, `DZ_RIGHT`, the four corners, `DZ_TOP` and
`DZ_BOTTOM` ([specs/vehicle-damage.md](../specs/vehicle-damage.md)). A hit's zone is decided from where the contact is against
the **half dimensions of the root box**, so the zones fit the car: front and rear start beyond half the half-length, a corner
also needs the contact beyond half the half-width, the roof and underbody need the contact beyond half the half-height. A car
therefore gets its zones from its own extents and from nothing else.

How much a hit counts for is `CarEventSpec::racing_defaults()`: the table every racing car of the original shares (all of
them link to one record, `racers`, so it is not a per-car value): 20 kN per level, a scale per zone of 0.8 front, 1.5 rear, 3.5
sides and front corners, 7.5 rear corners, 3.5 roof and 0.7 underbody, no hit points (it is never destroyed by damage), shock
40 for 3 s, no event sequencer (its timed sequencer effects do not play, as for the original's racing cars). The converted 2012 cars
have no corner damage overlays (`DAMAGE0_*` solids), so their dents do not show as a model swap; the 2012 game does not have them
either ([formats/chameleon-collision.md](../formats/chameleon-collision.md#damage-and-impact)).

## Making one

- **The 2012 content mod** writes one for each of its 93 cars from the car's own collision mesh in its 2012 bundle, with the
  tyres, and the visible body for the one car without a mesh ([nfsmw-2012-content.md](nfsmw-2012-content.md)). The 2012 game
  has no impact-point geometry to take over: [formats/chameleon-collision.md](../formats/chameleon-collision.md).
- **Any other mod** builds the set from points sampled on its car (its collision mesh, or the vertices of its body, plus the
  tyres) with `blackbox_collision::fit_body(name_hash, &points, &FitOptions::default())` and writes it with
  `blackbox_collision::write_bounds_pack(&[set])`; both are in `libs/blackbox-collision` and take and give bytes. Points are in
  the car's axes above. `BoundsSet::check_vehicle_body(&BodyLimits::default())` checks a set the way the game does.
- **By hand**, any tool that writes the format of [formats/collision.md](../formats/collision.md) will do.

## For the game's code

The pure entry points of `nfsmw-data` (`crates/nfsmw-data/src/car/physics`), which read `BOUNDS.BIN` through the install and
therefore through mod overlays, and read nothing else:

```rust
pub fn mod_car_bounds(dir: &GameDir, folder: &str) -> Result<(CarBounds, BoundsSet)>;
pub fn mod_car_collision(dir: &GameDir, folder: &str) -> Result<ModCarCollision>;   // bounds + geometry + events
CarGeometry::for_mod_car(bounds: Arc<BoundsSet>) -> Option<CarGeometry>;
CarEventSpec::racing_defaults() -> CarEventSpec;
```

## Not done

- Per-car damage numbers: the 2012 bundles hold per-car damage objects, but their zone numbers are not decoded
  ([formats/chameleon-collision.md](../formats/chameleon-collision.md#damage-and-impact)), so every converted car takes the shared table. Their hinged parts are decoded and played ([car-damage-parts.md](car-damage-parts.md)).
- The set was not compared with how the cars drive in the game with a window: the checks are the numbers (every one of the 93 sets is a
  valid vehicle body whose box holds the visible body to within 0.22 m) and the unit tests, not a run.
- Rear corner spheres, wheel spheres and a separate hood or door collision are not part of the layout; the stock sets have none.
