# Menu backdrop looks: which car, which room, which camera

What the 3D picture behind the menus shows, and where each part of it comes from. The first backdrop (one stand-in car in
the crib) is in [menu-backdrop.md](menu-backdrop.md); which screen asks for which look is in
`docs/specs/career-parity-menus.md` section 7 (not repeated here). Addresses, file offsets and dead ends are in
the [provenance file](../provenance/menu-backdrop-looks.md).

- **Sources:** `speed.exe` (Black Edition PC v1.3, image base `0x00400000`) read through the Ghidra MCP; `GLOBAL/FE_ATTRIB.bin`
  (class `frontend`) read with `blackbox-attrib`; `FRONTEND/PLATFORMS/*.BIN` read with `blackbox-solid`; offscreen renders.
- **Evidence tags:** **[exe]** read in the executable, not tested in game. **[verified]** checked against the install files or a
  render. **[ours]** this rewrite's choice. **[unverified]** a reading that no picture has confirmed.
- Auto names are not used here; they are in the provenance file.

## 1. The three looks

| Look | Shown for | Room | Car | Camera |
|---|---|---|---|---|
| `Look::Main` | main menu, options, quick race menus, alias manager | crib (`PlatformCrib.BIN`) | the career-start preset `M3GTRCAREERSTART` of `BMWM3GTRE46` | the car viewer's orbit camera with its 0.5 s settle **[ours]**; unchanged |
| `Look::Car(car)` | the career menu, the quick play brief | crib | the look's car | the same as `Main` |
| `Look::SafeHouse(car)` | the safe house menu and the screens it opens | safe house (`CAREER_SAFEHOUSE.BIN`) | the look's car, turned as the safe house stands it | the safe house pose of section 4 |

`MenuScene::with_look(dir, look)` builds the backdrop (`frontend/scene.rs`); `Frontend::sync_backdrop` swaps it when the screen
on top asks for another look. A car that cannot be loaded gives way to the default car; if that is missing too, the flat dark
backdrop stays.

## 2. Which car

### 2.1 The default car: the red BMW M3 E46 GTR **[verified]**

`CarLook::default_car()` is the car type `BMWM3GTRE46` with no preset, in its stock paint: a red M3 with the wide arches, the GTR
body kit and the big rear wing. It is what a loaded profile with no usable current car, and no profile at all, show. The other
candidates, rendered with `nfsmw view-car` (provenance, section 4):

