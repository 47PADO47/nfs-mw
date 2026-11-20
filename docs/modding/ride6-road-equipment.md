# RIDE 6 bikes: road equipment on and off (mirrors and homologation kit as an optional component)

The side mirrors and the homologation kit (the plate holder, the rear lamp housing) of a road bike are parts of the bike's data (`parts.json`
`mirrors` and `homologation`, [formats/ride6-bike-parts.md](../formats/ride6-bike-parts.md)). Before this change the importer kept both in the
body solid, so they could not be left off. They are now solids of their own with one optional component slot, the mechanism the developer tools
use for a group ([devtools-vehicle.md](../devtools-vehicle.md), "Groups on and off"). Tags: **[M]** measured, **[C]** a choice of this project.

## What the importer writes

Two more stock pieces next to the six of the shop ([ride6-bike-upgrades.md](ride6-bike-upgrades.md); `mods/ride6-bikes/src/build_pieces.rs`,
`PIECES`): `<ID>_KIT00_MIRRORS_<L>` and `<ID>_KIT00_HOMOLOGATION_<L>`, for the four levels of detail, modelled in the body solid's frame (the mirrors
are anchored to `BONE_CHASSIS` in the data and do not steer; they did not before either). A bike gets a piece when its `parts.json` lists the part:

| Bikes of the 293 | Parts | Result |
|---|---|---|
| 157 | `mirrors` and `homologation` | both solids, the slot |
| 2 (Italjet Dragster 200 and 700) | `homologation` only | one solid, the slot |
| 134 (the race and EM variants, the older bikes) | neither | nothing changes |

So **159 bikes** have the slot [M: from `parts.json` of the exporter's output, `C:\r6work\bikes`]. The road equipment's slot does not need a row in
the optional-parts catalogue: a bike the catalogue does not list still gets it (`Shop::plan` plans with no aftermarket parts then); the slots of
the shop exist only for a run with `--bike-parts` (`upgrades = true`), so without them the road equipment stays in the body as it was.

```toml
[[slot]]
id = "road_equipment"
group = "road"
accepts = ["r6:road.on"]
default = "r6:road.on"
optional = true

[parts."road.mirrors"]
solid = "DUCATI_PANIGALE-V4-S-2025_KIT00_MIRRORS"
role = "body"

[parts."road.homologation"]
solid = "DUCATI_PANIGALE-V4-S-2025_KIT00_HOMOLOGATION"
role = "body"
```

The catalog file `components/ride6-bikes.toml` defines one component for every bike, `r6:road.on` (name `Road equipment`), whose `part` effect shows
`road.mirrors` and `road.homologation`; a part id a bike does not bind is skipped when the car is drawn, so a bike with only the plate holder uses
the same component. The default is shown: nothing changes for a bike that is not touched.

## Using it

`vehicle group road off` (console, or the checkbox in the F11 vehicle panel) takes both off, `vehicle group road on` puts them back, `vehicle set
road_equipment none` is the same for the slot. The slot has no garage category, so the choice is kept for the session only
(`select::category_of`); the garage does not list it. In a screenshot run: `nfsmw view-car DUCATI_PANIGALE-V4-S-2025 --yaw 60 --zoom 1.6 --muted
--exec "vehicle group road off" --screenshot off.png`.

## Checked [M]

Imported with `ride6-bike-import import-all` (the five bikes below in a folder of their own, `--bike-parts C:\r6work\bike-parts`) into the isolated
profile `C:\r6work\profile-bikegaps`, then `view-car` on and off (`C:\r6work\bikegaps-shots\road_*_{on,off}.png`): the Panigale V4 S 2025 (the mirrors
and the plate holder go), the BMW S 1000 RR 2019 (the mirrors go), the KTM RC 390 2023, the Italjet Dragster 200 (the plate holder goes) and the KTM 450 (no
such parts: `vehicle group road off` changes nothing, as the slot does not exist). `cargo test -p ride6-bikes` has the plan, the manifest text read back with the
game's own content schema and the slot checked against the catalog (`options_manifest_tests.rs`).

## Format change

The output of the importer changed (new solids, a slot, a component), so `stamp.rs` `CONVERSION` is 8: a start of the native mod converts every bike again, and
`import-all` writes it directly. No other mod's output or stamp changes. The `geometry` command does not split pieces and must not be run on a bike
imported with options (as before, [ride6-bike-upgrades.md](ride6-bike-upgrades.md)).

## Not done

The lights (a preset of the vehicle, not a mesh) have no component; the mirrors do not fold or steer with the bars; a garage page for the slot.
