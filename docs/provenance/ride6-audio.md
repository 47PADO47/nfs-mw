# RIDE 6 audio and engine-sound behaviour

Modules: [`mods/ride6-bikes/src/sound`](../../mods/ride6-bikes/src/sound) and
[`libs/blackbox-carsound/src/overrun.rs`](../../libs/blackbox-carsound/src/overrun.rs).

- **Specs and formats:** [specs/ride6-engine-sound.md](../specs/ride6-engine-sound.md),
  [formats/ride6-audio.md](../formats/ride6-audio.md), [modding/ride6-bikes-sound.md](../modding/ride6-bikes-sound.md).
- **Sources read:**
  1. The user's own RIDE 6 install (Milestone, UE 5.6.1 build `5.6.1-1697004`): the unencrypted IoStore packages under
     `ride6/Content/assets/audio/` (events, bike audio data assets, the `CHASSIS_AUDIO` table, the audio Blueprint
     defaults) and `ride6/Plugins/PhysicsRide6/.../KTM_450SMR_2022` (engine min and max rpm), read with CUE4Parse and the
     game's `.usmap`.
  2. The UE SDK dump of that build (Dumper-7: class and struct layouts, names, function addresses).
  3. The game executable `ride6-Win64-Shipping.exe`, read in Ghidra: the vtables of the motor and backfire audio binders
     and of the physics effects element and the functions listed in the spec (`0x1451d0390`, `0x1451d51c0`,
     `0x1451d0280`, `0x1451b5030`, `0x1451c7440`, `0x1451436c0`, `0x145137500`, `0x1451377b0`, `0x14511be10`,
     `0x143ce3160`). Read for understanding; the spec restates the behaviour in our own words and equations.
- **Not read, on purpose:** the Wwise banks and `.wem` media. They are in a `.pak` with an encrypted index; no key was
  extracted, guessed or searched for, and none is stored. The rpm points, the crossfade curves and the backfire variant
  rule of Wwise are therefore unknown, and the spec says so.
- **What is written to the repository:** our own prose, equations and numbers (tuning constants of the shipped data
  assets, as for the other specs), code written from the spec, and synthetic tests. No decompiler output, no game
  recording, no bank, no extracted asset. The scratch tools (`resolve_bike_audio.py` and others) live outside the
  repository.
- **Licence:** `MIT OR Apache-2.0` for everything here. The numbers are facts about a product the user owns; the user's
  decoded recordings are theirs and stay in `C:\r6` or their mod folder.
