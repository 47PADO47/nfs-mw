# Original garage packages and flow

## Sources and limits

**[verified]** Package and object facts were measured from the installed PC
`FrontB.lzc`, using the project's `Catalog` parser and `dump-screen` command.
The original PC 1.3 executable used for static checks has SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
Ghidra's program identity matches that executable. The modified executable is
a different build; its addresses must not be used interchangeably.
Live read-only captures of the modified game were also recorded for the My Cars
manager, customization main, all three subcategory groups, body choices, engine
packages, paint and empty My Cars cart. They establish those layouts for one
saved Audi TT, not all garage behavior. This session made no purchases, cart
additions, deletions, tuning changes or save actions, and ended through Quit.

**[decomp]** Behavioral reference: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw),
CC0, `MyCarsManager.cpp`, `CarCustomize.cpp`, `CustomizeManager.cpp`,
`FEPkg_GarageMain.cpp`, `CustomTuning.cpp`, `uiPause.cpp` and `FEPackageData.cpp`.
The category constructors and platform/camera selectors were independently
checked against the PC executable. This is static evidence. Live original
screen timing, controller interaction and matched visual comparison remain
separate checks. No original code or assets belong in this repository.

## Package routing

The game reads and runs the actual installed FEng packages. Its normal garage
must use the shared FEng presenter, fonts, resources, animations and responses.
The developer's F6 screen is independent of this menu stack and persistence.

| Screen | Package |
|---|---|
| Saved My Cars grid | `MyCarsManager.fng` |
| Add a car | `Car_Select.fng`, customization context |
| Main categories | `CustomizeMain.fng` |
| Parts, Performance, Visual subcategories | `CustomizeCategory.fng` |
| Rim brands, vinyl families, decal locations/door positions | `CustomizeGenericTop.fng` |
| Body kits, hoods, roof scoops, tint and vinyl patterns | `CustomizeParts.fng` |
| Spoiler patterns and group filters | `Spoilers.fng` |
| Rim models and diameter filter | `Rims.fng` |
| Performance packages | `CustomizePerformance.fng` |
| Body/rim/vinyl colors | `Paint.fng` |
| Decal choices | `Decals.fng` |
| Door numbers | `Numbers.fng` |
| Gauge styles | `CustomHUD.fng` |
| Gauge color channels | `CustomHUDColor.fng` |
| Career purchase cart | `ShoppingCart.fng` |
| My Cars cart | `ShoppingCart_QR.fng` |
| Backroom cart | `ShoppingCart_BACKROOM.fng` |
| Driving performance tuning | `Pause_Performance_Tuning.fng` |

The subcategory, generic-top, parts, performance, spoiler, rim, paint, decal,
gauge and gauge-color screens have `_BACKROOM` package counterparts.
`CustomizeMain.fng` itself receives a room-state message instead of a separate
backroom package. Names are matched case-insensitively by the catalog.

## My Cars

The manager displays five columns and two rows. Its ten image slots are
`CAR_ICON_1` through `CAR_ICON_10`. They display manufacturer logos, with the
selected car's model logo in the header, falling back to `GENERIC_LOGO_256`
when its secondary logo texture does not exist. The original array region is
`ARRAY_SCROLL_REGION`, hash `0xd276ba2f`. The package provides its own frame,
arrows, title, counter, footer and performance bars.
See [garage navigation](garage-navigation.md) for the manager grid and separate
Add Car list boundary rules.
Horizontal movement wraps within each five-column row, including a short final
row. Vertical movement stops at the outer rows; moving down into a missing
column of the final row selects that row's last actual entry.

