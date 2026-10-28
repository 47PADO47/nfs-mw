# RIDE 6 bikes (`mods/ride6-bikes`)

Part of the [modding catalogue](README.md). A content-only mod that adds a bike of the user's own RIDE 6 install to NFS:
Most Wanted as a mod car ([api-content.md](api-content.md#cars)). The repository holds the converter and the manifests
only: the glTF exports, the converted files (`files/`, `cars/`) and the textures come from the user's install, stay on
their machine and are never committed (the `.gitignore` whitelist does not list them). See
[licensing.md](../licensing.md).

## What it does

```
RIDE 6 install ──export (the user's tool)──▶ *.glb, every level of detail
*.glb ──ride6-bike-import──▶ mods/ride6-bikes/{mod.toml, cars/ktm450.toml, files/CARS/KTM450/{GEOMETRY,TEXTURES,BOUNDS}.BIN}
```

1. **Export.** RIDE 6 is Unreal Engine 5.6. Its `.utoc`/`.ucas` containers open without an AES key, and its assets need a
   `.usmap` mapping file (the packages use unversioned properties). A glTF exporter that reads Unreal packages (CUE4Parse's
   `CUE4Parse-Conversion`) writes one `.glb` per part and level of detail (`chassis369.glb`, `chassis369_LOD2.glb`, ...).
   The usmap is made by running the game with a dumper; RIDE 6 is protected by EasyAntiCheat, so that run is the user's
   own and offline. The importer does not need the game.
2. **Convert.** `cargo run -p ride6-bikes --bin ride6-bike-import -- <exports> mods/ride6-bikes` reads the parts
   listed in `src/layout.rs`, places them on the frame, and writes the car files.
3. **Run.** `nfsmw --mods-dir <folder with ride6-bikes/> view-car KTM450` shows it; the garage lists it as "KTM 450 SMR".

## The KTM 450 SMR 2022

| Part | Export | Where |
|---|---|---|
| Frame, tank, seat, engine, forks | `chassis369` | the bike's own frame |
| Wheels | `f_rim369`, `b_rim369` | the hubs `BONE_F_PHY_WHEEL` (0.7355, 0.225) and `BONE_B_PHY_WHEEL` (-0.739, 0.232) |
| Tyres | `f_tyre003`, `b_tyre002` | the same hubs (the radius of the rim picks the tyre) |
| Brake discs | `frontbrake369`, `backbrake369` | beside the hubs (z -0.063 and +0.071) |
| Exhaust | `muffler369`, `exmanifold369_369` | in the frame's own space |
| Bar pad | `barpad369_369` | at `BONE_BARPAD` |

Levels of detail (`LEVELS` in `layout.rs`): solids `A` and `B` take the chassis' level 2 (43,000 vertices) with the other
parts at level 1, `C` the chassis' level 4, `D` its level 6. The game's solids hold 60,000 vertices at most; the full
detail chassis has 133,000.

## How it differs from a car

- **Two wheels, one body.** The game draws a car as a body and four copies of one wheel solid. The importer puts the whole
  bike, wheels included, into the body solid; the wheel solid is a single hidden triangle. The wheels do not turn.
- **Physics is a placeholder.** The engine builds a four-wheel car from `[stats]`: the smallest mass the table takes is
  200 kg (the bike is 113 kg) and the smallest track 0.8 m. The torque curve and gearing are rough values for a 450 cc
  single, not the game's data. A two-wheel dynamics model is not built.
- **Axes.** The export frame is glTF's (x forward, y up, z right). The car's frame is x forward, y left, z up
  ([car-customization.md](car-customization.md#the-files-of-a-car)); `layout::to_car` turns one into the other.
- **Textures.** Pictures stored in the glb (the livery and the cockpit) are kept at up to 512 pixels as 32-bit maps. Surfaces
  without a picture get a flat grey for what they are made of.
- **No rider.** The rider (suit, helmet, boots) is exported and previewed, but not part of the car: the suit is exported
  in its rest pose, and a mod car has no seat for it.

## Not done

- A rider on the bike, and animated forks and wheels.
- The bike's torque curve, gearing and tyre data from `PhysicsRide6/Content/Physics/VehiclesTuning`, which hold them.
- More bikes: the importer's part table is one bike. A second bike needs its own table and `chassisNNN` numbers, found in
  `graphics/Bindings/Bike/*.uasset`.
- Sound, lamps and damage parts.
