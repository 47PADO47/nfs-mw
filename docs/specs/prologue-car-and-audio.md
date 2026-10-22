# Prologue races: the forced car and the sound of the intro

Two defects seen in a new career's prologue races (`16.1.0`, `16.2.2`, `16.2.3`, `16.1.1`, `16.8.1`, `16.2.1`; chain in
[story-flow.md](story-flow.md) section 2). Status: the car fix is built; the sound change is built on a reading of the code
and has not been heard against the original.

Tags: **[ours]** = a rewrite decision, **[verified]** = checked in the install's data or the rewrite's own code,
**[unconfirmed]** = a hypothesis.

## 1. The forced car resolved on the wrong folder

Every prologue race forces the preset ride `M3GTRCAREERSTART` (`RaceStory.player_car_type`, story-flow.md section 2.2).

- A preset ride belongs to one car type: the `PresetRides` record names it (`car_type_name`), and its part hashes are
  that type's solids. For `M3GTRCAREERSTART` the type is `BMWM3GTRE46` ([menu-backdrop.md](menu-backdrop.md) section 7,
  [menu-backdrop-looks.md](menu-backdrop-looks.md)). **[verified]**
- `CARS/` holds two different cars: `BMWM3GTR` (free roam's dark grey M3 GTR) and `BMWM3GTRE46` (the career-start
  white E46). **[verified]**
- The flow named the car `BMWM3GTR:M3GTRCAREERSTART`. The loader builds the preset's parts over the `BMWM3GTR` car type;
  the solids those parts name are not in that folder or in the shared folders, so each is skipped without an error
  (`nfsmw_data::car::load_with` takes only the wanted hashes that exist). The world starts with a car that has a
  physics body, an engine and a camera, and no meshes: the invisible car. **[verified]** by reading the loader.

**Rule [ours]:** a forced preset is loaded on the folder of the car type the install's `PresetRides` names for it
(`BMWM3GTRE46` for the prologue). When the tables or the preset are missing, the front end's default car folder
(`BMWM3GTRE46`) is used. Code: `frontend/flow_prologue.rs` (`forced_car`, `preset_folder`).

## 2. The sound of the intro

A prologue race starts with its intro (story-flow.md section 1.1): a story movie (`storyfmv_rac01`, ...), then the
intro NIS (`IntroNisDD`, `GenericStart`, `IntroNis05`, `IntroNis10`, `IntroNis04`), with the car held on the grid.

What the rewrite played during those steps **[verified]** (code reading):

1. The world is frozen (`update.rs`), but the car's sound state was still fed to the audio system every frame.
2. The mixer reads the listener from the active lens (`scenes/world/sound.rs`), which during the NIS is the NIS camera,
   not the chase camera. The engine's level and its low-pass cutoff come from the camera distance
   ([car-audio-environment.md](car-audio-environment.md) section 2: the cutoff falls to 725 Hz at 50 m and beyond). The
   idling engine, and the traffic, sirens and helicopter that follow the same state, were therefore heard quiet and
   muffled: "very low and far".
3. The radio kept playing under the movie and the NIS; the movie's own sound plays in the music group.
4. The NIS rev data (`SOUND/NISREVDATA`) that drives the engine during a NIS in the original is not read by the rewrite,
   so the engine had no revs to follow in the NIS either.

**Rule [ours]:** while a story movie plays, or the NIS of a race's intro does, the car sound state is withheld (as for the
pause menu): the engine and the sounds that follow the car stop, the radio stops, and the movie's or NIS's own stream is
the sound. They start again when the cutscene ends. The bust's arrest scene (a NIS outside a race intro) is not
changed. Code: `scenes/world/sound.rs` (`cutscene_owns_sound`).

**[unconfirmed]** that this is all the user heard. Two other candidates were read and left alone:

- The NIS stream keeps the first two of its six channels (`audio/pcm.rs`). If the cutscene's dialogue sits on another
  channel, the take is too quiet. The channel layout of `NISAudio.big` is not known; it needs a look at how the original
  mixes the six channels.
- The movie's sound plays in the music group, so it is scaled by the music volume (default 60 percent) and the master
  volume; the original's level for FMV sound is not known.