The data list contains saved quick-race car records in stable order. It appends an Add New Car
entry when fewer than twenty custom cars exist and both the car-record and
customization-record stores have capacity. The add tile uses texture
`0x91d3a6ba` and label `0x29ae0714`. This tile is last and participates in the
displayed count. It opens `Car_Select.fng`; it does not
duplicate the currently selected setup as a side effect of opening the manager.
The Add selector uses the stock-car list, including its initially selected
model. Preview a stock setup before the first navigation gesture; confirming
the same model must not copy the fitted record's parts, paint or tuning. Cancel
restores the prior record selection and preview without creating a record.
Left/Right cycles the stock list circularly. Up/Down list-filter navigation is
inactive in customization mode. The native manufacturer arrows send Left/Right
through their mouse-release responses.
Its manufacturer image is `0x3e01ad1d` and model badge is `0xb05dd708`.
The model artwork names the car. Hide and blank all name placeholders
`0xd6d32016` and `0x79d6e45c`, including duplicate objects with the same hash;
the installed PC package includes an extra `Debug:` string under that name.
Showcase uses the same maker/model image bindings for the current preview.
Its controller-only hints group `0xd86aacf8` contains Rotate, Zoom Out, Zoom In
and their glyphs. Hide that group when the shared binding-aware orbit footer
is present; keep the surrounding native package artwork.
Selecting an existing car starts customization of that exact record. Multiple
records of the same model retain different handles and customization records.

The add-state message is `0x42ea22dd`; edit/delete state is `0x06d41ccc`.
These are state transitions, dispatched when the selected tile's mode changes
or the screen resumes. Repeating a transition every frame restarts authored
visibility scripts. The native performance labels, bars and model badge remain
inside the package's selected-car group.
The stats group uses the generic name hash `0x02ddf58c`. Footer suppression must
be scoped to each screen: the quick-race brief uses the same hash for Randomize.
The primary PC footer label object is `0xbfa25765`: Add uses label `0xc2598bd8`,
Edit uses `0xc9847935`. `PAD_BUTTON5` requests a Yes/No delete dialog; only Yes
deletes the selected record. `PAD_BUTTON0` opens Showcase for an actual car.
Back returns to Main Menu and saves a changed stable through the save flow.
The original postpones a newly selected car's load until a 0.25-second delay.

## Category membership and order

Main: **Parts**, **Performance**, **Visual**, in that order. Tuning is a driving
pause screen, not a fourth main garage category. A bounded search of the
original frontend screen registrations and controllers found no dyno or test
track garage entry. This does not assert that unrelated game modes lack tracks.

Parts: Body Kits, Spoilers, Rims, Hoods, Roof Scoops.

Performance: Engine, Transmission, Suspension, Nitrous, Tires, Brakes,
Turbo or Supercharger, with induction's label selected from the car's data.

Visual: Paint, Vinyls, Rim Paint, Window Tint, Decals, Numbers, Custom Gauges.
Backroom Visual contains Paint, Vinyls, Decals and Custom Gauges.
The fixed hero vehicle type suppresses Parts and Performance, and Rim Paint.
This is the installed `CarTypeInfos` entry's numeric type identifier 41, not a
folder-name or available-parts heuristic. Synthetic mod clones that reuse a
base entry's index without that native packed identity do not inherit this gate.
Capability checks for installed mod cars should derive from their part catalog.

The installed main/subcategory image template names are `OPTION_1`,
`OPTION_2`, etc. Hash `OPTION_1` is `0x4b4e3888`. `OPTION_MASTER` is
`0xb04669e3`, `ICON_SCROLL_REGION` is `0xb39a7599`, and `TITLE` is
`0x03c458c1`. Offscreen template coordinates are intentional: the original
scroller places the active window around the master image.

Rim-brand order: Stock, 5Zigen, ADR, BBS, Enkei, Konig, Lowenhart, Racing Hart,
OZ, Volk, Roja. Brand membership comes from `BRAND_NAME`; the rim screen filters
`INNER_RADIUS`. The shoulder/trigger controls wrap through allowed sizes.
Installed mod brands may extend this data without replacing known identities.

