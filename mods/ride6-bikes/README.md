# The RIDE 6 bikes mod

`ride6-bikes` adds the motorcycles of **RIDE 6** (Milestone, Unreal Engine 5.6) to NFS: Most Wanted as playable bikes: the
real models and textures of each bike's stock parts, its real two-wheel physics data, its default livery. It is a native mod
like [`nfsmw-2012-content`](../nfsmw-2012-content/README.md): the game converts the bikes on a background thread from your own
RIDE 6 exports, once.

**Nothing from RIDE 6 is in the repository or in a release.** The models, textures and data are made on your machine from your
own install. Without exports the mod adds nothing and the game is unaffected.

## Using it

1. Build the mod and copy it into the game's mods folder, from the repository folder:

   ```sh
   cargo install-mod ride6-bikes
   ```

   (`cargo install-mods` installs every mod the workspace builds. The command copies `mod.toml`, the library and the mod's
   folders except `src` and `tests`; bikes already generated into `mods/ride6-bikes/files` and `cars` travel with it.)
2. Start the game once. The mod writes a commented `config.toml` next to its `mod.toml` (not when the game's own config file has a
   `[mods.ride6-bikes]` table). Put the folder of your exports in it:

   ```toml
   source = 'C:\r6\bikes'          # one folder per bike: parts.json, glb/, tex/
   physics = 'C:\r6\physdump-all'  # the physics dump; default: a physics, physdump or physdump-all folder inside `source`
   ```

   The exports come from your tools outside the repository: `bikeparts.py` for each bike and the physics dump
   ([ride6-bikes.md](../../docs/modding/ride6-bikes.md), [ride6-bikes-batch.md](../../docs/modding/ride6-bikes-batch.md)).
   Optional keys: `rider` (the exporter's rider folder, with `RiderGP_PhysicsAsset.json` and `RiderBP.json` beside the pack for the fall's ragdoll, default a `rider` folder inside `source`; without it the bikes have no rider), `tyre_tables`, `sound` (the output folder of `sound-bikes`: bikes with tables there get RIDE 6's own engine sound), `pak_key`
   (the key of RIDE 6's pak files for the sound extraction; default: the built-in key, see below), `max_texture`.
3. Start the game again. The mod imports **every bike** of the folder (default stock parts, first livery) in the background
   while the game runs; nothing waits for it. When it is done the bikes are in the game; the log says how many were imported
   and lists each bike that was left out with the reason. Later starts import nothing unless the settings or the exports
   changed (the mod keeps a fingerprint in `.import-stamp`). Then, for example:

   ```sh
   nfsmw view-world --drive KTM450
   ```

   Car ids are derived from the vehicle ids: `ride6-bike-import ids <vehicle id>` prints one, the rule is in
   [ride6-bikes-batch.md](../../docs/modding/ride6-bikes-batch.md#car-ids-and-names).

The converted files are the mod's own `files/` and `cars/` folders (both belong to the converter and are git-ignored). The same
converter is the `ride6-bike-import` command line, for one bike or for experiments.

## Sound: the real engines of every bike

RIDE 6's engines are made by a Crankcase REV synthesiser from recordings stored in Wwise banks inside the game's `.pak` files.
Two commands extract them and turn them into each bike's `[engine_sound]` / `[car_sound]` tables and shared recordings
([docs/modding/ride6-bikes-sound.md](../../docs/modding/ride6-bikes-sound.md)):

```sh
ride6-bike-import pak-sound --tool <pak reader> --usmap <ride6.usmap> --out <banks folder> --resolved <audio data folder>
ride6-bike-import sound-bikes --bikes <source> --physics <physdump> --resolved <audio data folder>     --wwise <banks folder>/ride6/Content --out <sound folder> --vgmstream <vgmstream-cli>
```

then set `sound = '<sound folder>'` in `config.toml`. The pak files' index is encrypted; the key was **supplied by the user** (found
at <https://cs.rin.ru/forum/viewtopic.php?f=10&t=100672>) and **verified by mounting** (1096 banks appear). The mod carries it as its
default (`src/sound/pak_key.rs`); the environment variable `R6_AES` or the `pak_key` setting replaces it. The pak reader and
`vgmstream-cli` are separate programs you build or download; no recording, bank or key-derived file is in the repository.
