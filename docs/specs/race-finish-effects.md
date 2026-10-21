# Race finish effects: the finish line's flares and the win photo

What a race puts in the world at its end, and what the win photo (`InGamePhotoMaster.fng`) shows. Behaviour only; how
it was found is in [../provenance/race-finish-effects.md](../provenance/race-finish-effects.md). Evidence tags as in the
[docs README](../README.md#evidence-tags). The rest of the race is [race-mode.md](race-mode.md); the photo's text and
buttons are its section 11.

Code: `libs/blackbox-race/src/gate.rs` (`flare_points`), `crates/nfsmw-data/src/world/finish_fx.rs` (the flare group and
the replay cameras), `crates/nfsmw/src/scenes/world/finish_flares.rs` and `race/flares.rs` (the flares),
`scenes/world/photo_shot.rs` (the photo camera), `frontend/photo.rs` (the picture).

## 1. The finish line's flares

**[exe] [install]** A trigger shows an effect when it carries the `ParticleEffect` field (a text naming an
`emittergroup` collection) and the race turns the trigger on. The race turns its triggers on when it starts and off when
it is over, so the flares burn for the whole race, not only the last lap or the last seconds. Where they stand:

- `FlareSpacing` above 0: two flares, at the trigger's `Position` minus and plus `FlareSpacing` times the unit vector
  across the trigger. That vector is the cross product of the up axis and the trigger's heading, the same width axis a
  gate has (`blackbox_race::Gate::side`). `FlareSpacing` is therefore the distance of **each** flare from the centre:
  the pair stands twice that apart.
- `FlareSpacing` 0 or missing: one flare at the `Position`.
- The height is the `Position`'s own: the effect is not put on the ground.
- The effect is placed with no turn: its z axis is up, and it is a point, not a line along the gate.

**[verified]** Data: 148 of the 217 finish lines of the install carry `fxgame_flare_red`; every checkpoint has no effect.
The finish lines that have none are 67 of the 68 tollbooth races (a tollbooth race has no flares, the booth is the
mark), two sprints and the D-Day story pieces. The spacing runs from 4 to 16 m, road by road (5 on `10_2_1_sprint`, 9 on
`16_2_3_sprint`): it is authored to put the flares at the edges of the road the line crosses.

**The effect.** `fxgame_flare_red` is an `emittergroup` of four emitters, all read from the install by the same reader as
the exhaust flames and the explosion: `fxgame_flare_core_red` (10 a second, 0.5 s life, white to yellow to red, 0.25 to
1.75 m), `fxgame_flare_glow_red` (5 a second, 0.5 s, a yellow glow of 1 to 1.5 m, drag 0.25), `fxgame_flare_smoke`
(8 a second, 4 s, a red smoke that rises and grows from 1 to 6 m) and `fxgame_flare_sparks`. Nothing is one-shot: the
flare is a steady burner. The group's `IntensityRanges` is `(0, 0)`, which the reader treats as the full range.

**Rewrite.** At the start of a race (`begin_race`) the finish trigger's effect name and flare points go to
`FinishFlares`, which uploads the group's textures once and runs one set of four emitters per point on the fixed physics
step (frozen with the world behind the post-race screens). They go out when the race is taken down (`release_race`: the
end, an abandon, a restart). A race a mod registered has no trigger in the data and no flares. Tested by the pure
placement (`flare_points` and `race/flares.rs`) and, on the install, by `finish_lines_carry_the_red_flare_and_checkpoints_none`
and `the_install_has_the_finish_cameras_and_the_flare_group`.

## 2. The win photo is the world seen through a window

**[exe] [install]** The photo is **not** a picture rendered to a texture, nor a stored frame. The screen
`InGamePhotoMaster.fng` is drawn over the live world, and its picture object (`photofinish_toobooth.tga`, 1024 x 512,
at the back of the screen) is a polaroid on gravel and leaves whose window is **transparent** (alpha 0 inside, 255
outside; four such backdrops in `GLOBAL/HUDTEXTURESPHOTOFINISH.BIN`). What shows in the window is the game's camera
at that moment, which is the finish camera of section 3. Nothing is captured or kept; the "tilted photo" is the frame's
own artwork.

The backdrop is chosen by the race: its text field `0x038A3B53` (name unknown) names the texture, and the screen swaps the
picture's texture for it when it opens. The four textures are `PHOTOFINISH_SPRINT` (gravel and leaves; 196 races),
`PHOTOFINISH_DRAG` (cracked yellow paint; 12), `PHOTOFINISH_TOOBOOTH` (69; the texture the package ships) and
`PHOTOFINISH_RIVAL` (the screen takes it, in place of the race's own name, when the game's race kind is 2 and a test
on the race holds; the test was not read, so no photo here takes it **[unconfirmed]**). A race that names a texture the pack lacks (7 races
name `PhotoFinish_Sprint.tga`) takes the kind's own in the rewrite; what the original draws there is **[unconfirmed]**.

**Rewrite.** `PhotoReport::picture` carries the backdrop (`viewer::photo_picture`: the race's name when the pack has it,
else the kind's), `PhotoLogic` swaps the picture's texture and shows it, and the UI assets hold the pack
(`ui/assets.rs`: four compressed 1024 x 512 textures, about 2 MB). Without the texture the picture stays hidden and the
world shows whole, as before. The speed trap's photo (`SPEEDTRAP_GROUP`) keeps the package's own picture hidden: its
camera frame is another object, and its picture is not in this pack **[unconfirmed]**.

## 3. The finish camera

**[exe] [install]** When the photo screen has been up for 0.75 s it posts a cinematic: the cinematic group `Cinematics`
and a camera name, which sets the game's camera to the ice action (the replay camera player) on that track. The name
depends on the screen's argument: for a race it is the race's `FinishCamera` field (`DefaultFinish`, or a numbered
`PHoldFin_NN` that stands the camera by the road the race ends on); for the speed trap's screen it is `DefaultSpeed`; for
a kind that was not read (argument 1) `DefaultFinish`. `DragFinish`, `SprintFinish` and `TollFinish` are in the pack, but
no race names them. 0.75 s after the cut the screen's entrance script runs (the polaroid, the text and the buttons come
in), so the camera has settled when the photo shows.

The cameras are tracks of the replay pack (`0x8003B203`) of `GLOBAL/InGameB.bun`, in the format of
[../formats/animation.md](../formats/animation.md) ("ICE camera records"): 36 tracks of two or three keys, 2 to 5 s long (the finish ones 2 to 3 s).
Eye and look point are in **car space** (key bytes 4 and 5 are 0), the car being the player's: **+x forward, +y left,
+z up**, the origin the car model's. `SprintFinish`, for example, starts at an eye of (1.49, -1.35, 1.69) looking at
(1.04, -1.15, 1.58) and moves in 0.64 s (the first 0.212 of its 3 s) to an eye of (3.98, -2.41, 0.89) looking at
(3.55, -2.16, 0.87), where it holds: four metres ahead of the car and two and a half to its side, at the height of its
beltline, looking back at it; the lens is about 22 to 24 mm, a roll of a few hundredths of a turn. The photo is the camera
at rest.

**Rewrite.** When the race-over message ends and the report has a photo, `start_photo_shot` takes the track
(`ReplayCameras::finish_track`: the race's `FinishCamera`, else `DefaultFinish`, `DragFinish` for a drag race) and the
player's car as it stands, and the world's camera (`gameplay_lens`) is that camera at the end of the track (it does not
play the cut-in) while the post-race screens are up. The car is drawn whatever camera mode the player drove with. It ends
with the race (`release_race`). The camera is placed by `photo_shot::place` (car, hybrid and world spaces).

## 4. What is not done, or not known

- **The car after the line.** The photo shows the player's car where the world froze, which is where the race-over
  message ended, 3 s after the line. In the original the flares stand behind the car in the photo, which says the car is
  near the line then; whether the game takes the car over after the finish and brakes it (as an autopilot would) was not
  read **[unconfirmed]**. The rewrite leaves the car under the player's hands for those 3 s, so a car that crossed fast
  is well past the line in the photo. A demo run (`race start <race> demo`) puts the car at the line.
- **The cut-in.** The camera's first 0.64 s is not played; the photo screen has no animated frame, so only the settled
  picture is seen.
- **Which side.** The cameras are on either side of the car: `SprintFinish` is at `y = -2.41` (the car's right, with
  the axes of [car-assembly.md](car-assembly.md): +y left), `PHoldFin_15` at `y = +1.76` (its left, which is the
  front-left view of the screenshots of the original). The sign of +y was taken from the car-space convention, not from a
  comparison with the original's picture of a track with an eye to the right **[unconfirmed]**.
- **Keys 2 and above and the smoothing** of ICE keys are not applied (`docs/formats/animation.md`).
- **`PHOTOFINISH_RIVAL`** is never chosen (section 2).
- **The speed trap's camera** (`DefaultSpeed`) is in the pack but the trap's photo still shows the frozen game camera.

## 5. Offscreen check

`nfsmw play --drive --no-sound --exec "wait 12" --exec "race start 10_2_1_sprint demo" --screenshot out.png
--screenshot-delay 40 --screenshot-count 6 --screenshot-interval 1.5` runs the demo race to its end and photographs the
photo screen: the tilted polaroid over the leaves with `WINNER`, the time and the footer, the car seen from ahead and the
red flare smoke behind it. `--exec "race start 10_2_1_sprint"` followed by `--exec "wait 12"` and `--exec "tp 4329 40"` (a
spot before the finish line) shows the two red flares at the road edges. A drag race (`1_7_3_drag`) takes the yellow
cracked backdrop. (The console's `race` command only runs once the scene exists: a `wait` of ten seconds or more comes
first.)
