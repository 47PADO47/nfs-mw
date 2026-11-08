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

## 2. The categories in the shop [ours]

The gear is **for sale in a bike's shop and nowhere else**. The Visual category of the customization menu lists, right after Paint and
Vinyls, one entry for each part that has something to pick: **Rider Body**, **Rider Helmet**, **Rider Suit**, **Rider Gloves**, **Rider
Boots** (`frontend/garage_ui/choices.rs`, `rider_pages`, `shop_sells_gear`), each with a helmet icon of its own (`ui/rider_icon.rs`,
`assets/mods/rider-gear.png`, an original drawing). They are listed only when **all** of these hold:

| Condition | Why |
|---|---|
| the car on show is a **bike** (a mod car with `kind = "bike"`) | the gear is the rider's; a car has no rider |
| the player is in a **career shop** (`Model::shop`: a safe house or an auto shop with the career's owned car) | it is a purchase: it needs cash and a save |
| it is not the **backroom** (Junkman) | the backroom pays with markers, which the gear has none of |
| the setting `rider_gear` is on and a mod offers a catalogue | section 5 |

So **My Cars** (Customize Car), **the free roam garage** (no owned career car), **a car** and the F6 garage list none. The developer tools
are the other way to change it (section 5). Each entry opens a page (`Page::Rider { part }`, `gear_page.rs`) built like the plate
design page: the game's notice dialog, or a window of nine names in a theme with vertical menus.

| Input | Does |
|---|---|
| Left, Right (Up, Down in the vertical list) | the previous or next item, wrapping; the rider on the bike wears it at once; the text says the price |
| L, R trigger, mouse wheel | ten at a time, wheel one |
| Accept | puts the item in the **cart** and leaves |
| Back | leaves the cart as it was |

The first entry of every list is **Stock**: the rider's own gear (the suit, helmet, visor and boots of `parts.json`). It is free and
always owned.

**It is a shop item.** The cart carries it (`garage::Selection::gear`, a `rider_gear::Choice`; `Row::Gear(part)`), so it appears in the
shopping cart as `Rider Helmet: <item>`, with the other lines, can be unchecked there, and is part of the total. The checkout charges
it with the rest and then writes the choice (`rider_gear::purchase`, section 4) after the career's own save has succeeded. Leaving the
shop without paying (Back out, Leave Garage) discards it: the rider is dressed in what the profile kept.

**Prices and ownership [ours].** Neither the mod's catalogue (`gear.json` has no price) nor the 2005 price table (its entities are car parts)
has a price for gear, so each part has a **default price** (`garage/rider_gear/price.rs`) that every item of the part costs:

| Part | Price |
|---|---|
| Rider Body | 500 |
| Rider Helmet | 1,500 |
| Rider Suit | 3,000 |
| Rider Gloves | 600 |
| Rider Boots | 900 |

These sit near the first tier of the car shop's visual parts and are the ones to change if the career's economy is retuned. An item is
bought **once**: the profile owns it (`owned` in the file of section 4), and wearing it again, after another item or the rider's own
gear, costs nothing. The prices do not go through the career's unlock table (`shop_unlocks`): gear is open from the first visit to a bike's
shop, and there is no trade-in. The shop refuses gear for a car, with the setting off, and any id the catalogue does not list.

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

**One file for the profile, not one for each bike.** `rider_gear::choice::path` is the profile's `rider-gear.toml`; no part of the
path or of the table names a car. A bike swap (a different bike in the next shop visit, a different slot of My Cars, the free roam
bike) reads the same choice, and the shop's own selection carries it from one car to the next (`Model::select` keeps
`Selection::gear`); tests: `the_riders_gear_follows_the_player_not_the_bike` (`garage/menu_slots_tests.rs`) and the round trip of the file
(`garage/rider_gear/tests.rs`). The car's own record never holds it (`garage/presets.rs` and the career's garage extension leave
`gear` out), so a saved car is the same with any rider.

What the shop sold is in the same file, the items the player owns (the choice `rider_gear::purchase` writes at the checkout owns every
item in it; the developer tools' `commit` wears an item without owning it):

```toml
owned = ["helmet:HL_AGV_Corsa-R_Mono-Black", "suit:SU_Alpinestars_GP-Force_Black"]
```

## 5. The setting, the developer tools and no options rows

`rider_gear` (Gameplay options, **Rider Gear**; `config.toml` key `rider_gear`, environment `NFSMW_RIDER_GEAR`, console `set rider_gear
on|off`; on by default) turns the feature off: no gear page, no rider in the garage's backdrop, and the rider of the world wears his
own gear. It is a game-owned toggle declared in the mod settings table (`modding/backends/settings.rs`) so a mod cannot reuse its id.
The host pushes it to `rider_gear::set_enabled` each frame with the other settings (`sync_settings`, `Host::new`).
The `ride6-bikes` mod's own switches: `gear` (the catalogue folder) and `gear_texture` (README of the mod).

**The Gameplay options have no gear rows.** An earlier round listed one row for each part after the Rider Gear switch (Options >
Gameplay, in the main menu and the pause menu). They are gone: the gear is bought in a shop, not set in a menu, so the page keeps only
the **Rider Gear** on/off switch (the toggle of the external content). `Setting::Gear`, `options_gear.rs`, the `options-rider-*` messages
and `rider_gear::rescan` went with them; the catalogue is read when the world's garage model is made (`scenes/world/garage.rs`).

**The developer tools stay.** The `gear [list|set <part> <choice>|stock]` console command and the **Rider gear** section of the F11
vehicle panel ([devtools-vehicle.md](../devtools-vehicle.md)) still change the gear in any garage or in the world, without a shop and
without a price: they write the same choice to the same file (`rider_gear::commit`) and the rider dresses again. They do not mark the
item owned.

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
