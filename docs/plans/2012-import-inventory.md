# Inventory: what of NFS: Most Wanted (2012) can come into the 2005 game

The user's request: import as much of the 2012 game as possible into this game, everything toggleable, and, best of all,
**streamed from the original files** (read at load time, nothing written); where that is impossible, a very small converter
that is fast and small on disk. This page lists everything the 2012 install holds, what the mod
[`nfsmw-2012-content`](../../mods/nfsmw-2012-content/README.md) already takes, what is missing, and for each missing item
whether the 2005 game has a slot for it, what it costs, and whether it can be streamed. The order of work that follows is at the
end. Evidence tags as in the [docs README](../README.md#evidence-tags); format docs: [chameleon-bundle.md](../formats/chameleon-bundle.md).

Status words: **DONE** (in the mod today), **DOABLE NOW** (data is understood and the 2005 host has a slot), **NEEDS HOST FEATURE**
(data is understood, the host has no slot yet; the feature is named), **NOT FEASIBLE** (no data, no decoder, or no sense in the
2005 game).

How it was measured: a Python scratch reader of the `bnd2` tables of all 2,021 bundles (resource types per folder, sizes), the
readers of `libs/chameleon-*` for what they decode, the song and string tables read with the `LocalisedText` layout
([chameleon-language.md](../formats/chameleon-language.md)), and contact sheets of the textures (BC1/BC3 decoded with a scratch
script) looked at by eye. Nothing from the install is in the repository. **Nothing here was compiled or run** (the sizes of the
mod's output are of the files an earlier conversion left in `%APPDATA%\nfsmw\config\mods\nfsmw-2012-content`).

## 1. The install (6.3 GB, v1.4.0.0)

Names of resources are hashes; only the 85 `HAWAII/MAPS` bundles carry debug names, so what a resource *is* comes from its type,
its shader technique, its imports and a look at its pixels. Counts are resources, sizes are bundle files.

| Path | Size | What it holds (by resource types) |
|---|---|---|
| `VEHICLES/VEH_n_HI`, `_LO`, `_TR` | 349 + 145 + 10 MB | 93 + 93 + 11 cars: renderables, models, materials, textures, skeletons, animations, collision, sound graphs (`Ginsu` 731, `Wave` 3,908), handling and damage objects |
| `VEHICLES/VEHICLETEX`, `VEHICLELIST`, `WAVES`, `TRAFFICATTRIBS`, `RUMBLELIST` | 14 MB | 53 shared car textures, the vehicle and manufacturer list, 126 shared waves (pops), traffic attributes, controller rumble |
| `HAWAII/TRK_UNIT*` (169) | 3,324 MB | the city in streaming units: 10,775 distinct renderables (393 MB of geometry), 3,610 models, 2,842 distinct textures (2.7 GB), `PropObject` 112, `WorldObject` 270, `GroundcoverCollection` 400, collision soups |
| `HAWAII/DISTRICT_*` (7), `ARENA_*` (8) | 227 + 327 MB | district and arena geometry and textures (714 and 1,349 distinct textures) |
| `HAWAII/GLOBALRESOURCES` | 97 MB | the shared road library: road paint, arrows, manhole covers, grates, signs (STOP, speed limit, tow zone, police), traffic lights, asphalt, concrete, grass, bark, 261 textures with their normal maps (looked at) |
| `HAWAII/ENVIRONMENT` | 5 MB | sky domes (clouds at several times of day), a moon, cloud noise, 13 colour cubes, 220 environment objects |
| `HAWAII/SURFACELIST`, `SURFACESOUNDS`, `SHAREDWAVES`, `AMBIENCES`, `TRAFFIC`, `TRIGGERS`, `AI`, `PVS` | 11 MB | surface physics and effect records per road surface, 281 surface sounds, 83 shared waves, ambience zones, traffic lanes, triggers |
| `GLOBALEFFECTS` | 4.8 MB | 214 `BearEffect` particle effects, 129 textures |
| `POSTFX`, `GLOBALCONFIG`, `PHYSICS/*`, `CAMERAS`, `WORLDLIST` | 0.3 MB | post-effect states and 8 colour cubes, global config, physics tables, 45 cutscene camera takes |
| `SHADERS*.BNDL` | 5.4 MB | 373 shaders (three quality variants) |
| `CHARACTERS/ALL_CHARS` | 4 MB | 3 skinned characters (rest pose), 1 skeleton |
| `UI/SCREENS2` (94), `LAYOUTS` (43), `FONTS` (17), `IMAGES/*`, `EASYGUIDE`, `UICONFIG` | 38 + 1.4 + 2.4 + 28 MB | the front end as renderables plus textures: icons, panels, flags, backgrounds, photos; 17 fonts; 743 streamed pictures (469 are 512x256, most of the rest logos and photos) |
| `UI/SONGS` | **259 MB** | **58 full-length songs** (`.SPS`, stereo, 4 to 6 minutes, 44.1 or 48 kHz) and `SONGS.BNDL` with 62 song objects |
| `SOUND/STREAMS` (56 `.SPS`), `SOUND/*.BNDL` | 108 + 4 MB | 6-channel 30 and 60 second ambience loops, 18 stereo 1 to 2 minute cues, the sound graphs and 265 waves of the game-wide sound (`GLOBAL`, `METALCAR`, `SPEEDSOUNDS`) |
| `GAMELOGIC/*` | 4 MB | helicopter sound set (8 waves: loop, pass-by, wash), 11 `WEAPONS` bundles (hash names, a few textures and waves each), game modes |
| `EN_US/*`, `UI/SEQUENCEITEMS`, `UI/SEQUENCES/STREAMS` | 282 + 9 + 144 MB | speech (`SWC0` 231 MB), 217 feedback groups (animations, renderables, textures), localised speech event streams |
| `UI/MOVIES` | 1.2 GB | 63 `.VP6` videos (On2 VP6 in a container) |
| `scripts/*.asi` | | third-party plugins of the user (FPS unlock, HD effects): not game data |

## 2. What the 2005 game can take today

The slots a mod has ([api-content.md](../modding/api-content.md), [effect-variants.md](../modding/effect-variants.md)):

| Slot | What it takes | Reads through |
|---|---|---|
| Car manifest + `CARS/<ID>/` files | a whole car (solids, textures, bounds, stats, sounds, paints, parts, lamps, cameras) | `GameDir` layer: files read as bytes (`read_unwrapped`), WAVs and `.GIN` by path |
| Effect variants (nitrous, tire smoke, collision sparks, restored sparks, wind) | a profile (`profile.toml`) and pictures or WAVs under `files/<KIND>/<ID>/`; one option row per kind | `variant_profile` through the layers, once per chosen value |
| Trees variant (`model_swap`, the trees agent's branch) | a pack of solids and textures that replace solids of the world **by name** as sections stream in | `Streamer::set_rewrite` |
| Brand logo, racer names, music source, scenery pack, interior | manifests and files of the mod | `manufacturers/`, `[racers]`, `[music]` + `music/`, `scenery/`, `interiors/` |
| Extra camera views | `bb.camera` 1.1 | the mod steps its model per frame |
| Overlay file replacement | any install file, **whole**, by path, in the install's own formats | `GameDir` layers |
| Mod options | `bb.settings`: toggles and lists on the Gameplay page, texts translated ([translations.md](../modding/translations.md)) | `settings.add_option` |

Not available: replacing a texture or a sound **inside** a streamed 2005 container by name (only whole files, and only in the
2005 formats), a HUD skin (`docs/formats/custom-hud-skin.md` exists only on the unmerged `feat/custom-hud-asi`
branch), a font registry (planned in api-content.md), menu-sound or ambience slots, a rain or skid-mark variant kind.

## 3. The inventory

Cost is the code (lines, rough), the run time and the disk the item would add. **Stream** says whether it can be read from the
original file at load time with no file written (S), needs a small converter (C), or is a host feature that decides it (H).

### 3.1 Cars (all in the mod)

| Item | 2012 data | Status | Notes |
|---|---|---|---|
| Car bodies, interiors, glass, 3 or 4 levels of detail | `VEH_n_HI`, `_LO` | DONE | 67 cars (93 bundles: 20 liveries, 6 second ids); C level from `_LO` |
| Wheels (tyre, rim, disc, caliper) | `VEH_n_HI` | DONE | as one solid per car |
| Textures | `VEH_n_HI`, `VEHICLETEX` | DONE | BC1/BC3 cut to `max_texture_size` (1024) |
| Paints, two-colour packs, liveries | vehicle list, paint packs, bundle textures | DONE | 62 two-colour liveries, 5 cars with bundle liveries |
| Stats, nitrous, drift numbers, collision bounds | Genesys handling objects, poly soups | DONE | fitted, partly unconfirmed |
| Engine loops, shifts, turbo, backfire, pops, ignition, shared hits and scrapes | `Ginsu`, `Wave`, `WAVES`, `METALCAR` | DONE | WAVs written (174 MB) |
| Hinged parts, active aero, steering wheel, pieces | skeleton, damage clips | DONE | up to 25 parts a car |
| Lamps, flares, siren light bars, exhaust markers | corona objects, lamp records | DONE | `[[lamp_flare]]`, `[[light_bar]]` |
| Driver, poses, hands on the wheel, look back | `ALL_CHARS`, driver clips | DONE | 51 % of the geometry files (section 5) |
| Plates, 469 plate designs | plate meshes, designs | DONE | [nfsmw-2012-plates.md](../mods/nfsmw-2012-plates.md) |
| Brand logos | `UI/IMAGES/STREAMED` | DONE | 32 of 33 brands |
| Cameras (chase, bonnet, bumper), drift nitrous and "DRIFTING", engine auto-stop | camera objects, drift objects | DONE | |
| Traffic cars (11), police cars (5) | `_TR` bundles, police marks | DONE | |
| Driver and car as a **second set of vinyls** | none in the install | NOT FEASIBLE | the 2012 game has no vinyl editor data |
| Spoilers, body kits, hoods as shop parts | parts are skinned into the body | NOT FEASIBLE | not separable; static spoilers are in the body, active ones are `Spoiler` parts |
| Each car's wheel as a rim **choice** on other cars | the wheel solid of the 93 cars | DOABLE NOW (limited) | the manifest's `rims` list names solids of the car's own `GEOMETRY.BIN`: 0.14 MB per rim and car, so a few rims only until geometry is streamed; the wheel carries disc and caliper |
| Car pictures for the car select | 469 pictures 512x256 in `UI/IMAGES/STREAMED` | NEEDS HOST FEATURE | which picture is which car is not decoded; the game has no per-mod-car picture slot (the car list draws the 3D car) |
| Cutscene and story cameras, feedback shakes | `CAMERAS.BNDL` 45 takes, `EN_US/FEEDBACKGROUPS` | NOT FEASIBLE | scripted for the 2012 world |

### 3.2 Sound and music

| Item | 2012 data | 2005 slot | Status | Cost | Stream |
|---|---|---|---|---|---|
| **Soundtrack** | `UI/SONGS`: 58 `.SPS` of EL31 (Layer III) audio, titles and artists in the language table next to the song object's key | custom music library (`[music]`, playlists with captions) | **DONE** (key `music`, gameplay option *2012 soundtrack*): 43 titled songs | host: `.sps` decode in `custom/decode.rs` (25 lines, `ea_audio::sps::to_mp3`); mod: `soundtrack.rs` (about 230 lines), a playlist of about 5 KB written at every start in a few ms | **S** (the playlist names the install's files; the host decodes them while they play; nothing converted) |
| Ambience loops, city beds | `SOUND/STREAMS` 6-channel loops (`AMBIENCES`, `REVERBROADDATA`) | none (the 2005 ambience is in its sound banks) | NEEDS HOST FEATURE | an `ambience` variant kind like the wind (looping layers) | S once the kind exists |
| Menu and UI sounds | `UI/SCREENS2` waves (617), `SEQUENCEITEMS` (131), `SOUND/GLOBAL` (141) | none (2005 menu sounds are events in its banks) | NEEDS HOST FEATURE | an event-to-WAV map in the host | C (stream decode to WAV on first use) |
| Horn | graph `Player Vehicle Horn` in every car and in `SOUND/GLOBAL` | the car's `[car_sound]` has no horn list | NEEDS HOST FEATURE | one list key, one host call | C |
| Crash, scrape, body hits | `METALCAR` graphs | DONE (shared) | DONE | | |
| Skid and tyre squeal | `Genesys.Gen.TyreSoundParams` (lateral, longitudinal, braking) in `SURFACELIST` | none | NEEDS HOST FEATURE | the 2005 squeal comes from its banks; a tyre-sound variant kind | C |
| Helicopter | `HELICOPTERSOUNDS`: loop, pass-by, wash (8 waves, 81 KB) | the 2005 helicopter plays its bank sounds | NEEDS HOST FEATURE | a variant kind with three WAVs | C (tiny) |
| Police sirens | the siren is a state of the 2012 sound graphs; no separate siren sample was found | the cop car's siren is a bank sound | NOT FEASIBLE until the graph is decoded | | |
| Speech (cop radio, navigation) | `EN_US/SWC0` (231 MB), event streams | the 2005 speech tables | NOT FEASIBLE | other game, other script and other event names | |
| Rumble | `RUMBLELIST` | controller rumble variants | NOT FEASIBLE | unrelated events | |

### 3.3 Effects and look

| Item | 2012 data | 2005 slot | Status | Cost | Stream |
|---|---|---|---|---|---|
| Nitrous sound and screen, tyre smoke, collision sparks, restored sparks, wind | `GAMELOGIC/HL_*`, `POSTFX`, `SURFACELIST`, `GLOBALEFFECTS`, `SOUND/GLOBAL` | variants | DONE | 7 MB of files | C |
| Trees | `HAWAII/*` tree models and textures | `trees_variant` | DONE (other agent) | | |
| **Collision debris and glass** | `GLOBALEFFECTS` (214 effects; the collision ones are read for the sparks, the glass and debris ones are not) | none | NEEDS HOST FEATURE | a variant kind for "impact debris" | C |
| **Skid marks and tyre marks** | `TyreVfxBehaviour.SkidMarkParams` in car bundles; `SurfaceVfx.TyreMarksMaterials` in `SURFACELIST` | the 2005 skid mark decals (texture, width, fade) | NEEDS HOST FEATURE | a variant kind with one picture and four numbers per surface family | C |
| Rain drops, rain splash, wet road | rain effects among the 214; wet-road is a shader parameter | the host's rain ([rain.md](../rain.md)) uses the install's textures | NEEDS HOST FEATURE | a rain variant kind (picture and counts); the wet road has no data in a form that is read | C |
| Headlight beams, lamp flares of cars | coronas | DONE (flares); beams: NOT FEASIBLE (not decoded) | | | |
| Street-lamp coronas | `Genesys.Gen.Corona` on `PropObject`s | the world's lamps have 2005 flare data | NEEDS HOST FEATURE | with the props below | |
| Sky, clouds, moon | `ENVIRONMENT`: cloud domes at several times, moon, noise | the host's sky (`enhanced-sky.md`) draws 2005 textures | NEEDS HOST FEATURE | a texture swap by name (below) or a sky variant kind | C |
| Colour grading by time of day and weather | `ENVIRONMENT` 13 colour cubes, `POSTFX` 8 cubes (the nitrous cube is read) | the host's post pass | NEEDS HOST FEATURE | a 3D LUT stage and a `grading` kind | C (cubes are 16-32 cubed RGB) |
| Sun, fog, ambient colours | 220 environment objects (unnamed hash fields) | the host's time-of-day tables | NOT FEASIBLE | fields not decoded | |
| Post effects (vignette, bloom) of heat and speed | `POSTFX` states bound to live values | the nitrous screen uses them | DONE for nitrous; others NEEDS HOST FEATURE | | |
| Camera shake, moment cameras | feedback shake groups in `EN_US`; the 2012 chase camera model is used | the 2005 camera has its own moment shots | NOT FEASIBLE | no gameplay moment camera exists in the data ([chameleon-camera.md](../formats/chameleon-camera.md)) | |

### 3.4 The world

| Item | 2012 data | 2005 slot | Status | Cost | Stream |
|---|---|---|---|---|---|
| **Road, ground and sign textures** | `GLOBALRESOURCES`: asphalt, concrete, grass, arrows, STOP and speed-limit signs, traffic lights, manhole covers, grates | the 2005 textures are in streamed `.BUN` sections by name hash | NEEDS HOST FEATURE | a **texture swap** next to the trees' model swap (`Streamer::set_rewrite` already sees every section before the textures are decoded); a profile of `name -> picture` pairs, about 300 lines in the host, 400 in the mod | C: a swapped picture is a cut BC1/BC3 of 0.1 to 1 MB; the pack is a few tens of MB. Streaming the 2012 bundles directly is S once the swap reads a `Texture` resource itself |
| Street props and lamps | 112 `PropObject`, models in `TRK_UNIT*` and `GLOBALRESOURCES` | the same model swap as the trees, by solid name | DOABLE after the trees PR | which 2005 solid matches which 2012 prop is a manual table (names are hashes); 150 lines of mod code, no host code | C |
| Billboards, signs as models | models in `TRK_UNIT*` | same swap | DONE for the generic road signs and the traffic lights ([3.4a](#34a-other-props-of-the-world-what-the-trees-pattern-could-carry-assessment-november-2026), [chameleon-signs.md](../formats/chameleon-signs.md)); billboards and shop signs not | the manual match is the table of `signs/table.rs` | C |
| Ground cover (grass, bushes) | `GroundcoverCollection` 400 | the 2005 ground cover is baked in sections | NOT FEASIBLE | | |
| The city (districts, roads, buildings) | `TRK_UNIT*`, `DISTRICT_*` (3.5 GB) | a map pack ([api-content.md](../modding/api-content-worlds.md#scenery-and-map-packs)) | NOT FEASIBLE | a whole city needs roads, zones and AI data in the 2005 formats, and a second conversion of 3.5 GB | |
| Traffic lights as props | models plus textures | swap | DOABLE after the trees PR | | |

### 3.4a Other props of the world: what the trees pattern could carry (assessment, November 2026)

The trees variant ([effect-variants.md](../modding/effect-variants.md#profile-format-of-the-trees)) swaps solids of the 2005 world by
name for models of the 2012 world and fits them to the old footprint, so any prop that is a solid of the 2005 world can be
replaced the same way. Measured for this list: the 2005 solids by name (a scratch report of `STREAML2RA.BUN`, 16,349 distinct names: 6,700 terrain
`TRN`, 4,299 `SHD` shadows, 1,957 buildings `XB`, 1,163 objects `XO`, 498 signs `XS`, 474 walls `XW`, 147 trees `XT`; a world-space
`_DEINST` copy counts under the name it was made of) and the 2012 install (a Python reader of the `bnd2` tables of all
`TRK_UNIT*` bundles and `GLOBALRESOURCES`): **112 distinct `PropObject`, 109 of which resolve to a model, with a bounding
radius of 0.28 to 3.3 m (35 under 1 m, 61 of 1 to 2 m, 10 of 2 to 3 m, 3 over 3 m)** and **no lamp-post sized object among them**: the
2012 street furniture that is taller than a car is part of the district geometry, not a prop. The 2012 resources have no names
(only hashes), so which model is a bench or a hydrant is read off its pictures, by hand, as the 40 tree models were found by their shader.

| Class | What the 2005 world has (names; distinct names / solids) | What the 2012 install has | Look gain | Risk | Verdict |
|---|---|---|---|---|---|
| **Bushes, hedges** | `XT_CT_BUSH_1B_JG_00` (176 solids), `XT_BUSH_FOREST_1A/2A_JG_00` (125), `XT_GC_BUSH_1B_00` (47), `XT_CT_BUSHMULLB`, `XT_BUSHREDA` (24 names / 478 solids; four of them are already swapped) | bush and shrub cards on the `Tree_*` shader (the round bush is one of the eleven species; thin bushes, shrub strips and a bare shrub have textures but are not converted) | medium: low foliage is most of what the eye sees from the road | most of these 2005 solids are **effect 0** patches up to 100 m wide: with at most 32 copies a solid, a patch of bushes would become a few bushes; needs denser 2012 strips first | **best next step**, as families of the existing trees pack; not done: the visible result has to be looked at species by species |
| **Grass** | `XT_GRASSPATCH_1B_00` (59, leaf cards), `XT_BEACHGRASS` (69), `XO_GRASSCLUMPCT` (43), `XT_GRASSA/B/C` (45): 11 names / 221 solids | **no grass model**: grass is the `Groundcover` shader (400 `GroundcoverCollection` of `TRK_UNIT*`, drawn from an atlas by position) | high | none to the 2005 data, but nothing to put there | **not feasible** as a model swap; a picture swap of the grass atlas needs the texture swap of section 3.4 |
| **Street lamps** | `XO_StreetLightCb/C/G/M`, `XO_IRONLAMPSTANDINGB`, `XO_PARKLAMPA`, `XO_LIGHTPOLEA`, floodlights, cage and canopy lights (120 names / 135 solids) | district geometry, not props (see above); models exist per unit but are not identified | medium | the 2005 lamp has flares, a night glow and a physics body tied to the solid's name and markers (`docs/specs/scenery-sway.md`, the lamp tests of `residency`): a swapped model loses the markers the swap does not carry | **not recommended** until a swap can keep the markers |
| **Traffic lights** | `XO_TRAFFICLIGHTA/B` (poles with an arm, 104 and 99 placed), `C/D` (the signal heads, 53 and 287 placed; the 2005 head is a flat slab with a picture of three lenses and one red flare marker) and the lane signs `XS_TRAFFICSIGNALARROW/CROSS_JK` | the mast arm `0xBB97A2C5` of `GLOBALRESOURCES` (pole, arm and two heads in one mesh) and the signal head of `0xF6135E80` (`TRK_UNIT182`) | **DONE** (this branch): the arm with its heads cut away replaces `A`/`B` (each stretched to the old box), the head hung with its middle lens at the lamp point replaces `C`/`D` (14 solids), as the replacement sets of kind `traffic_lights` | the lit lamp of the `traffic_lights` option is drawn by the host at a point computed from the solid's name; the point was wrong for `C`/`D` (4.5 m above a head that hangs below its origin): now on the head, moving up for red and down for green by the lens pitch; a red flare marker is kept. The lane signs were **not** done (a two-frame animated texture cannot be a mesh of one picture) | [chameleon-signs.md](../formats/chameleon-signs.md) |
| **Road and shop signs** | `XS_*` (538 names): the regulatory and warning signs on poles, and the Rockport shop and highway signs | the street signs of the 2012 world are ordinary one-mesh models (plates on a pole), found by looking: stop (clean and weathered), speed limit 45 and 55, do not enter, tow zone. `Sign_RetroReflective` (5 materials) is used only by the signage of 12 building models | **DONE** for the generic signs the 2012 game has (8 solids: both stop signs, speed 45 and 55, do not enter, no parking), as a replacement set of kind `road_signs`, each model turned to face -y and fitted to the old box; **not** the speed limits 35, 50, 60 and 75 (a wrong number would be worse), the warning signs (the 2012 diamonds take their colour from the instance), and the shop, billboard and highway signs (names and texts of another city: a texture swap, see the road textures branch) | pictures are carried with the models (named `MW12_S<id>`) | [chameleon-signs.md](../formats/chameleon-signs.md) |
| **Billboards** | `XS_BILLBOARDA/D`, `XO_IRONBILLBOARD*` (11 / 19) | `World_SmashedBillboard_*` (the 2012 adverts) | low | adverts of the 2012 game (brands, licensing) in the 2005 city | no |
| **Benches, bins, boxes, tables, shelters** | `PARKBENCHA/B`, `TRASHCAN`, `DUMPSTER*`, `NEWSPAPERBOX*`, `PICNICTABLE`, `PATIOTABLE`, `BUSSHELTER*`, `PHONEB` (86 / 90) | among the 109 `PropObject` (0.3 to 3.3 m), identity unknown | low to medium (seen at 20 m) | these are **physics props**: the 2005 game gives each its bounds and mass by name (`PropCatalog`), so the model must be the same size and its origin where the old one's was; the 2012 origins are not measured | possible one by one, after a contact sheet of the 109 models |
| **Barriers, cones, barrels, sawhorses, tire piles** | `CRASHBARREL*`, `SAWHORSE`, `TIREPILE*`, `XW_*BARRIER*`, `MEDIANPOLE` (100 / 176) | props of the same 109 | low | as above; the Jersey barriers (`XW_`) are walls with collision | no |
| **Fences, rails, gates** | `XW_*FENCE*`, rails, gates (300 / 630) | `Fence_GreyScale_Doublesided` (17 materials, grey-scale tinted) | low | fences are long walls with collision; the swap fits a row of copies, which a fence is not | no |
| **Flags** | `XO_FLAG*`, `XO_TRIFLAG`, `XO_ROUNDFLAG` (14 / 46) | `Flag_Opaque_Doublesided` (4 materials) and `Flag_UVFlip_Opaque_Doublesided` | low | the 2005 flags are animated by the scenery sway; the 2012 ones by their own shader technique (`Flag_*`), which the 2005 engine has no equivalent of **[unconfirmed]** | no |
| **Rocks, boulders** | `XO_ROCKSMALL A/B/C`, `XO_BOULDERA` (15 / 43) | district rocks, not identified | low | 2005 rocks are effect 3 (normal mapped) | no |
| **Cars as props** | `MINIVAN_*`, `CARBODYA/B` fragments (14) | the 2012 cars (already imported, 93) | low | these are debris fragments of a smashable car, not cars | no |

**Why nothing of this was built with the trees fix.** The cheap and safe part is bushes, and it needs a look at each species
before its solids are listed (the 2005 bush patches are big and dense); a second, separate option row "Props" needs more than
the trees pattern gives: the world holds **one** pack in effect (`model_swap::set_active`), so a second kind needs a second slot
and the residency's texture upload and refresh (`residency/trees.rs`) to follow every pack, a `VariantKind` and `EffectKind`
(`variants.rs`, `bridge.rs`) with its catalogue messages in every language, and a mod key and a value for the mod's `effect_variants`. About 300 lines of
host code and the same again in tests, for a variant whose first content (bushes) is not looked at yet. The first step is to add
the bush families to `trees/stock.rs` (they ride the existing "Trees" option and the mod's `trees` key) and look at them; a "Props"
option is worth its cost only once there is a second class with content.

### 3.5 Interface

| Item | 2012 data | 2005 slot | Status | Cost | Stream |
|---|---|---|---|---|---|
| **Speed, nitrous and gauge art** | the 2012 HUD is a speed number and bars drawn from small panels in `UI/SCREENS2`; **no analogue dial was found** among the 208 distinct large textures looked at | the CustomHud skin format (unmerged branch `feat/custom-hud-asi`: `[SPEEDOMETER_DIGITAL]` digits atlas, bars, `NOS_LINIAR`) | NEEDS HOST FEATURE | the digits and bar pictures would be cut into a DDS atlas and an ini; waits for the skin reader of that branch | C |
| Fonts | `UI/FONTS` 17 bundles, `Font` resources with glyph textures | the font registry is planned, not built | NEEDS HOST FEATURE | | C |
| Icons, flags, panels, backgrounds, logos | `UI/SCREENS2`, `IMAGES` | the front end's own pictures by name | NEEDS HOST FEATURE | the same texture swap | C |
| Language texts | `UI/LANGUAGE` 14 languages | none needed | NOT FEASIBLE | the texts are the 2012 game's | |
| Movies | `UI/MOVIES` 63 VP6 videos, 1.2 GB | the 2005 video player | NOT FEASIBLE | no VP6 decoder in the repository | |

### 3.6 People and animation

| Item | 2012 data | Status | Notes |
|---|---|---|---|
| Driver models (3 characters, the police officer), hands | `ALL_CHARS` | DONE | |
| Cop and civilian models outside cars | the same 3 characters | NOT FEASIBLE | the 2005 game has none to replace; busts are cutscenes |
| Animations (1,297) | driver clips, door and hood clips used; the rest belong to scenes of the 2012 game | DONE for what a car uses | |
| Helicopter model | not identified among the 112 props | NOT FEASIBLE until found | |
| Weapons bundles (11) | hash names, a few textures and waves each | NOT FEASIBLE | no 2005 slot (pursuit tech is not in the game) |

## 4. Streaming: what the host reads, and how a 2012 file could be read in place

**Observation.** Most of what the host takes from a mod is read as bytes: cars (`read_unwrapped` of `GEOMETRY.BIN`,
`TEXTURES.BIN`, `BOUNDS.BIN`), variant profiles and their pictures, plates. The `GameDir` layer stack
([vfs.rs](../../libs/game-install/src/vfs.rs)) is path based: a layer is a folder indexed once, and `resolve` returns a path.
Engine WAVs, plate designs and custom music are opened by path.

**What can be streamed with no new mechanism.** Anything the host decodes itself from a path that already exists. The soundtrack
is the one case: the playlist names the original `.SPS` files, and the host decodes them with `ea-audio` while they play
(section 6, step 1). Nothing is written but the playlist (a few KB).

**What needs a conversion.** Everything that changes shape: cars (the 2005 solid format, axes, fitted wheels), textures (cut to
size, new mip chain), sounds (decoded to WAV, one loop per car). These are today converted once, in the background, and
written to the mod's folder.

**A generic mechanism, proposed: the virtual layer.** `bb.content` 1.1 would let a mod register a *virtual layer*: a list of game
paths with sizes and one callback `read(path) -> bytes`. `GameDir` keeps a layer entry as either a real path or a virtual one;
`exists`, `provider`, `size`, `read`, `files_in` and `subdirectories` work for both; `resolve` and `open_file` (which hand out
a path or a file) do not, so a virtual file must be read as bytes. The mod's callback converts on demand and keeps the last few
results in memory. What it needs from the host: about 150 lines in `vfs.rs`, an ABI table entry (`add_virtual_layer`) in
`abi/iface/content.rs`, `host/table/content.rs`, `sdk/content.rs` and the host backend. What it gives: the cars' `GEOMETRY.BIN`
and `TEXTURES.BIN` (80 % of the mod's disk, section 5) are never written; each car is converted from its 2012 bundle when the game first
opens it. Limits: a conversion of one car costs the car's share of the one second the whole run takes on six cores (the
README's figure; **not re-measured here**), so the first open of a car in the garage waits about that much; the memo of
[`files.rs`](../../crates/nfsmw-data/src/files.rs) keys a file by path, size and modification time, which a virtual file has none
of, so it would be keyed by path alone.

## 5. The existing car import, measured

The mod folder of the last conversion (78 manifests: 67 cars, 11 traffic cars; the 5 police cars were added later), in MB:

| Part | Size | Share of 750 MB |
|---|---|---|
| `files/CARS/*/GEOMETRY.BIN` (78) | 510 | 68 % |
| `files/CARS/*/TEXTURES.BIN` (78) | 82 | 11 % |
| `files/SOUND/MODENGINE` (67 cars: loops, shifts, turbo, pops as WAV) | 169 | 22 % |
| `files/SOUND/MODSHARED`, `PLATES`, `NITROUS`, `WIND`, `UI`, `TIRESMOKE`, `SPARKS` | 15 | 2 % |
| `cars/*.toml` (manifests), `BOUNDS.BIN`, `CAMERAS.TOML` | 1.9 | |

Inside the geometry files (a scratch parse of the chunk tree of all 78):

| Content | MB | Share |
|---|---|---|
| Driver figure and its steering and reverse poses, every level | 260 | 51 % |
| Body levels A to D | 139 | 27 % |
| Hinged parts, steering wheel, pieces | 83 | 16 % |
| Wheels | 27 | 5 % |
| of which `_B` solids that are copies of the `_A` solid | 234 | 46 % |

So **half of the disk is the driver** (the same 3 characters in every car, moved by the seat offset), and **46 % is the `_B`
duplicate** that the host needs because it picks levels A and B by name. The textures are 11 %: the shared car textures
(`VEHICLETEX`, 53 maps) are cut again for every car. The time is the README's "about a second on six cores once the files are in the cache"
and a few more seconds when they are not; it is **not re-measured**.

Leaner paths, by what they save and what they cost:

| Path | Saves | Cost | Where |
|---|---|---|---|
| Virtual layer (section 4) for `GEOMETRY.BIN` and `TEXTURES.BIN` | about 590 MB (79 %) | host about 150 lines + ABI entry, mod about 200 lines; first open of a car waits for its conversion | host + mod |
| Key `driver_poses = false` (**done**, default `true`) | 3.5 MB a car (about 230 MB for 67 cars, 31 %) | the driver stays seated and does not follow the wheel | mod only, 4 lines |
| One `_A` solid, `_B` as an alias | 234 MB (31 %) | the game's car model must accept two names for one solid | host (`blackbox-solid` reader) |
| Shared driver pack (4 characters once, the car gives only the seat) | most of the 260 MB | the host loads a shared driver pack | host |
| WAVs from the bundles' `SPS` at load | 169 MB (22 %) | the engine sound loader opens by path today; it would need bytes | host (`audio/car/own_engine.rs`) |
| `max_texture_size = 512` | about 55 MB of the textures | blurrier textures | config only (exists) |

## 6. Phase 2: what was done, and what was not

Done in this branch (each a key of the mod's config and a row of its README; nothing was compiled or run):

1. **Soundtrack, streamed.** The custom music player plays `.SPS` files (repacked to MP3 in memory); the mod writes a playlist of the 43
   titled songs of `UI/SONGS/`. Key `music` (default `true`: the option is offered), gameplay option **2012 soundtrack** (off by default;
   `bb.settings` toggle, label from `lang/en/nfsmw-2012-content.ftl`; a change applies at the next start). Disk: 5 KB. Time: a few
   milliseconds at start for the playlist; a song takes a few tens of milliseconds more to start than an MP3 (the whole stream is repacked first).
2. **Lean car files.** Measured above; key `driver_poses = false` leaves out the 8 posed figures of every driver (about 230 MB over 67
   cars). The default is unchanged (poses on), so the size of an existing install changes only when the player asks.

Not done, with the reason:

3. **The virtual layer** (section 4): the design is above, but it touches the `GameDir` index, five files of the `bb.content` ABI and
   its snapshot tests, the host backend, the SDK, and a split of the mod's per-car conversion into a manifest part and a payload part.
   That is too much change to write without a compiler; it is the next step if the 590 MB matter, and the only one that removes them
   without a loss. The `_B` alias and a shared driver pack are smaller host changes with the same saving per file.
4. **Rims of other cars**: the wheel is one solid with the tyre, disc and caliper, so a rim as a shop part needs the rim cut out by
   material first; not a cheap item.
5. **Props, signs, road and sky textures**: wait for the trees PR (the model swap) and need the texture swap of section 3.4.
6. **Variant kinds** (helicopter, skid marks, debris, rain, ambience, grading): each needs a profile schema, a host option row and a scene
   that draws or plays it, by the checklist in [effect-variants.md](../modding/effect-variants.md#adding-a-kind-checklist). Skid marks
   and the helicopter are the smallest.

Each one is a key of the mod's config, a row of its README, and, where the player should switch it in the game, a Gameplay option
(the settings pattern of [api-settings.md](../modding/api-settings.md)) that shows only when the mod provides it.

## 7. Phase 3: the lean import and the lazy files

The user's rule is that anything that integrates with an external game is streamed from the original files, or else converted
by something very small, fast and small on disk. The car import wrote 750 MB, so this phase worked on that, in the order of
what the measurements said. The write-up with the measured numbers is [lean-content.md](../modding/lean-content.md).

| Step | Result |
|---|---|
| 1. `_B` solids | 232 MB (46 % of the geometry) were copies of `_A`; the host reads a missing `_B` as `_A` (`car/mods/levels.rs`) and the converter stops writing it |
| 2. Recordings | 1,070 of 1,338 files were copies; written once, named by content (`sound_store.rs`): 123 MB |
| 3. Driver and poses | **cannot be shared**: the poses reach for each car's own wheel grips (no equal vertex buffer was found); lazy files remove them from the disk instead |
| 4. Textures | only 6.7 MB of 78 are equal across cars; not worth a host feature |
| 5. The virtual layer | built as **lazy files**, a smaller design than section 4's: the mod writes a stub for each file and gives the host one function (`bb.content` 1.1 `provide_lazy_files`); the host recognises stubs in `GameDir::read`. No list of paths, no change to the layer index. The 2012 mod uses it for `GEOMETRY.BIN`, `TEXTURES.BIN` and the recordings (`stream_cars`) |

Left: the 11 traffic cars are still files (about 25 MB before step 1), the ignitions and the small shared files are files, and
the texture swap and the world items of section 3.4 do not exist. The sizes after the change are computed from the earlier
measurements and the first-use times of a car are not measured: both need a build.