Vinyl-family order: Stock/remove, Flames, Tribal, Stripes, Racing Flags,
National Flags, Body, Unique, Contest. Pattern membership uses the part's group
number, zero through seven for the eight non-stock families. The part's packed
level/group byte stores the group in its low five bits and upgrade level in its
upper three bits; a group of eight is not a standard vinyl family.
Colorable vinyls enter `Paint.fng` before adding the vinyl and its remap colors
as one selection. Back from color editing resets the transient preview.
The same Paint package cycles remap channels through LT/RT, preserving each
channel's swatch and cursor. It hides trigger objects `0x2c3cc2d3` and
`0x53639a10` for one channel. For multiple channels it sends `0x1a7240f3`.
Filter label `0x78008599` uses `0xd8ee1a80` for one channel; two channels use
`0x5198ba16`, `0x5198be57`; three use `0x5198ba17`, `0x5198be58`, `0x5198c299`.
Accept adds the temporary vinyl and all remap colors together, then returns to
the vinyl choices. There is no separate category page of vinyl color channels.

Rim Paint uses one fixed installed rim-color palette, including its stock color.
It has no body-paint finish selector. Hide both authored filter groups and their
caption; LT/RT do nothing in this context. Body-paint finish families must not
change the stock rim-color entry or create a rim-paint preview.
The installed PC filter group is `0x2c526172`; its left/right glyph pairs are
`0x6b364f8b`/`0xd1cda594` and `0x79354351`/`0x9345e4a7`. These images have no
button response, so pointer clicks must use their native bounds explicitly.
Body paint retains a selected swatch for each finish. Horizontal grid movement
wraps inside the current twenty-column row, including a partially filled row.
Vertical movement stops at the first/last row. Moving down into a short final
row clamps an absent column to its final swatch.
Gauge-color Up/Down movement wraps through its one-dimensional color list.

Spoiler filters wrap through Sport, Tuner, Sport Carbon, Tuner Carbon, using
group values zero through three. Parts in common group four participate in each
filter. The original retains a cursor
per filter and restores the selected filter after Showcase.
Rim and spoiler filters also use the lower native left/right trigger images
for direction-specific pointer input. Their bounds are separate from the
top carousel arrows, so a filter click does not select an adjacent part.

Decal locations: Windshield, Rear Window, Left Door, Right Door, Left Quarter,
Right Quarter. Doors open a second category screen of positions; the original
constructor adds six door positions. Window/quarter entries open `Decals.fng`
directly. Body-dependent positions and shapes must remain compatible with the
selected body. Number selection uses `Numbers.fng`, with the two door sides
forming a paired selection; see [garage-decals](garage-decals.md).
Numbers uses two button strings in `FONT_MW_CUSTOM_NUMBERS.ffn`, hashes
`0x2a08ba92` and `0x1a88dc05`. Left/Right switches the active digit; Up/Down
changes it with wrapping zero through nine. The initial absent pair displays
`--`; its first scroll initializes both digits to zero. Accept adds both digits
on both doors and stays on the Numbers screen. `PAD_BUTTON4` removes the number
pair. The native digit group is `0x7a8355d9`, visible only for a valid pair.

The screen category numbers are stable routing identities:

| Route | Category range |
|---|---|
| Parts, Performance, Visual | `0x801` through `0x803` |
| Body, spoiler, rim brands, hood, roof | `0x101` through `0x105` |
| Seven performance categories, in the order above | `0x201` through `0x207` |
| Seven visual categories, in the order above | `0x301` through `0x307` |
| Vinyl stock/remove, then eight families | `0x401` through `0x409` |
| Six decal locations, in the order above | `0x501` through `0x506` |
| Six door decal positions | `0x601` through `0x606` |
| Stock rims, then ten brands | `0x701` through `0x70b` |

The original category textures differ between the front room and backroom:

| Category | Front icon | Backroom icon |
|---|---:|---:|
| Engine | `0xc15c94e6` | `0x4f424e0f` |
| Transmission | `0x01a29ffa` | `0xd142d3e3` |
| Suspension | `0x178475e7` | `0x00190eb6` |
| Nitrous | `0x9701bde4` | `0x6fea04c8` |
| Tires | `0x06e8e477` | `0x7373f1ef` |
| Brakes | `0xbaa23a28` | `0x4887f351` |
| Turbo | `0x06ef789c` | `0x12fe30a5` |
| Supercharger | `0x93603dfb` | `0x630071e4` |
| Paint | `0xa3b76154` | `0x0db89e17` |
| Vinyls | `0x55778e5a` | `0xd35f04c0` |
| Rim Paint | `0xd223f84a` | absent |
| Window Tint | `0x3f23165c` | absent |
| Decals | `0xda1dae54` | `0xa9135927` |
| Numbers | `0x45a1c644` | absent |
| Custom Gauges | `0x028f88bc` | `0x8ba602fc` |

