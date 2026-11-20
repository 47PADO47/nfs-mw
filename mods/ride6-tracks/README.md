# The RIDE 6 tracks mod

`ride6-tracks` adds the tracks of **RIDE 6** (Milestone, Unreal Engine 5.6) to NFS: Most Wanted as **second worlds** you can
drive a car or a bike on: the ground of the circuit with its kerbs, its pictures, and the surface under each wheel (tarmac,
grass, sand, gravel...). It is a native mod like [`ride6-bikes`](../ride6-bikes/README.md): the game converts the tracks on a
background thread from your own RIDE 6 exports, once. The host feature is in [docs/specs/second-world.md](../../docs/specs/second-world.md).

**Nothing from RIDE 6 is in the repository or in a release.** Without exports the mod adds nothing and the game is unaffected.

## Using it

1. Make the track pack from your RIDE 6 install (needs the exporter's tool, see [tools/ride6-exporter](../../tools/ride6-exporter/README.md)
   and [tools/ride6-tracks](../../tools/ride6-tracks/export.py); about 5 s a track):

   ```sh
   python -I tools/ride6-tracks/export.py Okayama --out C:/r6work/tracks/okayama-pack
   ```

2. Build the mod and copy it into the game's mods folder: `cargo install-mod ride6-tracks`.
3. Start the game once. The mod writes a commented `config.toml` next to its `mod.toml`. Put the folder of your packs in it:

   ```toml
   source = 'C:/r6work/tracks'     # one folder per track: pack.json, ground.r6m, tex/
   max_texture = 512               # the longest side of a ground picture
   brightness = 80                 # percent of the exported pixels (20 to 150), see below
   ```

4. Start the game again. The mod converts every pack of the folder in the background (under a second for Okayama); when it is
   done the world is in the game. Later starts convert nothing unless the settings or the packs changed (`.import-stamp`).
5. Choose the world: Gameplay options, **Free Roam World**, then Free Roam; or from the command line:

   ```sh
   nfsmw view-world --world okayama --drive BMWM3GTR
   nfsmw view-world --world okayama --drive KTM450 --muted --screenshot out.png     # offscreen
   ```

The converted files are the mod's own `files/WORLDS/` and `worlds/` folders (generated and git-ignored). The same converter is
the `ride6-track-import <pack> <mod folder>` command line.

## Size and time

Okayama, 512-pixel pictures, with the barriers, buildings and props: **147 MB** in 66 cells (180 MB before the objects were grouped;
106 MB without them): the objects are one section for each 500 m square of track, holding each model and its pictures once for the
squares' placements, instead of in every ground cell beside a placement. Export from the install 5 to 175 s a track (the objects
and the pictures are most of it), conversion 3 to 10 s, 25 to 530 MB of sections a track. `objects = false` drops the objects.

## Brightness

The game's world shader multiplies a picture by twice its light, so a picture made for a darker lighting model (RIDE 6's) comes out
pale: white buildings and rock near white. `brightness` (percent of the exported pixels, 20 to 150, default 80) scales the colour of
every picture and flat colour when a track is converted (`--brightness` of `ride6-track-import`); change it and restart, the tracks
convert again.

## Lap timing

The mod times the player's laps on the track's spline (`worlds/<id>.lap`, written by the converter): the start line is 25 m ahead of
the first slot of the starting grid, a lap counts after nine tenths of the lap were covered, and the time and the best lap show as a
HUD message ("Lap timing on" when a world opens, "Lap timing started" at the first crossing, "Lap 1  0:08.20  best lap").
The timer is unit-tested on a circle. To **see the messages without driving a lap**, `RIDE6_TRACKS_LAP_DEMO=<metres per second>`
runs a virtual car round the lap from 60 m before the line in the player's place (`lap.rs`, `Demo`):

```sh
RIDE6_TRACKS_LAP_DEMO=450 nfsmw view-world --world okayama --drive BMWM3GTR --muted --wait-for-load --screenshot lap.png --screenshot-delay 9
```

At 450 m/s a lap of Okayama (3,741 m) takes 8.3 s and the screenshot shows "Lap 1  0:08.20  best lap" (verified offscreen). A scripted
run of a real car could not be used: the mod ticks only when the world is advanced and a scripted run advances it in a batch, so the
mod never sees the car on both sides of the line.

## What a world has, and what it lacks

The ground (slices, kerbs, the runnable tarmac, the terrain, pieces of scenery placed with their own transform) with a picture and a
collision surface per triangle, barriers, buildings and props with walls for the solid ones, a horizon and a sea, the racing line, lap
timing, the start on the first slot of the starting grid (or on the first sample of the lap). No minimap in a second world: the
original HUD's disc, heat ring and flame are hidden (`HudState::second_world`), a drawn map is not built. No vegetation, grass,
crowd, traffic, police or races: see [the pack format](../../docs/formats/ride6-track-pack.md) and
[the spec](../../docs/specs/second-world.md#what-is-not-built).
