# Provenance: in-game screens' footers, dim panel, message screen and inbox key

Method and addresses for [fe-package-footer.md](../specs/fe-package-footer.md). Program `/speed.exe` (unmodded 1.3,
sha256 `80774c2e...`, checked with the Ghidra program information), image base `0x00400000`. Live reads come from the
front-end screen dump of the original run unattended (hidden, off-screen, muted); the sessions are private.

## Method

1. The package footers were compared object by object between the running original and the remake with the screen dump
   (both sides write the same schema). The flags of the footer objects (`0x8`, `0x40`, and the invisible bit 0 with the
   dirty bits a visibility change sets) showed which copy is drawn and that the game, not the package, changes them.
2. The test of flag `0x40` was found by scanning the code for byte tests of the object flags word (`+0x1C`): one object
   visitor makes such objects invisible, and the inlined set-visible refuses them. The visitor is created in the
   package-start method of the game's FEng interface (its vtable slot 3), which walks the package with three visitors.
3. The in-game push and pop were decompiled; the 44 call sites of the push were scanned for the arguments they push.
4. The message screen's handler has no Ghidra function; it was disassembled from bytes.
5. The keyboard defaults of the HUD actions were read from the action table (records of name pointer, device 1, scan
   code), found through the pointers to the action name strings.

## Addresses

| Role | Ghidra name | Address |
|---|---|---|
| set invisible (recursive, sets `0x2400001`) | `FUN_00514c70` | `0x00514C70` |
| set visible (recursive, refuses flag `0x40`) | `FUN_00514cc0` (also inlined, e.g. `0x0052AEC0`) | `0x00514CC0` |
| visitor: hide objects with flag `0x40` (vtable `0x0089B4A0`) | `FUN_00516230` | `0x00516230` |
| package start of the FEng interface (vtable `0x0089B438` slot 3) | none | `0x00525520` |
| package walk with a visitor | `FUN_005b1560` | `0x005B1560` |
| render one object (flag `0x10`: theme colour by render mode) | `FUN_00516630` | `0x00516630` |
| in-game push (the dim panel) | `FUN_005257f0` (`cFEng::QueuePackagePush`) | `0x005257F0` |
| in-game pop (record cleared on an empty stack) | `FUN_005258b0` | `0x005258B0` |
| win photo pushes with the skip flag | in `FUN_00605730`, `FUN_0060ac20` | `0x00605773`, `0x0060ADAF` |
| message screen handler | none (`uiSMSMessage::NotificationMessage`) | `0x00531D40` |
| message screen setup (hides Re-Play of a text message) | `FUN_00549150` | `0x00549150` |
| menu screen base constructor (back icon swap by region) | `FUN_00586290`, `FUN_00515bb0` | `0x00586290` |
| HUD pad handler (`FEngHud::JoyHandle`) | `FUN_0057c660` | `0x0057C660` |
| HUD hide on the in-game push: call slot 3 of every registered HUD | `FUN_0057d0f0` (list at `0x0092FD8C`, add `FUN_005947f0`, remove `FUN_005948b0`) | `0x0057D0F0` |
| HUD constructor (registers itself, builds the widgets) | `FUN_005a6600` (via `FUN_0059ca00`) | `0x005A6600` |
| HUD slot 3 / element mask (mask 0 hides every widget) | `0x005A0C50` -> `FUN_0057cdb0` | `0x0057CDB0` |
| package draw flag (filters; package screen record +0x26) | `FUN_005166a0`, `FUN_00591e30` | `0x005166A0` |
| delivery of a pending message -> show-SMS event | `FUN_005edfe0` -> `FUN_00631420` (pushes `SMS_Mailboxes.fng` with the slot) | `0x00631420` |
| inbox constructor (slot argument: open at that message, +0xE9; `HIDE_INCOMING`) | `FUN_00553260` | `0x00553260` |
| inbox handler (init sets +0xEB; then `READ` on the event handler and push `SMS_Message.fng`) | none (`uiSMS::NotificationMessage`) | `0x005534F0` |
| HUD action key defaults (`HUDACTION_PAD_DOWN/LEFT/RIGHT` -> `0x30`, `0x32`, `0x0F`) | data | `0x008F6FBC`, `0x008F6FF0`, `0x008F7024` |

## Dead ends

- No immediate of the pad copy's run-time textures (`0xC7162596` and neighbours) is in the executable, and no string
  hashes to them: they are built at run time.
- The reveal event of the keyboard art (`0xBDFD2DF9`) is no immediate or string either; the packages send it to
  themselves from their handler scripts.
- The two code tests of flag `0x8` on an object (`0x00429A59`, `0x0069C994`) belong to other systems.

## Message transition

The installed PC `SMS_Message.fng` package dump also documents the transition protocol in spec section 3.
Its entry script enables package buttons and sends `INIT_COMPLETE` at tick 500. The pad Back response and
the focused Done object's press response both start the same exit script on the package event handler.
The script posts the inbox control restoration at tick 0, then sends `EXIT_COMPLETE` at tick 500.

## First-car lot layout

A stock PC front-end snapshot during the first car purchase shows `Car_Select.fng` groups `0xA5E345EF` and
`0xB66F0FEC` running the authored `POS2` script (`0x001B20C3`). The Back group moves to y = 520 and Accept
moves to x = -240 in the package's coordinates. A separate stock screenshot shows Accept and Showcase without Back.
The remake applies this layout on the first-car purchase path. The snapshot covers that path.

## Rival bio and race photo copies

The installed PC first rival bio has a keyboard Continue group and a separate pad group containing Continue
and Cross. The pad root has ordinary group flags. The package scripts control each copy's reveal, but the saved
first-visit probe has no settled per-object dumps for both devices. The win photo's pad Race Stats text and
its offset shadow have the same authored depth, unlike Restart. The installed colors are white on `0xFC866` and
black on `0xFC868`. The reported remake frame shows the offset copy on top.

## Locked Challenge Rival during exit

The round-two remake trace records Back on `SafeHouseReputationOverview.fng` and a direct switch to
`MainMenu_Sub.fng`. Clip 18 shows the Challenge Rival box before the overview leaves while the bin still shows
unmet requirements. The installed overview's Back response posts `0x97BDB465`, which starts a 300-tick fade
on the hidden Challenge Rival group `0x55F6AA1A`. Starting a script in the remake runtime clears the object's
hidden flag. The overview reapplies the career's gate after the package scripts advance.
