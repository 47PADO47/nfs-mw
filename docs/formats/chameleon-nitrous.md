# Chameleon nitrous: what NFS: Most Wanted (2012) does when the nitrous burns

What the 2012 game's data says about the nitrous effects, for the `nitrous_variant` option ("NFS Most Wanted 2012", see
[nitrous-variants.md](../modding/nitrous-variants.md)). Everything was measured on the 2012 install (v1.4.0.0) with the
bundle reader ([chameleon-bundle.md](chameleon-bundle.md)) and by looking at strings and references in `NFS13.exe`. For the
evidence tags see [README.md](../README.md#evidence-tags): **[verified]** is read from the data and checked against
another source in it, **[exe]** is a string or table of the executable, **[unconfirmed]** is a reading that fits but is not
proved. No number here is invented: a number that is not in the data is not given.

The 2012 game has no data that is *named* for the nitrous in its particle and camera files. The sounds and the post effects
are driven by the live value `NitrousState.UsingNitrous` (and `InOverdrive`, the punch) of the local player, read by expression
strings in Genesys objects (kind `0x15`); the tail pipe particles are driven by a bit of the car's light-state word. That is how
they are found.

## Summary

| Effect | What the data holds | Status |
|---|---|---|
| Sounds | a looping sample over the burn, a one-shot after it, a one-shot at the start, in `GAMELOGIC/HL_*.BNDL` | [verified] samples and loop flags; roles [unconfirmed] |
| Post effects (vignette, bloom, colour grade) | `POSTFX.BNDL`: a state bound to `UsingNitrous` and one to `InOverdrive`, with keyframes and colour cubes | [verified] values; field names [unconfirmed] |
| Post effect fades | the sequence of the sound set blends a post effect state in and out | times read, meaning [unconfirmed] |
| HUD | the `NitrousViewModel` signals in the executable | [exe]; the HUD movies are not read |
| Tail pipe effects (start burst, blue glow, pop-off flame) | `SpotEffect` records of every car name the effects; their emitters, pictures and the glow's colour are in `GLOBALEFFECTS.BNDL` | [verified]: which effect, its emitters, pictures, glow colour; the emitters' programs (rates, lifetimes, speeds) are not decoded |
| Camera (field of view, shake) | no type or value that names the nitrous | not found |
| Speed lines, motion blur | no type or value that names the nitrous | not found |

## Sounds **[verified]**

`GAMELOGIC/HL_31208.BNDL` is the sound set of the heat levels (the only `HL_` bundle). Its `Genesys.Gen.HeatLevelSoundSet`
object has two fields for the nitrous (`Genesys.Gen.HeatLevelSoundSet.Nitrous`), each a `Genesys.Gen.Sequence`: a timeline
of items (`WaveSequenceItem`, `MixerChannelSequenceItem`, `BusMixerChannelSequenceItem`, `RaceCarEffectSequenceItem`,
`PostFXSequenceItem`, `PostFxOverrideSequenceItem`, `JumpTimelineController`, `QuitSequenceTimelineController`).

| Sequence | Expression strings | Samples it imports |
|---|---|---|
| burn | `Players.LocalPlayer.NitrousState.UsingNitrous` (and `(NOT)`), `GeneralSequence.Triggers["NITROUS_POSTFX_OFF"].IsTriggerSet[1](NOT)` | one **looping** stereo sample, 7.88 s, 48 kHz (stream loop flag set); one stereo one-shot, 1.88 s |
| start | `GeneralSequence.Triggers["NITROUS_POSTFX_OFF"].IsTriggerSet[1](NOT)` only | one six-channel one-shot; its first stereo pair decodes to 1.08 s of audio (the resource header says 3.28 s, which is the length the sequence gives it) |

- The burn sequence's two wave items: the loop starts at 0 and is given 8.885 s; the one-shot starts at 8.7 s of the
  timeline and runs to 10.594 s (**[unconfirmed]** reading of the fields: the one-shot is the sound after the burn). The
  jump controllers of the sequence read `Players.LocalPlayer.NitrousState.UsingNitrous(NOT)`, which fits a loop that is left
  when the nitrous stops.
- The sample roles used by the mod (loop = the burn, the one-shot of the same sequence = the end, the one-shot of the other
  sequence = the start) come from the loop flags and from which sequence holds the sample. That the other sequence is the
  *start* is **[unconfirmed]**: it holds the trigger name `NITROUS_POSTFX_OFF` but not `UsingNitrous`, and a
  `RaceCarEffectSequenceItem` (an effect of the car, id `0x1B0A3B`, not found in any loose bundle).
- The sound is routed by mixer channel items (a bus for the effect, the DSP chains of the sound set); the levels and the
  curves between the live values and the gains are in `ModulatingDoubleValue` items and are **not decoded**. The mod plays the
  samples at their own level.

## Post effects **[verified values, field names unconfirmed]**

`POSTFX.BNDL` holds a `Genesys.Gen.PostFXConfig` (object `0x78C7`) that lists the states: a default one and states bound to
live values by an expression string:

| State object | Bound to | Keyframe | Colour cube |
|---|---|---|---|
| `0x78AE` | (default) | `0x78DC` | `0x78F1` |
| `0x78F9` | `Status.HeatValue` | `0x78E5` | `0x789D` |
| `0x78F5` | `Status.IsInCooldown` | `0x78D9` | `0x7898` |
| `0x78FA` | `Reporter.NitrousState.UsingNitrous` | `0x78E9` | `0x789F` |
| `0x78FB` | `Reporter.NitrousState.InOverdrive` | `0x78EA` | `0x78A0` |
| `0x78F6` | `Players.GetLocalCompetitor.IsAboutToBeWreckedAndNotInSpectatorMode` | `0x78E0`, `0x78E1` | `0x789A`, `0x789B` |
| `0x78F8` | `Status.HasSomeoneJustEscaped` | `0x78E4` | `0x789C` |

A keyframe is a `Genesys.Gen.PostFxKeyframe` with the parts `Vignette`, `Bloom`, `General`, `Depth_of_Field` and
`Stereo_3D`. Field names are hashed; the fields are read by offset and their roles guessed from the names the executable
lists (`VignetteAngle`, `VignetteSharpness`, `VignetteAmountX`, `VignetteAmountY`, `VignetteCentreX`, `VignetteCentreY`,
`VignetteOuterColour`, `VignetteInnerColour` **[exe]**).

Values, the default keyframe and the nitrous one (offsets in the object):

| Part | Field | Default | Nitrous (`UsingNitrous`) | Overdrive (`InOverdrive`) |
|---|---|---|---|---|
| Vignette | two numbers at `+0x20` | 0.5, 0.3 | 0.526, 0.51 | 0.5, 0.3 |
| Vignette | two numbers at `+0x30` | 0.7, 0.555 | 0.459, 0.522 | 0.791, 0.668 |
| Vignette | colour at `+0x50` | 0.451, 0.451, 0.451, 1 | 0.037, 0.114, 0.089, 1 | 0.156, 0.156, 0.178, 1 |
| Vignette | the number at `+0x74` | 0.34 | 0.258 | 0.384 (with -0.3, 0.6, 1.3, -0.4 at `+0x64..+0x70`) |
| Bloom | colour at `+0x90` | 0.402, 0.402, 0.402, 1 | 0.597, 0.831, 1.0, 1 | 0.456, 0.896, 0.93, 1 |
| Bloom | the number at `+0xA8` | 0.2 | 0.2 | 0.6 |
| Bloom | the number at `+0xBC` | 1.35 | 1.35 | 1.55 |
| General | numbers at `+0xE4..+0xEC` | 1, 0.053, 1 | 1, 0.1, 1 | 1, 0.05, 1 |

The nitrous state pulls the vignette in on both axes, darkens its colour towards a dark teal, and tints the bloom blue; the
overdrive keeps the grey vignette and brightens the bloom.

Colour cubes (`0x52`, 16x16x16 entries of 3 bytes after a 16-byte header, 12,304 bytes). The nitrous cube (`0x789F`) is not
the identity: grey (index 8,8,8) reads 128, 147, 157 in its stored byte order, and the primaries are pulled towards each
other (pure red 37, 34, 223). The overdrive cube (`0x78A0`) **is** the identity. The default state's cube (`0x78F1`) is also
a grade. The byte order of the stored colours (blue first, as the identity cube's pure red `0, 0, 255` shows) makes the
nitrous grade a **warm** shift of the greys: red up, blue down **[unconfirmed]**.

The post effect is also blended by the burn sequence: its `PostFXSequenceItem` and `PostFxOverrideSequenceItem` items name a
second state (`0x162049`, keyframe `0x16204A`, in `HL_31208.BNDL`) with a vignette of (0.5, 0.501), (0.85, 1.0), a colour of
(0.047, 0.038, 0.072, 0.602), and numbers 1.302, 1.1, -0.178, 0.374. The items carry the times 9.55 and 1.117 (at the end of
the burn) and 0.75 and 0.25 (at its start): read as *start the blend at, for how long*, in seconds **[unconfirmed]**. The
state objects hold the pair 2.0, 1.0 (nitrous and overdrive) next to their expression, read as a blend time and a weight
**[unconfirmed]**.

**What the 2005 game here can do with it:** nothing yet. Its renderer takes a vignette that adds light and a radial blur
(`blackbox_gfx::FocusFilter`); the 2012 vignette darkens and the bloom and the grade are not controls of the renderer. The
values are kept here so that a renderer that grows those controls can read them.

## Tail pipe effects **[verified]**

The effect id of the sound sequence (`RaceCarEffectSequenceItem`, `0x1B0A3B`) is not what plays the flames; the car does. Each
car bundle has a `Genesys.Gen.VehicleVfxBehaviour` object (the one that holds the lamps, see
[chameleon-lamps.md](chameleon-lamps.md)) with `SpotEffect` records: at +0x0C a reference to an effect, at +0x14 a locator
group (the tail pipes, or another place), at +0x18 a bit of the **light-state word**. The Porsche 911 (`VEH_1085007`) has eight:

| Effect (`GLOBALEFFECTS.BNDL`, kind `0x301`) | Group | Bit | What it is made of (emitters in order) |
|---|---|---|---|
| `0x16205B` | pipes | 9 | three smoke puffs (`0x18092F`: a 128 x 128 soft cloud), a flame (`0xF5824`: the 512 x 512 flame sheet), a 128 x 128 flipbook (`0x69DE0`), a second flame |
| `0x22030B` | pipes | 20 | one emitter, material `0x250D1`: a 256 x 256 soft glow |
| `0x24F2D` | pipes | 14 | one smoke puff, a flame, the 128 x 128 flipbook, a second flame, and a last emitter with no picture (`Position`, `Velocity`, `Age`, `RecipLifetime`, `Radius`: no colour, **[unconfirmed]** a light) |
| `0x24F34` | pipes | 8 | three smoke puffs |
| `0x24F2F` | pipes | 19 | a material of shader `0xF7E12` and two cloud pictures (**[unconfirmed]** heat haze) |
| `0x24F5A`, `0x14023F` | `0x1401FC` | 12 | other places |
| `0x220308` | `0x126DFD` | 20 | one smoke emitter (in 6 cars it is listed in the pipes' group too, before `0x22030B`) |

**The bits.** The executable's light-state routine builds the word each frame **[exe]**. Bit 9 (`0x200`) is set when the vehicle's
state byte (at +0x195B of the vehicle, +0x33 of its nitrous input record) is 1; bit 20 (`0x100000`) when it is 3 and two more
conditions hold (a per-player flag is 2 and a vehicle check passes), only for a vehicle that is not flagged as traffic; bit 14
(`0x4000`) when neither the pedal bit (8, `0x100`) nor bit 9 is set and a timer that starts at 2 at a gear change is running; bit
8 when the pedal is pressed. The byte comes from the nitrous button's state in the input system, remapped **[exe]**: its states
0 and 1 (nothing, and **no nitrous to use**) both give 0, 2 gives 1 (the button was just accepted: the start), 3 gives 2, 4 gives 3
(burning). So an empty tank with the button held is state 0: **no start, no burn, no effect and no sputter** are in the data. The
lamps page read bit 9 as the reverse gear; no car has a reverse lamp record and the two screenshots below say otherwise.

**Looked at the screenshots of the game** (a white Porsche 918, from behind): while the nitrous burns, a blue-white glow sits
in each pipe, with bloom (effect `0x22030B`); when it lights, an orange-yellow flame bursts from each pipe, with sparks and dust
(effect `0x16205B`: the flame sheet and the small flipbook). That fits the pictures: the flame sheet is 25 frames of an
orange fireball in a 5 x 5 grid, and the glow is a grey soft cloud that the effect colours.

**Cars.** 87 of the 93 cars name the same start (`0x16205B`) and pop (`0x24F2D`) effects and a burn effect `0x22030B`; six
(`122699`, `122706`, `122713`, `510072`, `1085764`, `1467242`) have no pipe effects. The mod takes the three from the first car
that has them.

**An effect** is a bytecode program per emitter. The format, the operation table and the parameter blocks are in
[chameleon-effects.md](chameleon-effects.md); read with it, the three effects are (numbers are the effect files'):

| Effect | Emitter | Spawn | Life (s) | Look |
|---|---|---|---|---|
| start `0x16205B` | 3 smoke puffs, 128 x 128 | 80, 300 and 300 a second, **no window end** | 0.2-0.4, 0.05-0.225 | small cyan-white additive puffs, 0.07-0.37 m wide, riding with the pipe; scaled by an input of the crash kind: **not played** |
| | 2 flames, 512 x 512 sheet | 100 and 40 a second for **0.5 s** | 0.1-0.2 | 0.29 to 0.68 m, rising at 10 m/s2, riding with the pipe |
| | flipbook, 128 x 128 | 100 a second for 0.5 s | 0.2-0.5 | small orange sparks |
| burn `0x22030B` | glow, 256 x 256 | 5 a second plus 1 for each m/s of the car, no window end | 0.25-0.5 | colour (0.03, 0.52, 0.86), 0.3 m wide growing to 0.7 m at the peak of its alpha and to 5.6 m at the end of its life, takes 0.8 to 0.9 of the car's velocity at birth, then **loses 75 % of it every step** (the file says it is left behind as a trail; the screenshots show none, and the mod lets it ride with the car) |
| pop `0x24F2D` | smoke, 2 flames, flipbook | 100, 100, 40 and 100 a second for 0.5 s | 0.1-0.5 | the same flames, with a big faint dark smoke |
| | a last emitter with no picture | burst of 1 | 0.25 | a light (radius 0.5), not played |

The glow's colour is the first colour of its gradient: (0.0307, 0.521, 0.863, 1.0), the blue seen in the screenshot. Not
decoded, and assumed 1 or 0: the effect's parameter array (it scales the rates and the shell), the flipbook cell rule, the blends,
the light and the heat haze (`0x24F2F`, refraction sprite). The mod turns what is decoded into its profile
([nitrous-variants.md](../modding/nitrous-variants.md#the-2012-variant)).

## HUD **[exe]**

`NFS13.exe` has a `NitrousViewModel` (the HUD's data source) with these signals: `NitrousAmount` ("the current nitrous
amount"), `NitrousHighest` ("the highest nitrous amount since the player started using nitrous"), `UsingNitrous`, a
"nitrous punch" flag, `InOverdrive`, and `NitrousReadyPulse` ("cycles from 0-1 over an interval that decreases with the
amount of nitrous available"), `NitrousSignal`, `NitrousReadySignal`. The HUD itself is in the UI movies, which are not read,
so the look of the bar and its flash is unknown.

## Not found

- **The effect's parameter array**, the flipbook cell rule, the blends and the lights: see [chameleon-effects.md](chameleon-effects.md#decoded-and-not).
- **Camera.** The car bundles have `CameraGameplayExternal` (with `Speed_based_movement.High_Speed_Shake`,
  `Acceleration_movement`, `Camera_Roll`) and `CameraGameplayExternalGlobals.LensProperties`; none of them names the nitrous.
  The executable has "The FOV of the Gameplay External camera" and "Max Fov", "Min Fov" in a debug table. A field of view or
  shake that follows the nitrous is not in the data read.
- **Speed lines and motion blur.** `MotionBlurAmounts` is a shader constant of the executable; nothing ties it to the nitrous.
- **Underglow, heat shimmer.** Nothing in the data.

## How to repeat this

1. Scan the bundles of the install for the strings `NitrousState`, `NITROUS_POSTFX` and `Nitrous` (blocks are zlib streams;
   the strings are in block 0 of kind `0x15` and `0x14` resources).
2. Follow the imports of the objects that hold them: waves (kind `0x81`, decoded as in
   [chameleon-sound.md](chameleon-sound.md)) and post effect states.
3. Read the floats of the keyframes at the offsets above.

The mod's real-install test (`MW2012_GAME_DIR`, `nitrous::tests`) finds the sequences and the three samples.
