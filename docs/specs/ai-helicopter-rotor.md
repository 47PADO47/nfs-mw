# The police helicopter's rotor sound

Part of [ai-helicopter.md](ai-helicopter.md), section 7: the bank, the publisher's words, the mixer state and what the
module does with each word.

## 1. Bank, class and words

- **Rotor bank and class [verified: install files, speed.exe].** The rotor is the `FX_Helicopter` AEMS class of
  `IG_GLOBAL/FX_Helicopter_MB.abk` (87 304 bytes on the Black Edition install), registered in the sound manager at
  `0x004ac950`. Its mixer state is 11, the helicopter state of the map ([formats/mixmap.md](../formats/mixmap.md)). The
  publisher `0x004d7810` stores twelve words in the class object at +4 to +0x30 and clamps them: word 0 to 32767, 1 to
  plus or minus 8192, 2 to 0..1, 3 to 32767, 4 to 65535, 5 to 1024, 6 to plus or minus 100 (the distance), 7 and 8 to
  25000, 9 and 10 to 32767, 11 to 1023 (the speed). The sound object's constructor `0x004dfce0` passes it twelve fields
  and sets the parent's ready flag. What the module does with each word is in section 2.
- **Not found:** the update that fills those twelve fields. `0x004dfce0` is in the sound class's vtable (`0x008999b0`),
  but the other slots are not function starts for the decompiler. The sources of the rotor's volume, pitch and azimuth
  (the mixer outputs) are open, see Q4.
- **Mixer [verified: installed `MAPOUTPUT.mxb`].** State 11, object 0: the volume master writes slot 1 (azimuth slot 0),
  the pitch master writes slot 2. Its inputs are one 3D control (spatial object 1, which reads controller 1: distances to
  the car and camera at 0 and 1, azimuths at 2 and 3, the has-position flag at 15), the state's sub-mix (events only)
  and state 0 events. The map does the distance rolloff, so the words carry no extra falloff.
## 2. The rotor module's words [measured: the installed module run in the remake's AEMS interpreter]

Each word was swept alone with the others held, and the thirteen players' inputs were read back
(`libs/blackbox-aems` install test `the_rotor_plays_its_loop_at_the_pitch_word_and_its_wash_up_close`).

| Word | Use |
|---|---|
| 0, 2, 8, 9 | Not read by the module. |
| 1 | Pitch, Q12 (4096 plays at the recorded rate). The loop takes it as is, the other layers an offset of it. 0 stops every layer. |
| 3 | Volume of every layer. |
| 4 | Azimuth, passed to every layer. |
| 5 | Rotation (0 to 512): picks the blade samples and bends their pitch. |
| 6 | Signed distance: fades between the near layers and the far layer; behind (negative) the loop goes quiet sooner. |
| 7 | Low-pass, Hz (25000 open). |
| 10 | Reverb send. |
| 11 | Speed (0 to 1023): fades the fast loop against the hover layers. |

The players: 0 is the turbine loop; 1 to 4 the blade chops (one-shots, re-fired); 5 to 8 the distant layer, rising
with distance; 9 to 12 the close **wash**, the wind of the downdraught: strong within about 5 m, a few percent of
that at 30 m, gone by 80 m. The mixer has no low-pass or reverb output for state 11, so the remake gives the open
filter and no send.

- **Rust.** `audio/heli` plays the module with the words above. The scene's `rotor_frame` gives the frame: the chopper body's position, nose
  direction and velocity. The rotor stops on pause or with no car.