## Installed visual controls

`Paint.fng` has eighty swatch button objects, `COLOR_1` through `COLOR_80`,
arranged in four rows of twenty. The swatch colors come from each installed
part's `RED`, `GREEN`, `BLUE` attributes. The array scroller owns selection,
focus and paging. Standard body paint filters are Gloss, Metallic and Pearl;
installed mod palettes can extend these families. The captured modified game
also exposes a Custom family with forty swatches. Rim paint has its own palette.
Shoulder/trigger controls change the filter or vinyl
remap channel. A custom RGB diagnostic control is not an original garage widget.

Vinyl remap colors use car-part slots 79, 80 and 81 with `BRAND_NAME = 0x03e871f1`.
They are not the body paint library. Gauge backing, needle and character colors
use slots 133, 134 and 135 without that brand filter. Each palette comes from
parts compatible with that car and slot, retaining part identities and database
order. Its RGB values are the installed `RED`, `GREEN`, `BLUE` attributes; equal
RGB values do not justify merging different parts. Palette metadata includes the
part's upgrade level for the existing unlock and transaction rules.
Garage selections retain each chosen native color-part hash alongside its RGB.
Two native parts with equal RGB remain distinct in preview state, cart state,
checkout and native save records. RGB-only presets remain supported: an unchanged
matching native swatch keeps its existing identity. Arbitrary RGB edits clear the
explicit native identity and remain in the existing record-bound extension.
Nullable color-part arrays use sparse channel/hash entries in TOML so one chosen
channel can be saved without null array elements. Old presets default to no
explicit color identities.

Gauge color editing stays inside `CustomHUDColor.fng`: its three layer options
are Needle, Characters and Backing, with icons `0x70f56628`, `0xbf6682c9` and
`0xcc9e1ce4`. The selected layer's palette fills that package's own `COLOR_n`
images. These layers do not route through the body Paint package.

The cart contains twenty-five template rows. Each row has `PART_NAME_n`,
`PRICE_n`, `TRADE_IN_n` and `CHECK_ICON_n`. It uses `ARRAY_SCROLL_REGION`.
The visible window has six rows in My Cars and four in career. Vertical
selection does not wrap. Total, available funds and eligible installed-part
trade-in update from active items. My Cars omits the career financial columns.
Cart labels use native localized category and package/part names. Paint includes
the installed finish and `SPEECHCOLOUR`; gauge names come from the native
`HUDINDEX` part. The check image is rebound from its authored arrow placeholder
to `0x696ae039` for active items and `0xe719881c` for inactive items, alongside
the native `CHECKED`/`UNCHECKED` scripts.

The shared financial text objects in part/paint screens are:

| Value | Main object | Shadow object |
|---|---:|---:|
| Selected cost | `0x6dfae796` | `0x20c83c31` |
| Funds | `0xa2a779b6` | `0x2922f251` |
| Total | `0x36140053` | `0xc3a57dc5` |
| Trade-in | `0xe83b28e3` | `0x2f5f0a1e` |

Choice-screen Cost is the highlighted item's gross price. Trade-in is the
eligible installed item credit, calculated against the entry baseline. Total is
the committed active cart's signed net amount; highlighting alone does not
change it. Cart summaries separately show gross item cost, trade-in credit,
signed net total and funds remaining. A net cash refund is not the trade-in
amount. Backroom totals count the actual marker items instead of cash.
The backroom cart retains the last selected root section. Its side panel shows
remaining markers in native order: brakes, engine, nitrous, induction, chassis,
tires, transmission for Performance; body, hood, spoiler, rims, roof for Parts;
vinyl, decals, paint, gauges for Visual. Active items subtract their matching
marker kind, including each separate decal. The summary's available-marker sum
is scoped to that root section. Unchecking a row restores its marker count.

