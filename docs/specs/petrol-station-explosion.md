# Petrol-station explosion

Status: authored object effects and original explosion recording selected from installed data; full gameplay audio comparison remains open.

## What the install says [measured]

Read from `GLOBAL/ATTRIBUTES.BIN` (the game database, through `blackbox-attrib`):

- `effects/exp_gasstation` names the particle group through the `emittergroup` field (key `0xABA86E60`).
- That group, `emittergroup/fxex_gasstation`, has four emitters: `emex_gas_flash1`, `emex_gas_fire2`,
  `emex_gas_dust1` and `emex_gas_base`. Each is an `emitterdata` record read by the same loader as the exhaust
  flames.
- The generic groups `explosions`, `fire` and `smoke` have no emitters in the install, so only the petrol
  station has an authored explosion particle effect.
- The petrol-station breakables are `smackable` records `gaspumpexplosion`, `gasstationroof` and
  `gasstationsupport`. Their event sequences are `gaspumpexplosion_seq`, `gasstationroof_seq` and
  `gasstationsupport_seq`; a placed prop carries the hash of its sequence name as its `event_sequencer`. The
  installed test `installed_gas_pump_explosion_reports_a_petrol_blast` confirms a gas-pump strike starts an
  explosion wave.
- `simsurface/explosion` links only tire-dust groups, so it is not the explosion's effect.
- The dedicated blast recording is `aud_moment_strm/expl_gas_station`, an installed
  `SoundFX` moment-stream selector. Generic debris and break impact events are
  separate from this recording; their empty stitch lists do not justify borrowing
  an unrelated wall impact. See the original moment behavior below.

## What the rewrite does [chosen]

- **Which explosions.** A wave whose source has a petrol-station sequence (the three above) starts a blast at the
  wave's point. Other explosions keep only their physics.
- **Effect.** The blast runs the four `exp_gasstation` emitters at that point for `BLAST_SECONDS` (2.0 s, our
  choice; the original's duration is not in the install). Its particles are drawn as textured billboards, like the
  exhaust flames. Up to four blasts are alive at once. Blasts step on the fixed 60 Hz physics tick.
- **Sound.** Request the installed petrol-station moment recording at the blast
  marker, independently of whether its particle textures loaded. Preserve the
  marker across physics/render coordinate conversion and scene notifications.
  Cache decoded audio during driving setup. Missing data produces a diagnostic,
  with no replacement collision sound.

## Not done

- Ghidra on `speed.exe` does not yet show which `ESpawnExplosion` path starts `exp_gasstation`, or how long the
  original runs it.
- Complete arbitration between explosion, Speedbreaker and other NIS moment streams,
  and matched original gameplay PCM/spatial DSP. The current explosion voice is
  separate from the existing Speedbreaker voice.
- Smoke and fire for other explosions (the generic groups are empty in the install).
- Exact original visual equivalence remains open; lifecycle and rendered remake checks do not establish it.
- Blasts do not reset when the world pauses or a batch runs; they follow the physics tick.

## Original moment behavior [decomp; installed selectors verified by tests]

The original moment map associates gas-station marker type1 with
`aud_moment_strm/expl_gas_station`. Its collection supplies `stream`, `param`,
`VolSlot`, `IsPositioned`, and priority. The moment recipient passes the two
selector masks to `SoundFX`, whose index event selects the recording from
`SOUND/STREAMS/NISAudio.idx` and `NISAudio.big`. Reuse the validated two-field
`moment_take` selector and bank-specific sample alignment; never guess a bank
entry or use a collision stitch as a substitute.

The main MomentStrm mixer object6 supplies the selected volume slot and azimuth
slot0. Publish blast-to-player and blast-to-camera planar distances and azimuths
to main position controller2, including its valid flag, and update while playing.
Positioned recordings use authored azimuth; unpositioned ones remain centered.
Quantize gain through Q15 shifted right8 into the original 0..127 stream range.
Playback rate remains1. A current identical moment suppresses re-triggering for
2 seconds; a later accepted blast replaces the existing explosion voice. Stop
when its scene leaves and let natural sample completion clear the current voice.
Suppress/dismiss a blast farther than120m from the player, matching the original
moment producer's range cleanup. Do not restart an ended recording each frame.

Tests must decode the uniquely selected installed recording, produce audible
replay PCM, retain the actual event point, exercise authored distance/azimuth,
suppress chained same-moment restarts, and stop at scene teardown. These checks
establish integration behavior; they do not establish original output identity.
