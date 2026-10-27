# Provenance: the 2012 gameplay cameras

Covers [`libs/chameleon-camera`](../../libs/chameleon-camera) and [formats/chameleon-camera.md](../formats/chameleon-camera.md):
the camera data of the 93 cars, the reader, and a behaviour model built from that data. Structure follows
[chameleon-tools.md](chameleon-tools.md). Process: [licensing.md](../licensing.md).

## Sources

| Source | License | What was used |
|---|---|---|
| The user's NFS: Most Wanted (2012) install (v1.4.0.0, `D:\Need for Speed - Most Wanted`) | n/a (never copied into the repo) | Every number tagged **[verified]**: the 93 `VEHICLES/VEH_<n>_HI.BNDL`, `VEHICLES/TRAFFICATTRIBS.BNDL`, and the strings of `NFS13.exe` |
| Ghidra, with `NFS13.exe` loaded (Ghidra MCP) | n/a | A bounded attempt to find the camera update code (below). Nothing was copied; no decompiled code or tool output is in the repository |
| [chameleon-vehicle-stats.md](../formats/chameleon-vehicle-stats.md) | this repo | How a Genesys type resource and a Genesys object are laid out (written earlier from the same install) |

No community tool describes these camera types, and no code from any source is in the crate. The Rust was written from
the layout in the format doc. Because the camera code of the executable was **not** read, no spec in `docs/specs/` was
written: there is no decompilation to put through the spec-first process.

## Method and tools

Python 3.14 scratch scripts in the session scratchpad (not in the repository), reading the install read-only and
reusing the earlier bundle probe (`probe.py`: header, entries, zlib blocks). Each is described so the result can be
repeated; the Rust test that repeats the check is named.

| Script | What it does | Reproduced by |
|---|---|---|
| `gen.py` | Parses the `GenesysType` resources (0x14) of a bundle into `{id: name, type hash, size, fields[(type hash, name hash, size, offset, count, alignment)]}` and the `GenesysObject` resources (0x15) into blocks with their type imports (offset with bit 31 set = structure offset) | `read_tests` (layout), `every_car_has_the_three_cameras_with_plausible_values` |
| `t1.py` | Lists every type whose name contains `Camera` in the Porsche's bundle with its fields | the table in the format doc |
| `t2.py` | Dumps every camera structure of one car with its floats at the type's offsets | `the_porsche_911_has_the_documented_values` |
| `t3.py` | Reads the 135 camera floats of all 93 cars into a table and counts distinct values per field | `the_globals_and_the_rear_view_are_shared_by_all_cars` |
| `t4.py` | Joins the camera numbers with the car's box (half extents, centre) and axles from the physical definition, to look for geometry in the camera numbers | none (negative result) |
| `t5.py`, `t7.py` | Prints the per-car table and the spread, mode and outliers of each varying field | the "Per car" table |
| `t6.py` | Reads `TRAFFICATTRIBS.BNDL` | `the_traffic_bundle_has_eleven_containers` |
| `t8.py` | Counts containers and shake effects per car | the "Short answer" in the format doc |
| `h1.py` to `h3.py` | Tries to recover the hash function of field names (below) | none (negative result) |

## Findings

1. **The camera types.** Each car bundle has 275 Genesys types; 24 are cameras. The ones above are described with their
   sizes, fields (hash, size, offset, count, alignment) and the container that ties them. The pointer fields carry
   alignment 0x104; inline structures 4. **[verified]**
