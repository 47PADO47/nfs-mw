# The rider's gear: what the game does with it

How the player picks the helmet, suit, gloves, boots and body of the rider of a bike, how it is kept, drawn and turned off. The
data and the files are in [ride6-rider-gear.md](../formats/ride6-rider-gear.md); the rider is
[ride6-rider-render-integration.md](ride6-rider-render-integration.md). Tags: **[ours]** a choice of this port, **[unverified]**
written without a build or a run (no cargo was allowed), the lead checks it.

## 1. What a mod offers

A mod ships `RIDER/GEAR/gear.json` in its `files/` (the format is generic: bodies, helmets with an optional visor, suits, gloves
as a colour, boots, each with the paths of a mesh pack and a picture, and the names of the rider's own meshes that each part
replaces). The game reads it with `garage::rider_gear::Catalogue` when a garage model is made (`scenes/world/garage.rs`, `catalog`)
and again when the rider loads. A catalogue that does not read, or has nothing in any list, is as if there were none: no page and
no rider in the garage's backdrop. Nothing names RIDE 6 in the game side; the `ride6-bikes` mod writes the file.

## 2. The categories in the garage [ours]

The Visual category of the customization menu of a **bike** (a mod car with `kind = "bike"`) lists, after License Plate and
Plate Design, one entry for each part that has something to pick: **Rider Body**, **Helmet**, **Suit**, **Gloves**, **Boots**
(`frontend/garage_ui/choices.rs`, `rider_pages`). A car, the backroom and a game without gear list none. Each entry opens a page
(`Page::Rider { part }`, `gear_page.rs`) built like the plate design page: the game's notice dialog, or a window of nine names in
a theme with vertical menus.

| Input | Does |
|---|---|
| Left, Right (Up, Down in the vertical list) | the previous or next item, wrapping; the rider on the bike shows it at once |
| L, R trigger, mouse wheel | ten at a time, wheel one |
| Accept | keeps the item and leaves |
| Back | puts back the item he wore and leaves |

The first entry of every list is **Stock**: the rider's own gear (the suit, helmet, visor and boots of `parts.json`). Gear costs
nothing and is not in the cart; leaving the garage has nothing to discard.

## 3. Preview

The garage's backdrop and `view-car` are one scene (`scenes/car`). It draws the rider of a bike mod car on the bike at rest
(`scenes/car/rider.rs`): the world's `Rider`, made with `Rider::previewing()`, loaded the first time the bike is shown, posed and
skinned each update, his parts listed after the car's. He wears `rider_gear::current()`: the item on show (`set_preview`), else the
kept choice. When the preview changes, `rider_gear::generation()` moves and the rider dresses again (`dress_again`: his meshes and
pictures are made again, the pack is not read again). The rider is drawn in the garage only while the setting is on. The shadow
the backdrop captures is made when the picture changes, so a gear change that keeps the same meshes does not move it **[unverified]**.

## 4. Persistence [ours]

The choice is the player's, not a part of a car: one for every bike. It is kept in `rider-gear.toml` in the profile's mod data
folder beside `my-cars.toml` (`rider-gear-guest.toml` in the settings folder for a guest), the way the licence plate texts are
(`garage/plates.rs`); the original game's save has no place for it and stays untouched:

```toml
[gear]
body = "suit001_female"
helmet = "HL_AGV_Corsa-R_Mono-Black"
```

A part left out wears the rider's own gear; an id the catalogue lacks does too. The file is replaced atomically. The rider of the
world reads the kept choice (`committed`) when he loads and when the state moved; only the garage's backdrop reads the preview.

## 5. The setting

`rider_gear` (Gameplay options, **Rider Gear**; `config.toml` key `rider_gear`, environment `NFSMW_RIDER_GEAR`, console `set rider_gear
on|off`; on by default) turns the feature off: no gear page, no rider in the garage's backdrop, and the rider of the world wears his
own gear. It is a game-owned toggle declared in the mod settings table (`modding/backends/settings.rs`) so a mod cannot reuse its id.
The host pushes it to `rider_gear::set_enabled` each frame with the other settings (`sync_settings`, `Host::new`).
The `ride6-bikes` mod's own switches: `gear` (the catalogue folder) and `gear_texture` (README of the mod).

## 6. How the parts are put together

Planning stays the one of the stock rider (`parts::plan_without`): the entries of `parts.json` name a mesh and the role of one of
its sections, and `gear::dress` changes the list. For a choice: a **body** (the chosen one, or the catalogue's first when only the
suit or the gloves are chosen, since the stock suit mesh has no hand triangles to colour) replaces the stock suit and arms, its
`main` section wears the chosen suit's picture (else the stock suit's) and its `hands` section the gloves' colour (else the suit's
picture); a **helmet** replaces the stock helmet and visor (the visor of the item, translucent, when it has one; a goggle band
dark grey); **boots** replace the stock boots. A mesh pack's bones are found by name in the rider's skeleton. A mesh or picture
that does not load is logged and that item is not worn. Pictures are block-compressed (BC1 to BC3) uploaded as they are, mip
levels below 4 pixels dropped; a gear colour is a one-pixel picture.

**The helmet camera.** The first-person camera hides the meshes named in the bike's `[bike.camera.helmet] hide_meshes`
(`helmetRacing_default`, `visorClosedRacing_default`). The importer gives every helmet and visor pack those names, so whatever
helmet is worn is hidden from inside it, with no change to the camera code (`view.rs` is computed again when he dresses again).

## 7. Commands that show it (offscreen, muted)

```
nfsmw view-car KTM450 --muted --screenshot rider-stock.png
nfsmw view-car KTM450 --part rider-helmet=12 --part rider-suit=30 --muted --screenshot rider-helmet-suit.png
nfsmw view-car KTM450 --part rider-gloves=5 --part rider-boots=9 --part rider-body=3 --muted --screenshot rider-body.png
nfsmw view-world --drive KTM450 --camera-mode hood --muted --screenshot rider-hood.png
```

`--part rider-<part>=N` takes the number counted from 1 in the garage's list (1 is Stock), an id, or `stock`. The world takes the kept
choice: put a `rider-gear.toml` (section 4) in the profile folder (`BLACKBOX_PROFILE_DIR` for an isolated run) and the drive scene
wears it. Garage: `nfsmw view-world --drive KTM450` and the shop; Visual, Helmet.

## 8. Gaps

Gear is one colour picture per item (the colour sliders, normal and PBR pictures are not imported); gloves are a flat colour;
goggles, faces and the open visor are not offered; female bodies use the male rider's grips and pegs; the rider in the garage is
at rest (no animation); the shadow is not refreshed for a change that keeps the meshes. See the format page's Gaps.
