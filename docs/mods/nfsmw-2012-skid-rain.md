# The 2012 mod's skid marks, rain splash, rain drops and wet road

`nfsmw-2012-content` offers four more effects of the 2012 game as variants of the 2005 game's own, each a value **NFS Most Wanted 2012**
of a gameplay option that shows only while the mod is loaded and the files exist (and says "Original" until the player picks it). The
profile formats and what the game does with them are in [effect-variants-marks-rain.md](../modding/effect-variants-marks-rain.md); what
the 2012 install holds is in [chameleon-tyre-marks.md](../formats/chameleon-tyre-marks.md) and
[chameleon-rain-water.md](../formats/chameleon-rain-water.md). **Read the second page first if you expect rain: the 2012 game has none.**

| Effect | Option (Gameplay) | Setting (the mod's `config.toml` or `[mods.nfsmw-2012-content]`) | What is written | Made from |
|---|---|---|---|---|
| Skid marks | Skid marks variant | `skid_marks = true` | `files/SKIDMARKS/MW2012/`: up to six 32 x 32 PNGs and `profile.toml` (about 4 KB) | the slip and spin pictures of the tyre mark materials of `SURFACELIST.BNDL` and `GLOBALEFFECTS.BNDL`: decoded |
| Rain splash | Rain splash variant | `rain_splash = true` | `files/RAINSPLASH/MW2012/`: `RING.PNG` (6 KB), `SPRAY.PNG` (70 KB), `profile.toml` | the 2012 water splash ring and spray column pictures; the numbers are chosen |
| Rain drops | Rain drops variant (a replacement set of kind `rain_drops`) | `rain_drops = true` | `files/REPLACE/RAIN_DROPS/RAINDROPPC.png` (3 KB) and `replace/rain-drops-mw2012.toml` | one drop computed from the 2012 round dots; the bend is chosen |
| Wet road | Wet road variant | `wet_road = true` | `files/WETROAD/MW2012/profile.toml` | chosen factors: nothing decoded to read |

All four are converted with the cars (one pass, at the first start or when the settings, the converter or the 2012 files change; the
import stamp covers the four settings, `CONVERSION` 27) and are streamed in the sense of the mod's rule: the install is read **by
byte ranges** (`trees::ranges`: the bundle's table and the resources asked for) and only a few KiB come out. A `false` setting writes
neither the files nor the option value.

## What it costs

Measured on this machine with the real install (`MW2012_GAME_DIR`, `cargo test -p nfsmw-2012-content --lib -- --ignored
marks_assets::tests::the_real rain_assets::tests::the_real --nocapture`):

- skid marks: **129,816 bytes read** of the two bundles' 4,903,585 (2.6 percent), about 4 KB written, under 0.1 s;
- rain pictures: **111,710 bytes read** of `GLOBALEFFECTS.BNDL`, about 80 KB written (the spray picture is most of it);
- nothing is converted when no setting is on, and nothing is read by the game while the option says "Original".

In the game, a skid mark variant costs one textured batch for each picture in use (at most six) and the cutting of a mark at every
repeat of its picture (a few quads for a 3 m mark); the splash costs at most 8 ground queries and 384 quads a frame while it rains.

## What was not heard or seen

`docs/modding/effect-variants-marks-rain.md` lists what is chosen and not decoded. The offscreen screenshots of the marks, the splash
and the lens drops are the ones named in the pull request description; no comparison with the running 2012 game was made.
