# Petrol-station explosion

Status: effect and sound built as a first slice; not compared with the original on screen.

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
- The sound is named by event, not by stitch: `audioimpact/debris` and `audioimpact/smackable_break` are
  `SMOKABLE` with an `EVENT` description and no stitch levels. Their event-to-bank map lives in the `.csi` files,
  which are not decoded yet.

## What the rewrite does [chosen]

- **Which explosions.** A wave whose source has a petrol-station sequence (the three above) starts a blast at the
  wave's point. Other explosions keep only their physics.
- **Effect.** The blast runs the four `exp_gasstation` emitters at that point for `BLAST_SECONDS` (2.0 s, our
  choice; the original's duration is not in the install). Its particles are drawn as textured billboards, like the
  exhaust flames. Up to four blasts are alive at once. Blasts step on the fixed 60 Hz physics tick.
- **Sound (stand-in).** Each blast plays the loudest stitch of the heaviest `WALL` impact collection, from
  `Stich_Collision_MB.abk`. The explosion's own sound is not identified yet, so this is a stand-in.

## Not done

- Ghidra on `speed.exe` does not yet show which `ESpawnExplosion` path starts `exp_gasstation`, or how long the
  original runs it.
- The explosion sound: decode the `.csi` event map, then play the real `smackable_break`/`debris` sound.
- Smoke and fire for other explosions (the generic groups are empty in the install).
- Visual check on screen: the textures and sprite path are unit-tested only through the blast lifecycle.
- Blasts do not reset when the world pauses or a batch runs; they follow the physics tick.
