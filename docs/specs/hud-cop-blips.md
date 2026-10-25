# Cop icons on the minimap

How the original game draws a police car on the HUD minimap: which icon, its colour in normal driving and in a
pursuit, and how the colour changes over time. The minimap itself (the picture, the placement rule, the turning) is
in [hud-minimap.md](hud-minimap.md), section 7 of which says the blips were not implemented. This spec fills in the
cop case. The icon images and their names are in [../formats/minimap.md](../formats/minimap.md).

- **Source:** `speed.exe` v1.3 (`Need For Speed Most Wanted Black Edition`), disassembled and decompiled in Ghidra.
  Addresses are the binary's. No code was copied from any other project.
- **Evidence tags** as in [docs/README.md](../README.md#evidence-tags). **[binary]** = read in the decompiled
  code; **[inferred]** = a reading of the code that a capture has not yet confirmed; **[user]** = what the game
  looks like, reported by the user from play.

## 1. What is shown

- Each police car on the map is one element named `MMICON_COPCAR_<n>` (`n` = 0..7), a HUD object in the
  `HUD_SingleRace.fng` package. The format string `MMICON_COPCAR_%d` is at `0x0089d924`. **[binary]**
- The icon is white in normal driving and flashes blue and red during a pursuit. **[user]**
- The icon is the car arrow, not a dot: its texture is `Minimap_Icon_Car.tga`, the same arrow as the player's
  `PLAYERCARINDICATOR` (14 x 16, package colour `0xffebebeb`). Probed in `HUD_SingleRace.fng`, `MMICON_COPCAR_0` and
  `_1`. **[package]** A cop is a moving blip: it points along the car's nose, not the player's arrow (`hud-minimap.md` section 7).
- A car is drawn only while the minimap view covers it: inside the disc, never pinned to the rim, pursuing or not.
  The rewrite once pinned a pursuing car to the rim; the user asked for that to stop. The original's behaviour here
  has not been checked. (Section 8.)
- The minimap setup (`FUN_00548810`) hides the icon groups, then builds the blips (`FUN_00531060`, one object per
  eligible car), then the other minimap parts (`FUN_00531350`). **[binary]**

## 2. Which cars get an icon

`FUN_00531060` walks the list of vehicles `DAT_0092ce9c`..`DAT_0092cea4`. For each one:

1. It calls virtual slot `0x88` on the vehicle; if that is false the car is skipped. **[binary]** The meaning of
   slot `0x88` is not yet named.
2. It calls slot `0x8c` on the minimap element, which must be true, then slot `0x44` for the car's type and compares
   it with `DAT_0092c4f4`. Equal means the racer / helicopter branch (`FUN_0052f660`). Otherwise the car is a cop:
   the branch builds the cop object (`FUN_0052f500`, vtable `0x0089d768`) with the element `MMICON_COPCAR_%d`. **[binary]**
3. The cop object gets its position and heading from the car's world position (`FUN_00579840`) through the
   minimap calibration (the same mapping as [hud-minimap.md](hud-minimap.md) section 2). **[binary]**

A cop object is only created when the element exists in the package; a missing icon makes no blip. **[binary]**

## 3. The visibility flag

`FUN_0052f500` decides whether the icon is shown with one test:

```
shown = (HUD_flag_word & 4) != 0     # FUN_0056d540(base + 0x38, 4); base = DAT_0091cf90 + 0x10
```

When `shown` is true the object stores `+0x34 = 0` and clears the element's hidden bit (`FUN_00514cc0`). When it is
false it stores `+0x34 = 1` and sets the hidden bit (`FUN_00514c70`). **[binary]**

`param_1` is a **category mask**, not a pursuit flag: the same constructor takes `4` for cops (`FUN_00531060`), `8` for
racers and helicopters (`FUN_0052f660`), `0x20` for roadblocks (`FUN_00531350`) and `1` for the player's own arrow
(`FUN_00530f40`). `FUN_0056d540(flags, mask)` is `(flags[+8] & mask) != 0`, and `FUN_00530df0` builds the matching
`OPTION_ICON_` / `ICON_VIS_GROUP_` objects from the same table (`DAT_008f3ab8`, 0x14 bytes an entry, the mask in the
second word). So bit `0x4` is the HUD's "cop icons on" category setting: a cop icon is shown while its category is
enabled, whether or not the car is pursuing. **[binary]** (resolves old open questions 1 and 2.)

Cop icons are only built when the minimap is in mode 3 (`FUN_0055b8c0` sets the mode at `+0x164`; mode 3 calls
`FUN_00548810`, the only caller of `FUN_00531060`). Mode 1 builds the table-driven world icons and mode 0 other ones, with
no vehicle-list icons. The same class builds the roadblock icons (`FUN_00531350`, `MMICON_ROADBLOCK_%d`), so mode 3 is the
pursuit-oriented layout and may be shared by the world map's Pursuit tab. Mode 3 is chosen when a query on the player's
vehicle interface (the handle at `0x004040e0`, used at 101 sites for the vehicle interface) answers true, else from a settings
byte at `+0x46` of the HUD settings object. Which question that is has not been named, so whether the original shows
patrol cops outside a pursuit at all is **[open]**; if mode 3 means "in a pursuit", it does not.

## 4. The colour cycle

Vtable slot `+0x0c` of the cop object is `FUN_0051f6c0`. While `+0x34 == 0` it runs once per update and sets the
element's colour from a counter `+0x38` that runs 1..8 and wraps to 1. **[binary]**

| Counter | Colour (as stored) | Shade |
|---|---|---|
| 1, 2 | `0xff0000ff` | blue |
| 3, 4 | `0xffcccccc` | grey |
| 5, 6 | `0xffa00000` | dark red |
| 7, 8 | `0xffcccccc` | grey |

The decompile is `if (n < 3) blue; else if (4 < n && n < 7) red; else grey` (`0xffcccccc` is the default). An earlier
version of this table read steps 3 and 4 as red and 5 and 6 as blue, which does not match the code. The constructor
stores `-1` in the counter, so the first updates are blue before the cycle settles to 1..8. The values are ARGB:
`0xff0000ff` is pure blue, `0xffa00000` a dark red, and `0xffcccccc` a light grey. The pattern is blue, grey, red,
grey: the two light bars of a police car. **[binary]**

This update runs for every visible cop icon (the class has no pursuit condition), so the original flashes whether or not
the car pursues. The rewrite keeps a plain white icon for a car that is not pursuing, which is the [user] report of the
normal colour and is not confirmed by the code. **[open]**

The user reports the normal colour as blank white. **[user]** The code never writes white for a non-pursuing car:
the element keeps its own colour from the package, which is white. **[inferred]**

## 5. The pulse

Vtable slot `+0x28` is `FUN_0051f730`. It runs only while `+0x34 == 0`. It writes the scale of the sub-element
`+0x3c` each update: scale = `32 + 32 * f`, where `f` comes from `FUN_0045dbd0` (a time function, not yet read), and
the counter `+0x38` runs 1..32. **[binary]** The sub-element is not one of the eight icons, so its role is not yet
known (likely the pursuit ring or the GPS target pulse). **[open]**

## 6. Timing

The colour counter moves once per call of `FUN_0051f6c0`. The caller is a per-frame virtual dispatch that has not
been traced, so the rate is not known. The rewrite moves the colour counter once per `TICK_SECONDS` (0.1 s) of frame time, not once per frame. Stepping per frame
ran the eight-step cycle at about 7.5 Hz, which blended into a flicker rather than a light bar. The 0.1 s rate is a
choice, not a measured value. **[open]**

## 7. Open questions

1. ~~Polarity of flag `4`.~~ It is the cop-icon category mask (section 3). What sets the category word, and what selects
   minimap mode 3, are still unread.
2. **Hidden-bit meaning.** `FUN_00514c70` sets bit 0 of the element's flags word at `+0x1c` and `FUN_00514cc0`
   clears it. Which of show or hide that makes needs the FEng flag table.
3. ~~Colour byte order~~ ARGB, from the blue and red stored values (section 4).
4. **Update rate** of the colour cycle (section 6).
5. **Slots `0x88` and `0x8c`** (which cars are eligible).
6. **The pulse sub-element** (section 5).

A capture of a pursuit in the original, with the minimap on, would settle 3 and 4 by eye.

## 8. Implementation

The map blips are an abstract system, the same for every kind of blip:

- `crates/nfsmw/src/hud/blip_style.rs`: `MapBlip` (style name, world position, an `active` condition) and
  the trait `BlipStyle` (name, icon pool, `shows` condition, `look(tick)` for colour, fade and spin). Pure, tested.
- `crates/nfsmw/src/hud/blip_styles.rs`: the game's styles: `Cop` (flashes while `active`; `visibility` is `InZone` for every car), `Racer`, `Checkpoint` (shows only while `active`), `Helicopter`.
  `BlipStyle::visibility` takes the blip, so one style can gate some of its blips and not others.
- `crates/nfsmw/src/hud/blips.rs`: `BlipBinding` places every registered style's blips with
  `blackbox_minimap::place_blip`, keeps a per-style pool of HUD objects, and advances a tick each frame.
  `HudBinding::add_blip_style` registers a custom style.
- The cop feed: `TrafficWorld::map_blips` gives each cop car a `MapBlip`, `active` set while a pursuit runs.

Custom blip: implement `BlipStyle` for a type, give it icon names that exist in the HUD package, and register it
with `HudBinding::add_blip_style`. A condition goes in `shows`, an animation in `look`.

Status: the workspace builds and `cargo test -p nfsmw hud::` passes (51 tests). The pure style tests pass in a
scratch crate under clippy `-D warnings`. Done: the cop feed (`TrafficWorld::map_blips`, `active` while a pursuit
runs). Racer, checkpoint and helicopter have their styles and pools, and the world has a `map_blips` list that a
race or helicopter system can fill, but nothing fills it: the rewrite has no race or helicopter state yet, so those
three do not show in play. Not done: those systems, and a capture comparison.
