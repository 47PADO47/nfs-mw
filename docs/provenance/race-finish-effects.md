# Race finish effects (`scenes/world/finish_flares.rs`, `race/flares.rs`, `photo_shot.rs`, `frontend/photo.rs`)

- **Spec:** [race-finish-effects](../specs/race-finish-effects.md); data in
  [formats/gameplay-races.md](../formats/gameplay-races.md) and [formats/animation.md](../formats/animation.md).
- **Restricted sources read:** `speed.exe` v1.3 in Ghidra (read only: no function was renamed, retyped or changed in the
  project) and a byte-level disassembly of a stretch the Ghidra project has no functions for; the install's `GLOBAL/gameplay.bin`,
  `GLOBAL/ATTRIBUTES.BIN`, `GLOBAL/InGameB.bun` and `GLOBAL/HUDTEXTURESPHOTOFINISH.BIN` through the Rust readers
  (**[verified]**). The decompiled class sources (dbalatoni13/nfsmw) were not read. No code or bytes of the executable
  are copied; the understanding is written in the spec in the rewrite's own terms.
- **Evidence tags:** [docs/README.md](../README.md#evidence-tags). **[exe]** = read from the machine code or data of
  `speed.exe`; **[verified]** = measured on the install.

## 1. The flares **[exe]**

- The fields are read by their Vault hashes: `ParticleEffect` is `0x5EF34802` and `FlareSpacing` `0x394ABBC6` (the
  hashes of the names, as `blackbox_attrib::vlt_hash` gives them). The only code that reads them is one trigger method
  (`0x005E2B10`, found by searching the executable for the two constants). The strings `fxgame_flare_red`, `FlareSpacing`
  and `ParticleEffect` are in no data of the executable: the effect's name only comes from the trigger's data.
- The method returns at once when `ParticleEffect` is empty. It copies the trigger's position (three floats at `+0x58`),
  and reads `FlareSpacing`. For a spacing above 0 it takes the cross product (`0x0045F7C0`) of the vector `(0, 1, 0)` with
  the trigger's direction vector (`+0x68`), scales it by the spacing, and builds two points with `0x0045F640` (a
  scaled add): the position minus and plus that vector. It starts the effect at each (`0x005E2A50`) and stores the two
  handles at `+0xB8` and `+0xBC`. With no spacing it starts one effect at the position and stores it at `+0xB8`. The
  trigger space is the physics one (y up), so the cross product is the horizontal vector across the heading, of the
  heading's length: the spacing is the metres from the centre to each flare.
- `0x005E2A50` makes the effect object by name (the name is hashed: `0x004522F0`, `0x004519D0`), converts the point from
  the physics axes to the effect's (`x' = z, y' = -x, z' = y`, up stays up: a plain move with no turn), and puts it on the
  effect list. There is no ground test in it.
- The method is called from the trigger's enable path (`0x005E8540` and `0x005E87F0`, which turns on every trigger a
  race's trigger list names and bumps an enable count), from a refresh over all triggers (`0x005EBD90`, which also turns
  the effects on or off by the trigger's state) and from a reset (`0x005E2C60`). So the flares exist while the trigger is
  enabled, which the race does from its start.
- **[verified]** the 148 finish lines with the effect, their spacings (4 to 16 m), the 67 tollbooth races without one, and the
  four emitters of `fxgame_flare_red` (an `emittergroup`, class key `0xABA86E60`): the install tests in
  `crates/nfsmw-data/tests/real_install/gameplay.rs` and `src/world/finish_fx.rs`.

## 2. The win photo **[exe] [install]**

- The screen class (constructor `0x0058E630`, a `PhotoFinishScreen`, vtable `0x008A0F30`) keeps its argument at `+0x34`:
  0 a race, 1 and 2 two other kinds (2 is the speed trap: it shows `SPEEDTRAP_GROUP` and its camera frame). For argument 0
  it takes the texture name from the race: `PHOTOFINISH_RIVAL` when the game's race kind (`+0x1964` of the race manager) is 2
  and a test on the race (`0x005FBD90`, not read) holds, else a text field of the race (`0x005FCC60`: key `0x038A3B53`),
  hashes it (`0x00460BF0`), finds the picture object `0x286A9CD4` in the package (`0x005251A0`) and swaps its texture
  (`0x0056A880`: four slots of `(texture hash, object)`). Nothing renders to a texture: the pack is
  `GLOBAL/HUDTEXTURESPHOTOFINISH.BIN` (`0x008A1764`), four textures that `blackbox_tpk::read_texture_packs` reads:
  `PHOTOFINISH_SPRINT` (`0xA5D041A9`), `_RIVAL`
  (`0xB75D0B87`), `_DRAG` (`0xB7F38627`), `_TOOBOOTH` (`0xF2B144D7`, the package's own resource), 1024 x 512 DXT3, **[verified]**
  by decoding them: the polaroid's picture has alpha 0.
- The message handler of the screen is in a stretch of code Ghidra has no functions for (`0x005A7EA0` to about
  `0x005A8700`; the slots of the vtable point into it). Its disassembly shows: on the accept message (`0x406415E3`) the
  buttons; on `0xC98356BA` (a periodic tick) two timers off the game clock (ticks of 0.25 ms: `DAT_00925B14 * 0.00025`),
  0.75 s after the screen opens it posts a cinematic event (`0x00621120`: group `Cinematics`, a camera name, 0) and calls
  a method on every HUD element registered (`0x0057D0F0`; what it does was not read), and 0.75 s after that it runs the entrance script `0x0013C37B` on the event handler `0x47FF4E7C`. The camera name
  is `DefaultFinish` for argument 1, `DefaultSpeed` for argument 2, and the race's `FinishCamera` text (key `0x62DFC259`,
  which is `vlt_hash("FinishCamera")`) for a race.
- The cinematic event's destructor (`0x00640550`, where this engine does the work) registers the pair with the camera
  system (`0x0046E2D0`) and selects the camera action `CDActionIce` (`0x00479EB0`): the ICE replay player. An earlier event,
  `ECameraPhotoFinish` (`0x00640410`), only starts a camera blend on the camera manager (`0x0073DEF0`) and calls three
  routines that were not read; it sets no camera track.
- The camera names are tracks of the replay camera pack (`0x8003B203`, `ICEReplayCameras`, 36 records) of
  `GLOBAL/InGameB.bun`, found by their name in the record header (`+0x17`): `Challenge`, `DefaultSpeed`, `DefaultFinish`,
  `DragFinish`, `SprintFinish`, `TollFinish` and `PHoldFin_00` to `PHoldFin_28` and more. Their keys have eye and look in
  car space (space bytes 0); the values are listed in the spec. **[verified]** with the Rust reader
  (`ReplayCameras`, `the_install_has_the_finish_cameras_and_the_flare_group`). Which file holds the spaces' meaning is
  [formats/animation.md](../formats/animation.md) ("ICE camera keys"); that the car-space origin is the car model's origin
  (the look point of `SprintFinish` is at 0.87 m, a car's belt line, over the ground) is **[inferred]** from the data and
  confirmed only by how the rewrite's photo looks (a car filling the lower middle of the frame).
- **Not read:** the handler's other branches (the Restart and Race Stats buttons, the speed trap's texts) beyond what
  `frontend/photo.rs` already did, the test `0x005FBD90`, the cinematic's end (whether the camera is released when the
  screen closes), and whether the player's car is taken over after the finish line. Each is in the spec's section 4.