2. **One container per car.** 93 of 93 bundles have exactly one object with a `VehicleCameraContainer` (1,744 bytes in the
   Porsche, 28 structure imports), and two `CameraGameplayShakeEffect`s (one inside the container's object, one separate),
   identical in all cars. `TRAFFICATTRIBS.BNDL` has eleven containers, same size. **[verified]**
3. **The values.** 135 camera values (floats, ids, small integers) per car were compared over 93 cars. 106 are the same in
   every car (the globals, the yaw spring, the rear view, the response triples, the field-of-view pairs); 29 vary,
   the ids that name the car's own objects among them, the rest mostly in the chase camera.
   The spread is in the format doc. The SWAT van (`1467242`) is the one car with a different bonnet, and its bumper holds
   the numbers the others have on their bonnet. **[verified]**
4. **No camera geometry in the mounted cameras.** The bonnet eye (1.3178 up, 0.2077 forward) is the same on 92 cars and the
   bumper eye (0.6, -1.5) on 63; joined with the box and axles (`t4.py`) the three bumper classes follow the car's size
   class (the four with a farther target are the longer, heavier bodies) but no number matches the bumper, the roof or the
   axle of a car. So they are offsets from an anchor the data does not name. **[unconfirmed]**
5. **Field names cannot be recovered with a hash.** The names of the nested structures are known (`Camera_Roll`,
   `Yaw_spring_modification`, ... are type names) and so are the 32-bit hashes of the members that hold them. `h1.py` tried
   those pairs against CRC-32 (reflected, MSB-first, with and without inversion, polynomials 0x04C11DB7, 0xEDB88320,
   0x82F63B78, 0xEB31D82E), FNV-1 and FNV-1a, Jenkins one-at-a-time, DJB2 (add and xor), SDBM, Murmur2 and Murmur3, over the
   name as written, lower case, upper case, with spaces for underscores, with a trailing NUL, reversed, and with the
   result inverted or byte swapped: **no match**. `h3.py` did the same with the type names against the type hashes
   (full name, without `Genesys.Gen.`, last component, dots as underscores): no match either, although the base types
   (`float32_t`) are plain CRC-32 (see chameleon-vehicle-stats.md). The hash is some other function.
6. **Mapping of numbers to roles.** Done by value and variation: speeds in mph fit the Tesla Roadster (125) and the shared
   180; the bumper and bonnet field-of-view pairs only make sense as horizontal angles; the rear view's 3.3333 and 400 look
   like a 400 by 120 mirror; the `Params` of the gradient adjustments are two equal numbers for climb and descent. Every one
   is **[unconfirmed]** in the format doc.

## The attempt on `NFS13.exe` (Ghidra)

Ghidra had `NFS13.exe` (53,821 functions) and the 2005 game's `speed.exe` open. Done:

* **Strings.** The executable names few camera things: `The transform of the Gameplay External camera.`,
  `GameplayExternalCameraTransform`, `The FOV of the Gameplay External camera.`, `GameplayExternalFOV`,
  `GameplayExternalImpactShakeAngles`, `IsCockpitCam` ("Is the in-car view - called bonnet in code"),
  `DrivingBehaviourCameraTransform`, `DrivingBehaviourFOV`, `CameraEntityModuleState`, a `CAMERAS.BNDL` name, and
  bystander and cutscene field-of-view limits. No Genesys field or type name of the camera data, no `Yaw_spring` or
  `Speed_based`. **[exe]**
* **References.** The strings are used by the registration function of the script module `CameraEntityModuleState`
  (`FUN_005d4270`), which binds each name to a getter. The getters are short: the transform getter copies a 64-byte
  matrix from offset 0x10 of a camera object that the entity keeps at offset 0x36A0 of its state, the field-of-view
  getter reads a float at offset 0x90 of the same object, and the impact-shake getter calls a virtual method (slot at
  0x2C) of it. **[exe]** So there is a camera object with a transform, a field of view and an impact shake angle
  output; its class, its constructor, its update and the code that reads the Genesys data were not reached.
* **Not found.** Searching for stores to the 0x36A0 slot found only the three reads above; scoring every function by how many
  of the chase camera's field offsets it loads as floats gave only generic vector and matrix code (the offsets are those of
  4x4 matrices too), and the type hashes of the camera types occur only in a dependency table, not next to code. The
  attempt was stopped there because the work was open-ended. A later session could start from the camera object's
  virtual table through the getters' owner, or from the places that read the Genesys object by its container type.

The Ghidra scripts were small Java scripts (a reference listing, an instruction scan for one displacement, a scoring pass);
they were removed from the Ghidra scripts folder afterwards. **Nothing is copied from Ghidra output into the repository**;
the behaviour model is built from the data and says so.

## What the model is, and is not

The model is a camera that uses the numbers of the car, with the shape the structure names suggest. It is **not** the
2012 game's camera. Where the file gives a number but no way to know its unit or role it is carried in the data types
(`param_*`) and left unused rather than guessed. The constants that are not in the data are options (`CameraOptions`) with
defaults, listed in the format doc.

## Reproducing

```sh
CHAMELEON_GAME_DIR="D:/Need for Speed - Most Wanted" cargo test -p chameleon-camera -- --ignored
```

reads all 93 cars and the traffic bundle, checks the ranges and the Porsche's values, and drives the model for every car
and view. The scratch scripts are not needed to repeat the checks.