Recoloring an installed vinyl or gauge queues its parent style again, even when
only a native color identity changes at equal RGB. Auxiliary color parts cost
zero; the parent style's ordinary cost or backroom marker still applies. Normal
garage quotes and visible financial values share one installed pricing reader.

Part choices use the category icon repeatedly; the original does not require a
unique thumbnail per body/hood/rim model. Carbon hood, roof and spoiler variants
have distinct icon hashes. Performance package choices use `0xb8c8c0d4`.
Choice captions use the part's `LANGUAGEHASH` when present, then its native
name, with existing mod labels as a fallback. Stock/removal choices use the
native stock label. Rim names remove the six-character size suffix, which the
separate native size widget displays. Both caption and shadow receive the same
resolved name after the status/detail binding.
The performance description contains `DETAIL_TEXT_LINE1` through `3`,
`PERFORMANCE_DETAILS_ICON1` through `3`, and `BRAND_ICON_1` through `3`.
Brand textures come from the car's `frontend` arrays `p_engine_<level>`,
`p_transmission_<level>`, `p_suspension_<level>`, `p_nitrous_<level>`,
`p_tires_<level>`, `p_brakes_<level>` and `p_induction_<level>`.
The selected package name is localized from its global `PN_LEVEL_n` label;
Junkman uses its dedicated label.

Performance description level is the selected local stage plus the category's
global maximum minus that car's available stage count. Global maxima are three
for Tires, Suspension, Induction and Nitrous, and four for Brakes, Transmission
and Engine. Description labels are `PD_<CATEGORY>_<level>_<line>`, using CHASSIS,
TURBO/SUPERCHARGER and NITROUS names for those categories. RX-7/RX-8 engine
descriptions have a rotary variant for line one of levels two and four. The
Castrol Ford GT reward has one engine description and brand override. Its
original gate is a runtime Castrol Easter Egg. The remake maps the loaded
profile's persisted Castrol entitlement to that display gate; this does not
implement the original runtime cheat-entry system. Stock or
backroom/Junkman descriptions show one category-specific line instead of three.
Career backroom performance offers Junkman, not ordinary stage packages.
The ordinary performance carousel contains local stages one through that car's
maximum, with no separate stock option. Original equipment remains a distinct
baseline state for comparison and the diagnostic overlay.

The player's part-status image is object `0xd0582feb`: installed texture
`0x696ae039`, cart texture `0x1a777e25`, hidden otherwise. The career-status image
is `0xcffb7033`: locked texture `0xf0574bb2`; a newly unlocked part uses the
object's own texture and blink response. Locked state comes from unlock rules,
not from affordability. Carbon variants use their original category icon and
installed part metadata. No extra custom status panel is needed.
Category pages start with both status objects hidden; only a selected part's
status binding may show them. Numbers compares both door digit pairs for its
installed/cart state. Its unset display is one dash per digit; when no numbers
are installed, that unset pair carries the installed status.

The gauge sample in `CustomHUD` and `CustomHUDColor` sits under group
`0xdee8632b`. Its initial script chains to HIDE. Opening or resuming the page
must play that group's authored SHOW script `0x5079c8f8` once; texture changes
alone leave the sample transparent. Keep the package's exit fade, including
its authored alpha. Visibility checks must inspect the presented gauge nodes
after the entrance animation, rather than only their texture identities.

The installed stat bars are filled from preview and installed performance using
the frontend-specific rating endpoints and package blend documented in
[garage-performance-bars](garage-performance-bars.md). A separate physical or AI
performance estimator must not supply those UI ratings. `BASE_<name>` retains
its full authored width. `FILLBAR_<name>` has width `160 * preview rating`, starts
at the base's left edge and retains the texture's pixels per UV unit. The
`HANDLE_<name>` left edge is `base left + 160 * installed rating`; it is an
absolute installed-value marker, not a signed difference. My Cars uses its
selected record for both values. Matched original runtime comparison remains a
separate check from this static reconstruction.

## Preview, cart and checkout

