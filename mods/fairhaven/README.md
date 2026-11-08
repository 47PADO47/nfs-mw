# The Fairhaven mod

`fairhaven` lets you drive a car or a bike in **Fairhaven**, the open city of **NFS: Most Wanted (2012)**, inside NFS: Most Wanted
(2005): the 2012 city is read from **your own 2012 install** and is a second world of the game (`--world fairhaven`, or Gameplay
options, Free Roam World). A native mod like [`ride6-tracks`](../ride6-tracks/README.md); the host feature is in
[docs/specs/second-world.md](../../docs/specs/second-world.md), the design and the numbers in
[docs/modding/fairhaven.md](../../docs/modding/fairhaven.md).

**Nothing from the 2012 game is in the repository, in a release, or copied to disk.** Each of the city's 169 streaming units is read
from the install's files by byte ranges and turned into a section file of the game **in memory**, when the camera comes near it.
The mod's own folder holds a few hundred bytes of text for each unit and nothing else. Without the install the mod adds nothing.

## Using it

1. Build the mod and copy it into the game's mods folder: `cargo install-mod fairhaven`.
2. Start the game once. The mod writes a commented `config.toml` next to its `mod.toml`. Put the folder of your 2012 install in it:

   ```toml
   source = 'D:\Need For Speed Most Wanted 2012'   # the folder with HAWAII/ and SHADERS.BNDL
   enabled = true                                    # false: the world is not offered
   max_texture = 512                                 # the longest side of a picture
   ```

   The same keys work under `[mods.fairhaven]` in the game's config file.
3. Start the game again. The mod writes the world's manifests (169 units, a few hundred kilobytes in all, once) and finds where
   the player starts.
4. Choose the world: Gameplay options, **Free Roam World**, then Free Roam; or from the command line:

   ```sh
   nfsmw view-world --world fairhaven --drive BMWM3GTR
   nfsmw view-world --world fairhaven --muted --wait-for-load --screenshot out.png      # a flight, offscreen
   ```

`fairhaven-unit <install> <unit>` makes one unit (or all of them) as the game does and prints what it came to; it is for measuring.

## What the world has, and what it lacks

The ground, roads, buildings and the models placed on them, with their pictures and the collision of the road and the ground, so a
car or a bike can drive on it. The 2012 shading (lightmaps, reflections, lit windows, the blend of the terrain's layers) is not
reproduced: each surface is drawn with its diffuse picture under the game's own lighting. No traffic, police, races, props that
break, trees of the 2012 ground cover, water or minimap yet: see the status in the design page.
