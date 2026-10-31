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
     `0x143ce3160`). Read for understanding; the spec restates the behaviour in our own words and equations. Its
     strings were also hashed (FNV-1, lower case) to name four game parameters of the sound banks.
  4. **The Wwise sound banks and media of the install** (`ride6/Content/WwiseAudio`, 1096 banks, 1372 loose media), read after
     the **`.pak` index key**, which **the user supplied** (found at <https://cs.rin.ru/forum/viewtopic.php?f=10&t=100672>,
     not visited by the agents; **verified by mounting**: 88 paks instead of 44 mount and 1096 banks appear). The key is a
     constant of the RIDE 6 mod only (`mods/ride6-bikes/src/sound/pak_key.rs`) with `R6_AES` and the `pak_key` setting as
     overrides. The banks were read with our own parser (`bnk`, `hirc`, `rev`), decoded with `vgmstream-cli` (ISC
     licence, <https://github.com/vgmstream/vgmstream>, a separate program, not part of this repository); the
     extraction itself uses CUE4Parse (Apache-2.0, <https://github.com/FabianFG/CUE4Parse>) as an external tool.
     Wwise's bank format is described in our own words in [formats/ride6-audio.md](../formats/ride6-audio.md); no Audiokinetic
     code or header was read.
- **Not reproduced:** the Crankcase Audio REV synthesiser (its parameter block is not interpreted; only the table of its
  recordings is read), the Wwise parameter curves of the plain recordings (volume and pitch RTPCs of the thump and clunk
  objects), the mixing of the engine and exhaust busses.
- **What is written to the repository:** our own prose, equations and numbers (tuning constants of the shipped data
  assets, as for the other specs), code written from the spec, and synthetic tests. No decompiler output, no game
  recording, no bank, no extracted asset, no decoded audio. The scratch tools (`resolve_bike_audio.py`, the pak reader
  and others) live outside the repository.
- **Checked against the game:** the decoded loops of the KTM 450 were measured (autocorrelation) at the pitch their cycle count
  says; the game's own mixer rendered offline (`measure_cars` of `crates/nfsmw`) plays the converted bike with a level of
  -19 dB at every engine speed and a fundamental that follows `rpm / 120` from 3170 to 11000 rpm. Nothing was listened to.
- **Licence:** `MIT OR Apache-2.0` for everything here. The numbers are facts about a product the user owns; the user's
  decoded recordings are theirs and stay in `C:\r6` or their mod folder.