Three states are distinct: installed setup, active cart setup and highlighted
preview. Highlighting an item changes the rendered preview, but does not add it
to the cart. Accept adds the item or replaces that slot's pending item. Accept
on the installed item removes a pending replacement for that slot. Locked items
cannot enter the cart. Returning from a choice screen resets the transient
highlight to installed parts plus active cart choices.

`PAD_START` opens the cart. Done/Back from a Parts, Performance or Visual
subcategory also opens the cart when active items remain. With no active items,
it removes unchecked items and returns to Customize Main. This keeps ordinary
Customize Main exit separate from a pending purchase.

Cart `PAD_BUTTON4` (PC key 2) toggles the selected item's active checkbox; paired
door-number entries toggle together. `PAD_BUTTON5` (PC key 1, Clear All) unchecks
all items. It does not immediately delete the rows. Back clears unchecked items,
sends the cart-back message and pops the parent page while retaining active
items. Accept performs Checkout. An empty active cart resets the preview and
returns to Customize Main. A nonempty eligible cart asks Yes/No, defaulting to
No. The question labels are `0x71d9e710` for My Cars, `0x8ebaa44b` for career,
and `0x04810898` for the backroom. Insufficient funds uses `0x0a984a42`.

Confirmed checkout installs only active items and returns to Customize Main;
the player remains in the garage. Backroom checkout returns to its front room.
Replacing an unpurchased cart item retains the original installed trade-in basis.
Discarding the cart restores the installed setup and leaves money, heat, markers
and saved records unchanged. My Cars has no career cash charge. Backroom uses
the relevant reward markers. Save failures must preserve a recoverable cart and
the pre-purchase economy.

The non-career Performance subcategory offers Max Performance through
`PAD_BUTTON4` (PC key 2). Its Yes/No question is `0x892cb612`. Confirmation adds
the highest unlocked regular package for each performance category to the cart.
It skips unavailable categories and does not automatically install Junkman parts
or commit the cart.

Done/Back from My Cars Customize Main returns to My Cars Manager. Back from
career backroom Customize Main returns to front-room Customize Main. Back from
career front-room Customize Main exits to free roam. The room-switch control
uses `PAD_BUTTON5`, checks career markers, stays on Customize Main and preserves
the selected category. Its switch object `0xdc6ee739` is hidden when no customize
markers exist, or for the fixed hero car. Pressing the switch at zero markers
does nothing. Normal-screen input cannot also act on F6, drive the
vehicle or operate the underlying pause menu.

Free-roam auto-shop entry requires no event and no pursuit object, including
cooldown. The safe house has a separate cooldown exception; the auto shop does
not use it. Entering records that shop's native respawn marker, and leaving uses
that marker for position and heading. See [career-safehouse](career-safehouse.md)
section 3 and [career-safehouse-2](career-safehouse-2.md) section 9. These rules
were independently rechecked against the matched PC trigger-permission routine.

## Tuning

The original entry is Driving Pause / Performance Tuning, icon `0x483238fd`,
label `0x6a3672a2`, command `0x78f1c035`. It is absent from post-race screens,
split screen and DDay. Non-career Tollbooth and Challenge Series omit it too.
Otherwise it is enabled when at least one tuning path is available. It edits a temporary
copy of the selected vehicle's tuning, with Accept storing the settings and
Cancel preserving the installed values. Back with changes asks to discard;
Yes exits without storing, No remains in the screen. Start stores and unpauses;
Accept stores and returns to Pause Main. Accept copies all profiles and the
active index into the customization record and applies tuning to the running
vehicle immediately. Its three profiles wrap through
shoulder/trigger input. Profile-name labels are `0x40230063` through `65`.

The seven paths are Steering, Handling, Brake Bias, Ride Height, Aerodynamics,
Nitrous and Induction. Steering/handling/ride-height require an installed
suspension upgrade. Brake bias requires brakes. Nitrous and induction require
their corresponding upgrades. Aerodynamics requires an aftermarket body or
spoiler. Inactive sliders remain visible with explanatory help.

