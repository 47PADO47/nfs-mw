# Garage manufacturer decals

## Inputs and evidence

The PC parts table defines six decal model slots, 70–75, followed by six
groups of eight decal texture slots, 83–130. See [cardata.md](../formats/cardata.md).
The installed table's decal parts use part ID 80. Their `NAME` attribute is a
base texture hash, rather than a string-table offset [file]. Installed geometry
provides decal surfaces with `DUMMY_DECAL1` through `DUMMY_DECAL8` texture
references. Each reference identifies an authored position on that surface [file].
This implementation uses asset records and geometry. It does not port external
code or replace installed files. Matched original-game visual captures remain
required to establish pixel parity.

Door-number slot identities and paired editing are described by the CC0
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) behavioral reference,
`CustomizeNumbers` and `FECustomize.hpp` [decomp], summarized independently by
the garage reference investigation. Pricing is covered in
[garage-shop.md](garage-shop.md). No reference code was copied.

## Selection and compatibility

Keep the existing nine model/appearance selections stable. Store decal part
identities separately in six arrays of eight optional hashes, in native order:
windshield, rear window, left door, right door, left quarter, right quarter.
An empty position has no decal. Retain all eight native slots for future asset
extensions, even when an installed mesh exposes fewer positions.

Door texture positions six and seven are reserved for the first and second
number digit. Select these from the installed `NUMBER_LEFT` and `NUMBER_RIGHT`
brand groups. Their geometry uses `DUMMY_NUMBER_LEFT` and `DUMMY_NUMBER_RIGHT`.
The number editor applies the same pair to both doors; the native slots retain
independent identities. Number availability follows the selected body's meshes.

Offer a decal only when the part belongs to the texture slot's compatible type
and its installed texture can be decoded. Offer a position only when its
authored geometry contains the corresponding placeholder. Read door and quarter
surfaces from the selected body kit. Window surfaces use their installed model
slots. Reject saved identities and positions that the selected car/body lacks.

## Drawing

### Native navigation icons [reference]

The CC0 reference's `CustomizeSub::SetupDecalLocations` supplies these installed
frontend bitmap hashes, in the same zone order used by the decal record:

| Zone | Bitmap hash |
|---|---|
| Windshield | `0x52ded91d` |
| Rear window | `0xac7937b4` |
| Left door | `0xda88b711` |
| Right door | `0xc9a967c4` |
| Left quarter | `0x2c710c4d` |
| Right quarter | `0xffa7d360` |

Windshield, rear window and quarter locations open decal choices directly.
Doors open the position selector first. `SetupDecalPositions` assigns six left
door bitmaps `0xfe957f48` through `0xfe957f4d`, and six right door bitmaps
`0x2e40eadb` through `0x2e40eae0`. These icon identities are the same in both
rooms. A mod that authors several non-door positions can retain an extra
position selector, using that location's icon when no native position icon
exists. One-position non-door locations retain the native direct route.

`CustomizeDecals::BuildDecalList` uses shared bitmap `0x697b4ad4` for removal
and every manufacturer choice. Its separate name argument identifies the
manufacturer label. `AddPartOption` and `CustomizePartOption` pass these as
`IconOption(texture, name, description)`; the manufacturer name hash does not
select a frontend logo. Preserve this native shared icon rather than inventing
manufacturer bitmaps. Ignored installed tests verify the location/position
bitmaps exist in the installed UI assets and retain the direct/door routing.

### Black/white manufacturer filter [reference]

`CustomizeDecals` starts with its Black filter selected. Its current cart part,
or otherwise the installed part, supplies the initial brand and color: Black
when the part's native `NAME` attribute equals `BRAND_NAME`, White otherwise.
With no part, keep Black and select removal. Returning from Showcase retains
the saved filter and carousel index.

Each filtered list starts with removal. Black includes a part only when
`NAME == BRAND_NAME`. White includes it only when `NAME` equals the bString
continuation of `BRAND_NAME` with `_WHITE`. This is an exact attribute/hash
comparison; filenames, rendered RGB and localized labels do not classify color.
Unrecognized mod variants remain valid saved/rendered parts but do not enter
either original filter. Mods following the same attributes enter automatically.

Either trigger toggles Black/White and rebuilds the list. Preserve the currently
highlighted `BRAND_NAME` across the rebuild, selecting its opposite-color part.
If that brand has no matching color, select removal. Selecting removal before
switching keeps removal selected. The filter caption object `0x889bacb6` uses
language hash `0x41f0a3a5` for Black and `0x436a98e9` for White. Browsing the
filter previews the resulting native part; only Add to Cart commits it to cart.
The package also contains an offscreen Toggle Colour button with the same name
hash. Bind the nonbutton caption explicitly and retain its authored visibility.
The host also hit-tests the authored color bar and trigger images so pointer
input reaches that same filter toggle, without a separate custom control.

### Decal surface textures

The decal model part's `SHAPE` attribute selects `SQUARE`, `RECT` or `WIDE`.
Continue the decal texture part's base `NAME` hash with the corresponding
`_SQUARE`, `_RECT` or `_WIDE` suffix. The base hash itself is not the bitmap
identity. This rule comes from the CC0 reference's `CarRenderInfo` decal update
[decomp]; installed `CARS/TEXTURES.BIN` contains the suffixed texture identities
[file]. `SIZE` belongs to the authored model; preserve its geometry and UVs.
Texture availability and assembly must use the suffixed identity.

Draw a decal model only when its zone has at least one selected decal. Replace
its selected placeholder texture hashes with the selected decal texture hashes.
Keep unselected placeholders hidden. Apply this independently to each zone and
available driving LOD; identical placeholder names across zones must never
cause a decal to appear on another zone. Retain the mesh's original UVs,
transforms, material groups and installed texture alpha. No generated atlas,
guessed screen placement or movable decal transform is involved.

Decal textures remain independent of paint and vinyl composition, so repainting
does not erase them. Stock assembly keeps every decal surface hidden. Existing
preset rides with populated native decal slots use the same drawing path.

## Validation

Synthetic tests cover the complete native slot mapping, forged part identities,
missing zones, and a body-kit change that removes a position. The ignored
installed test assembles representative cars, checks each offered zone, and
verifies texture bindings and isolation from unselected zones. These tests do
not establish matching original-game appearance or live controller navigation.
