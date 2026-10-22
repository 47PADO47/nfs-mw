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
| `Look::Main` | main menu, options, quick race menus, alias manager | crib (`PlatformCrib.BIN`) | the career-start preset `M3GTRCAREERSTART` of `BMWM3GTRE46` | the front end's camera (section 4.4): the `main_fe` intro, then the pose of each screen |
| `Look::Car(car)` | the career menu, the quick play brief | crib | the look's car | the same camera as `Main` (a first menu of this look plays the `career_manager` intro) |
| `Look::SafeHouse(car)` | the safe house menu and the screens it opens | safe house (`CAREER_SAFEHOUSE.BIN`) | the look's car, turned as the safe house stands it | the same camera: the `career_safehouse` intro, then the screens' poses |

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
| `PlatformCrib.BIN` (stage 1) | five `BACKDROP_BACKDROP_*` (`v18`, this install's is a modified room): `BACKDROP_BACKDROP_CRIB` (the room, pivot scale 26.04, bounds -222..222 m by 217 m tall) and four `..CAST_SHADOW_MAP*` | all five, each placed by its pivot matrix |
| `CAREER_SAFEHOUSE.BIN` (stage 2) | `BACKDROP_CRIB` (the garage, 4,387 vertices, x -15..12.7, y -12..11.3, z 0..8.5), `BACKDROP_REFLECTION` (the garage mirrored under the floor), `BACKDROP_CURRGEN` (the bright sky outside the windows), four `BACKDROP_CAST_SHADOW_MAP*` and `BACKDROP_CAST_SHADOW_MAP_NEXTGEN` (shadow caster geometry), seven `STEAM EMITTER*` (particle markers) | the first three |

The shadow casters are what the original draws its shadow maps from, and the emitters are markers; neither is a visible part
of the picture, so they are skipped **[ours]**. Without `BACKDROP_CURRGEN` the windows show the clear colour and the floor loses
its bright reflection; with it the picture has the pale floor and the white windows the safe house is remembered for
(provenance, section 5). All three solids are pre-lit like the crib (`Shading::Prelit`).

**Each solid is placed by its pivot matrix** (`Solid::transform`, translation in the last row, as stored). The safe house's drawn
solids have the identity; the crib's room is stored about 26 times smaller than it stands and its pivot scales it up
(26.04 on every axis), so drawn unscaled it is a small box round the car with its houses a few metres away. The earlier
backdrop (the orbit camera in a too small room) did not show it; the original's camera poses (section 4.4) put the camera
inside the unscaled houses. **[verified: render against a picture of the original's main menu]**

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
`[0x24]`, target x, y, z `[0x2C, 0x30, 0x34]`, the move's decay `[0x1C]`, duration `[0x20]` and order `[0x48]` (the last three
shape the move onto the pose, section 4.4). The camera is built each frame as follows
(`0x004769E0`, `0x0046B7E0`, `0x006CF0A0`; all angles are degrees):

- the eye is `-distance * (sin(yaw) * sin(pitch), cos(yaw) * sin(pitch), cos(pitch))` from the stage origin;
- it looks at the target;
- its up vector is world up turned about the view direction by the roll, against the right-handed sense: a positive roll lets
  the horizon fall to the right of the picture **[verified: the original's main menu (roll 7.3) and career menu (roll 24.7)]**;
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

### 4.4 The move onto a pose **[exe]**

The mover (`0x004769E0`, state 0) is given a start (its own pose when the move begins), a goal and the goal's decay `k`,
duration `d` and order `n` (`0x004770B0`). Every frame it adds the frame time to its clock, takes `t = clock / d` (1 once
`clock >= d` or when `d <= 0`) and blends

```
s = 1 - e^(-k t^2) * cos((n + 0.5) * pi * t^2)
pitch, yaw, distance, roll, fov = s * goal + (1 - s) * start      target = start + (goal - start) * s
```

`s` is 0 at the start, swings past 1 `n` times with a shrinking overshoot and is exactly 1 at `t = 1` (the cosine is zero
there), where the mover goes to state 2 and holds. The **field of view blends too** (the word at `+0xA0` is blended with
the others), and the blend ends by wrapping yaw and roll into 0..360 (pitch is not wrapped). `0x004770B0` first moves the
goal's pitch, yaw and roll by whole turns to the copy nearest the start (`0x0046B980`), so a move never takes the long way
round.

**The safe house intro.** `0x007A94C0` runs when the scene's intro flag (`+0x58`) is set (and for the garage by
`0x007A9620`): it builds the names `angle_entry_` + `career_safehouse` and `angle_final_` + `career_safehouse` (strings at
`0x008B74A0`, `0x008B74B0`, `0x008B722C`), sets the mover straight onto the entry pose (`0x0046B920`) and starts a move
to the final pose with the final pose's shape. While the flag is set `0x007B9EA0` records the screen's pose hash but starts
no move, so the intro is not cut short by the safe house menu's own pose (`angle10`, which has a 0.6 s ease). Poses read
from `FE_ATTRIB.bin` (provenance, section 3.6):

| Pose | Decay | Duration | Order |
|---|---|---|---|
| `angle_entry_career_safehouse` | 8 | 0.6 | 3 (not used as a goal) |
| `angle_final_career_safehouse` | 9 | **3.15 s** | 1 |
| `angle10` (`career_mainmenu_sub`, the menu's own pose) | 8 | 0.6 s | 2 |

A later screen with another pose hash starts a 0.4 to 1 s move from wherever the camera is; the rewrite shows one pose for
all safe house screens **[ours]**, so only the intro is played. **[verified: `real_install` test reads the values]**

`CameraMove` (`frontend/camera_move.rs`) implements the blend. The camera is one object for the whole front end,
`CameraRig` (`frontend/camera_rig.rs`), because the original's mover outlives its screens while a `MenuScene` is rebuilt
when the look changes (the main menu to the career menu): the front end and every backdrop hold a handle on it, the
backdrop's `update` advances it with the frame time and its `frame` takes the pose. The poses are read from the install
once (`nfsmw_data::frontend_camera::PoseBook`).

**The crib.** The stage name of `0x007A1E70` is `main_fe` in the crib and `career_manager` with the career context bit 8; the
rewrite plays `career_manager` when the first backdrop of a session is the career menu's (`Look::Car`) and `main_fe` for
every other crib look **[ours]**. `main_fe` runs 6 s from `angle_entry_main_fe` (93.1, -58, 18, ...) onto
`angle_final_main_fe`, which is `angle3`, the main menu's pose. The career menu in the crib is the career manager's screen
(`career_manager` -> `angle_career_manager`: roll 24.7, yaw 64, 4.5 m, the strongly tilted side view of the car) **[verified:
a picture of the original's career menu]**; the safe house menu is `career_mainmenu_sub` -> `angle10`.

**Screens.** A screen names its pose collection from its file name and arguments (`factory::pose_for`, the collection
names of section 4.2): `MainMenu` `mainmenu`, the career menu `career_manager`, the safe house menu `career_mainmenu_sub`,
the Quick Race sub menu `quickracemain_mainmenu_sub` **[unverified: which of the `quickrace_` and `quickracemain_` prefixes
the original uses where]**, Options `options_*`, the Quick Play brief, the challenge series, the track screens, the rival
challenge, the alias manager. A screen with no pose (dialogs, lists, the rest) keeps the camera. The stack gives the pose
of the topmost screen that has one (`Screens::pose`), and the front end hands it to the rig every frame
(`Frontend::sync_backdrop`): a pose that differs from the last one starts a move from where the camera is with the new
pose's decay, duration and order, so opening a screen and backing out of it both ease, to the pose of the screen that
is then on top. The first screen of a stage only notes its pose while the intro runs its first move, as the original's intro
flag does **[ours: the flag's other callers were not read]**.

A backdrop built from nothing (the menus coming up after free roam or a race) resets the rig, so its stage plays its intro;
a backdrop swapped for another look of the same stage keeps the camera. The Showcase keeps the car viewer's orbit camera
(the player turns it) **[ours]**. `view-screen --screenshot` starts the camera at the end of its intro (`CameraRig::settled`);
a scripted `play` run plays the intros.
The car stands as in section 4.1 in the crib too (yaw 304.97, platform 1).

**The room's turn.** With the car, the camera and the poses as above, the crib's houses stood in front of the camera. Turning
the room 180 degrees about the vertical axis puts Squidward's house behind the car's front bumper with its door under
the windows, and the pineapple behind the car in the career menu, as the original's two pictures show them
(`Stage::room_yaw`). The angle was found by trial against the pictures (180 +- 10 degrees); the executable was not found
to turn the room. The safe house needs none. **[unverified cause; verified against the pictures]**

## 5. Light, cost and leaks

- The car is lit by the front end's light rig, the same as the crib's (`original_lighting::frontend`). The original's safe house
  default matches it ([car-lighting.md](car-lighting.md)). The room is pre-lit. **[ours]**
- Per platform or per screen? The light research found one front-end rig (world-space entries, ambient zero) in the car lot, the
  safe house and the shop, with the back room's theta negated; the crib uses the same default, so the crib and the safe
  house need no different light. No per-screen light was found; the executable's light selection (`car-lighting.md`) was not
  re-read for this look. **[exe, from the lighting spec; unverified for the crib by a picture]**
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
- The intros and the screen moves of section 4.4 are played; the user's rotate and zoom of the car on some screens (state 1) are
  not, and the poses of the screens that `factory::pose_for` does not list (the rap sheet pages, the safe house sub
  screens other than the rival challenge, the customization shop) are not used: those screens keep the camera.
- The field of view is read as the horizontal angle of a 4:3 picture and kept as the vertical angle it implies at every window
  shape (65 degrees across give 51.1 up). The original does not say; the executable only converts the angle to 16-bit units
  for the view (`0x00476E10`). The car looks about 1.4 times as tall in the original's picture than with 65 read as vertical,
  which the 4:3 reading puts at 1.33. The original's main menu picture (1920x1080, pose `angle3`, 55 degrees) has the car
  0.40 of the picture high and the render of this reading 0.44, so the original's angle is a little wider still. A picture
  at a known window shape would settle it. **[unverified]**
- The pictures of the original show the main menu's car red and the crib's floor sand-coloured; the render stands the white
  career-start preset (section 1) on a dark floor strip with other textures. Neither is looked at here; the car and the
  textures are other work. **[unverified]**
- The poses of the other platforms (customization shop, back room, car lot) are in the attributes but their rooms are not drawn.
- `BACKDROP_CURRGEN` is the sky only by its shape and its effect; the original's draw order and blending for it are not read.