The object families are `OPTION_NAME_n`, `OPTION_DATA_n`, `OPTION_BACKING_n`,
`LEFT_ARROW_n`, `RIGHT_ARROW_n`; slider groups are `SLIDER_GROUPn`.
The negative and positive images are `FILLBAR_LEFT_SLIDER_n` and
`FILLBAR_RIGHT_SLIDER_n`, with `BASE_LEFT_SLIDER_n` and
`BASE_RIGHT_SLIDER_n` defining their ranges. In the installed package the bases
have signed widths -82 and +82; the authored fill placeholders have widths -38
and +38. Full travel uses the base range, not the placeholder width. Each fill's
signed width is its base width times its active half-range fraction. Its origin
stays at the base's signed top-left plus (2, -12); changing the negative width
therefore keeps its physical right edge fixed. Texture U ends at signed fill
width divided by the installed fill texture width. Preserve the other UV edges,
the fill height and the base geometry; hide the unused half at zero.
The generic widget formats an optional `OPTION_DATA_n` value from 0.0 through
10.0. Both installed PC packages omit these data strings; their visual control
is the signed bar, and the host must not synthesize an extra numeric caption.
Help uses the
original `HELP_TEXT_LINE1` and continuation objects, with a dedicated help state.

## Showroom, camera, animation and sound

My Cars uses the main frontend crib. The career shop uses platform
`CUSTOMIZATION_SHOP.BIN`; the backroom uses `CUSTOMIZATION_SHOP_BACKROOM.BIN`.
Their camera context names are `customization_shop` and `backroom`. The intro
collections are `angle_entry_<context>` and `angle_final_<context>`.
The common car stand is documented in [menu-backdrop-looks](menu-backdrop-looks.md).

For screen-specific shots, remove `.fng` from the package name and try
`customize_<screen>_<decimal category>`, then `customize_<screen>`. Its
`cam_angle` reference selects the actual camera collection. A missing collection
uses the existing frontend camera fallback.

Decal choice screens retain their location category `0x501` through `0x506`
(windshield, rear window, left/right doors, left/right quarters), even for door
slots `0x601` through `0x606`: the controller explicitly substitutes the current
location. This selects `customize_decals_1281` through `_1286`, with matching
`customize_decals_backroom_` names. Door-slot selectors use the matching
GenericTop collections: both doors share `angle_decal_doors`, or
`angle_backroom_door_main` in backroom. Additional locations use the same
category lookup with the original generic/default fallback when missing.

The original camera returns to the screen pose after 300 idle ticks after manual orbit, then starts automatic
horizontal rotation at speed 0.5 when the screen's `cam_user_rotate` permits it.
The return step uses the selected camera's `cam_anim_speed`. The counter
decrements only while orbit and zoom inputs are all zero. `cam_user_rotate` is
a boolean frontend attribute with key `0x998dd0db`. The inspected frontend schema
contains no `cam_auto_rotate` field or authored orbit-bound attributes. The
matched PC camera update integrates horizontal and vertical rotation input as
degrees at sixty times the input value per second. Radius input integrates at
7.5 distance units per second. Its ordinary limits are vertical angles 94.5 to
110 degrees and radius 4.65 to 6.65. Car Guys camera bypasses those clamps and
idle-countdown decrementing. Thus the original automatic horizontal input of
0.5 corresponds to thirty degrees per second. These constants come from the
matched camera update, not from guessed UI sensitivities.

Room geometry uses the `BACKDROP` solid-name prefix. The cast-shadow flag
restricts a shadow render pass to marked solids; it does not itself exclude
those solids from normal rendering. `NEXTGEN`/`CURRGEN` filtering follows the
selected render path. Emitters are not room meshes.

The package's own input, focus and transition messages run through FEng.
Original Sound responses must reach the existing menu-audio event sink. Avoid
an extra controller-generated accept/back sound when that response already
played it. Missing sound or animation evidence is not a verified parity pass.

## Validation requirements

Package parsing and synthetic routing tests establish identities and state
transitions. Installed renders must additionally cover My Cars, all three main
categories, the specialized choice screens, cart, backroom and tuning. Validate
select/add/remove/disable/checkout/cancel/save/reload separately from image
matching. Original runtime comparison must use matching car, room, selected
part, game installation and active input device. A rendered original asset by
itself does not establish complete visual or behavioral parity.
