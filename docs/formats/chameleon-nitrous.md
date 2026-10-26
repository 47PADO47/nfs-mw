# Chameleon nitrous: what NFS: Most Wanted (2012) does when the nitrous burns

What the 2012 game's data says about the nitrous effects, for the `nitrous_variant` option ("NFS Most Wanted 2012", see
[nitrous-variants.md](../modding/nitrous-variants.md)). Everything was measured on the 2012 install (v1.4.0.0) with the
bundle reader ([chameleon-bundle.md](chameleon-bundle.md)) and by looking at strings and references in `NFS13.exe`. For the
evidence tags see [README.md](../README.md#evidence-tags): **[verified]** is read from the data and checked against
another source in it, **[exe]** is a string or table of the executable, **[unconfirmed]** is a reading that fits but is not
proved. No number here is invented: a number that is not in the data is not given.

The 2012 game does **not** use particles or camera data that name the nitrous: the effects are driven by the live value
`NitrousState.UsingNitrous` (and `InOverdrive`, the punch) of the local player, read by expression strings in Genesys
objects (kind `0x15`). That is how they are found.

## Summary

| Effect | What the data holds | Status |
|---|---|---|
| Sounds | a looping sample over the burn, a one-shot after it, a one-shot at the start, in `GAMELOGIC/HL_*.BNDL` | [verified] samples and loop flags; roles [unconfirmed] |
| Post effects (vignette, bloom, colour grade) | `POSTFX.BNDL`: a state bound to `UsingNitrous` and one to `InOverdrive`, with keyframes and colour cubes | [verified] values; field names [unconfirmed] |
| Post effect fades | the sequence of the sound set blends a post effect state in and out | times read, meaning [unconfirmed] |
| HUD | the `NitrousViewModel` signals in the executable | [exe]; the HUD movies are not read |
| Tail pipe flames | an effect on the car named by a `RaceCarEffectSequenceItem`; the effect resource is not in any loose bundle | not found |
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

## HUD **[exe]**

`NFS13.exe` has a `NitrousViewModel` (the HUD's data source) with these signals: `NitrousAmount` ("the current nitrous
amount"), `NitrousHighest` ("the highest nitrous amount since the player started using nitrous"), `UsingNitrous`, a
"nitrous punch" flag, `InOverdrive`, and `NitrousReadyPulse` ("cycles from 0-1 over an interval that decreases with the
amount of nitrous available"), `NitrousSignal`, `NitrousReadySignal`. The HUD itself is in the UI movies, which are not read,
so the look of the bar and its flash is unknown.

## Not found

- **Flames at the tail pipes.** The `RaceCarEffectSequenceItem` of the start sequence names an effect by an id that no loose
  bundle of the install holds (`GLOBALEFFECTS.BNDL` has 214 `BearEffect` resources, all named by hash). A car's `*_EXHAUST`
  markers are the only data about the pipes. The mod therefore leaves the 2005 game's own blue flame as it is.
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
