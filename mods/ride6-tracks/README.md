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

## What a world has, and what it lacks

The ground (slices, kerbs, the runnable tarmac, the terrain) with a picture and a collision surface per triangle, the start on the
first slot of the starting grid. No barriers, buildings, trees, grass, crowd, sky of its own, lap counting or lap times, and no
traffic, police or races: see [the pack format](../../docs/formats/ride6-track-pack.md) and
[the spec](../../docs/specs/second-world.md#what-is-not-built).