| Folder / preset | Picture |
|---|---|
| `BMWM3GTRE46`, stock | red, wide arches, big wing: **the default** |
| `BMWM3GTRE46`, preset `M3GTRCAREERSTART` | white with the same body kit and wing (the career-start car, the main menu's car) |
| `BMWM3GTR`, stock | dark grey street M3 GTR with no arches and a small spoiler (free roam's `DEFAULT_CAR`) |

The earlier `CarLook::starter()` named `BMWM3GTR`; that is the grey car, not the red one, so it was wrong.

### 2.2 The profile's car **[ours, from exe + verified data]**

`CarLook::of_profile(profile)` reads only the loaded profile (`CareerHandle::with_loaded`), no files:

1. No profile loaded: the default car.
2. The settings' current-car handle (`settings.car_handle`) is the starter handle `0x38A3C42C`: the career-start preset
   (`BMWM3GTRE46` + `M3GTRCAREERSTART`). [exe: [career-flow.md](career-flow.md) section 3] `bstring_hash("M3GTRCAREERSTART")` is
   that handle **[verified]**, which is how every preset ride is told from an ordinary car (rule 4).
3. The handle is not in the stable (`stable.car_by_handle`): the default car.
4. Otherwise the look keeps the record's handle and keys, plus the lot's name for the record's front-end collection
   (`career.data.cars`, `LotCar.name`, matched on `frontend_key`). The names are turned into a `CARS/` folder and preset when
   the backdrop is built (`CarLook::resolve`, which reads the install's car tables):
   - a **preset ride**: a `PresetRides` entry whose name hashes to the handle gives the preset and its car type
     (`BL15` gives `GTI`);
   - a **stock car**: the car type whose lower-case name hashes (`vlt_hash`) to the record's vehicle key, else to its
     front-end key (the vehicle key of `bmwm3gtr` is `0xCD88083F`) **[verified]**;
   - else the lot name through `pick_folder` (`cobaltss` gives `COBALTSS`);
   - else the default car.

The lot name alone does not find the folder for six cars (`a3_20t`, `a4_32`, `m3`, `rx8speed`, `carrera_gt`, `imprezzawrx`
are front-end names, not folders), which is why the keys come first. **[verified: probe of `FE_ATTRIB.bin` against
`CarTypeInfos`]**

## 3. The stage files

`FRONTEND/PLATFORMS/<stage>.BIN` hold the room; the stage number maps to a file in [menu-backdrop.md](menu-backdrop.md)
section 1. Neither the car nor a camera is in the file: **no solid carries a position marker** (`markers` is empty in all six
files), and the placement is in the executable (section 4) and the `frontend` attributes. **[verified]**

| File | Solids | Drawn |
|---|---|---|
| `PlatformCrib.BIN` (stage 1) | five `BACKDROP_BACKDROP_*` (`v18`, this install's is a modified room) | all five, as before |
| `CAREER_SAFEHOUSE.BIN` (stage 2) | `BACKDROP_CRIB` (the garage, 4,387 vertices, x -15..12.7, y -12..11.3, z 0..8.5), `BACKDROP_REFLECTION` (the garage mirrored under the floor), `BACKDROP_CURRGEN` (the bright sky outside the windows), four `BACKDROP_CAST_SHADOW_MAP*` and `BACKDROP_CAST_SHADOW_MAP_NEXTGEN` (shadow caster geometry), seven `STEAM EMITTER*` (particle markers) | the first three |

The shadow casters are what the original draws its shadow maps from, and the emitters are markers; neither is a visible part
of the picture, so they are skipped **[ours]**. Without `BACKDROP_CURRGEN` the windows show the clear colour and the floor loses
its bright reflection; with it the picture has the pale floor and the white windows the safe house is remembered for
(provenance, section 5). All three solids are pre-lit like the crib (`Shading::Prelit`).

## 4. How the original stands the car and the camera **[exe + verified]**

### 4.1 The car

The stand routine (`0x007A96C0`) places the car at the stage origin, turned about the vertical axis by a yaw taken from the
platform number (the front-end manager's `+0x18`):

| Platform | Yaw (degrees) |
|---|---|
| 1 crib, 4 shop backroom | 304.97 |
| 2 career safe house | 304.97 |
| 3 customization shop | 304.97 |
| 5 car lot | 340.0 (and tilts of -0.379 and -0.0002) |

(the same constant, `0x008B71A8`, serves four platforms; the car lot's is `0x008B71E0`.) The car's x and y are the two globals
at `0x009B9E50` and `0x009B9E54`, which are zero and never written. Its height is the mean wheel radius minus the mean wheel
height, less 0.025 (`0x008916CC`), so the tyres sink 2.5 cm into the floor. The front wheels are also steered 21.67 degrees
(`0x00905E20`) in the menu. The rewrite takes the yaw and the 0.025 and keeps the car's own `floor_height` for the rest
(`CarScene::stand_on`); the steering is not drawn **[unverified]**.

### 4.2 The camera

Each menu screen has a camera pose, found through `GLOBAL/FE_ATTRIB.bin`, class `frontend`:

1. The screen's collection name is the lower-case file name without the extension, prefixed by the context: `career_` when the
   career context bit is set, `carlot_` with the car lot bit, `customize_` with the customize bit (plus the part id),
   `quickrace_` or `quickracemain_` in the quick race menus, `options_` in the options, `online_` online, and no prefix on
   the main menu. A name with no collection gives `default`.
2. That collection holds a reference (field at layout offset 0) to an `angle*` collection, which holds the pose.

A pose is 11 values (layout offsets in brackets): pitch `[0x3C]`, yaw `[0x54]`, distance `[0x50]`, roll `[0x44]`, field of view
`[0x24]`, target x, y, z `[0x2C, 0x30, 0x34]`, the move's decay `[0x1C]`, duration `[0x20]` and order `[0x48]` (the last three only
shape the 0.4 to 1 s ease between two poses, which the rewrite does not play). The camera is built each frame as follows
(`0x004769E0`, `0x0046B7E0`, `0x006CF0A0`; all angles are degrees):

- the eye is `-distance * (sin(yaw) * sin(pitch), cos(yaw) * sin(pitch), cos(pitch))` from the stage origin;
- it looks at the target;
- its up vector is world up turned about the view direction by the roll;
- the field of view is the pose's `[0x24]` value, and 16-bit angle units (65,536 per turn) round every angle first.

`CameraPose` (`frontend/camera_pose.rs`) holds these and builds the lens.

### 4.3 The safe house pose

`MainMenu_Sub.fng` in the career context is `career_mainmenu_sub`, which refers to `angle10`; that pose is identical to
`angle_final_career_safehouse`, the pose the safe house's entry move ends on, which confirms the lookup. **[verified]**

| Pose | Pitch | Yaw | Distance | Roll | FOV | Target |
|---|---|---|---|---|---|---|
| `angle10` (the safe house menu) | 97.3 | 110 | 4.25 | 16 | 65 | -0.2, -0.5, 0.3 |
| `angle_entry_career_safehouse` (where the entry move starts) | 94.4 | -58 | 9 | -13.1 | 55 | 2.3, -0.3, 1 |
| `angle3` (`mainmenu`, the crib) | 97.7 | -3 | 5.5 | 7.3 | 55 | 1.2, -0.3, 0.3 |

The eye of `angle10` is at (-3.96, 1.44, 0.54): 4.25 m from the car, half a metre above the floor, behind and to the right of the
car's tail. That is the low rear three-quarter view of the red M3 with its wing and plate in a garage. **[verified: render]**
The other safe house screens have their own poses (`career_manager` `angle_career_manager`, `career_safehousebounty`
`angles_bounty`, `career_safehousemilestones` `angle21`, `career_safehouseracesheet` `angle_racesheet`,
`career_safehousereputationoverview` `angle20`, `career_safehouserivalchallenge` `angle22`, `career_car_select` `angle4_d`);
the rewrite shows `angle10` for all of them **[ours]**. Table of values: provenance, section 3.

## 5. Light, cost and leaks

- The car is lit by the front end's light rig, the same as the crib's (`original_lighting::frontend`). The original's safe house
  default matches it ([car-lighting.md](car-lighting.md)). The room is pre-lit. **[ours]**
- Building a backdrop reads the car and the room and uploads them: 64 to 130 ms for the crib looks and 90 to 190 ms for the
  safe house on a warm run (the first build of a session also reads the car tables, about 0.5 s). Nothing is added per frame:
  the safe house camera is one matrix. **[verified: install test timings]**
- `MenuScene::close` frees the car and the room's meshes and textures; the host parks and restores backdrops
  through it. Building and closing every look three times in a row leaves no mesh or texture behind
  (`frontend/look_tests.rs`). **[verified]**

## 6. Checking a look offscreen

`nfsmw view-screen <screen>.fng --look main|car|safehouse [--car FOLDER[:PRESET]] --screenshot out.png` fixes the backdrop to a
look whatever the screen asks for (both options are hidden). Without `--car` the look shows the default car. Examples:

```
nfsmw view-screen MainMenu_Sub.fng --look safehouse --screenshot safehouse.png
nfsmw view-screen MainMenu_Sub.fng --look car --car BMWM3GTRE46:M3GTRCAREERSTART --screenshot career.png
```

## 7. Not done, not verified

- The original's reflection of the car in the floor (`BACKDROP_REFLECTION` is the room's; the car's own mirror image is drawn by
  the front end's reflection pass) is not drawn. **[unverified]**
- The entry move (a 0.6 s ease from `angle_entry_career_safehouse` to the pose) and the user's rotate and zoom of the car on
  some screens are not played; the camera is the final pose from the first frame.
- The field of view is read as the vertical one; the original does not say. A comparison against the original at 4:3 would settle
  it. **[unverified]**
- The roll's direction (positive turns the up vector towards the camera's right) follows the axis-angle routine's convention, not a
  comparison with the original's picture. **[unverified]**
- The poses of the other platforms (customization shop, back room, car lot) are in the attributes but their rooms are not drawn.
- `BACKDROP_CURRGEN` is the sky only by its shape and its effect; the original's draw order and blending for it are not read.
