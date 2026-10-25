# Front-end menus (screens, icon menus, option widgets, flow)

What the game code around the FEng menu packages does: which screen logic runs a package, how the icon menus
and the option screens drive the objects of their package, and how the screens follow one another. The packages
themselves are described in [formats/frontend.md](../formats/frontend.md), how they run in
[feng-runtime.md](feng-runtime.md), and how pad input reaches them in [feng-input.md](feng-input.md).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled), under
  `src/Speed/Indep/Src/Frontend/`: `FEPackageData.cpp` (screen factory), `FEManager.cpp`,
  `MenuScreens/Common/{FEMenuScreen,feIconScrollerMenu,feUIWidgetMenu,feWidget,Slider}.cpp`,
  `MenuScreens/Safehouse/{uiMain.cpp,options/uiOptionsMain.cpp,options/uiOptionsScreen.cpp,
  options/uiOptionWidgets.cpp}`, `MenuScreens/InGame/uiPause.cpp`,
  `MenuScreens/Loading/{FEBootFlowManager,FEMovieScreen,FESplashScreen}.cpp`, `FEngHashes/*`. Read for
  understanding; no code copied. Provenance: [provenance/frontend-menus.md](../provenance/frontend-menus.md).
- **Evidence tags** as in the [docs README](../README.md#evidence-tags). **[decomp]** = read in the decompiled
  code, **[verified]** = checked against the packages of the install (object names hash to the ids below),
  **[ours]** = a decision of this rewrite where the original is not reproduced.

## 1. Screens

The game keeps a stack of loaded packages ("screens"). Each screen has a logic class chosen by the package
file name (`ScreenFactoryData`); the engine passes it every message a package sends to the game
(`NotificationMessage(message, object, controller mask, package)`) and a tick message every frame. Package commands
from responses or from code are **switch** (unload the top screen, load another), **push** (load over it, the
lower screen loses pad control) and **pop**.

| Package | Logic (original class) | Used for |
|---|---|---|
| `MainMenu.fng` | `UIMain` (icon menu) | the main menu |
| `MainMenu_Sub.fng` | `UIOptionsMain` (icon menu) when the game mode is options | the option categories |
| `Options.fng`, `OptionsPCDisplay.fng`, `Pause_Options.fng` | `UIOptionsScreen` (widget menu) | the rows of one category |
| `Pause_Main.fng` | `PauseMenu` (icon menu), or `UIOptionsMain` with the options flag | the pause menu |
| `LS_EALogo.fng`, `LS_PSA.fng` | `MovieScreen` | the boot movies |
| `MW_LS_Splash.fng`, `WS_MW_LS_Splash.fng` | `SplashScreen` | the title screen |

The screens that ship with the install and are not covered here (career, quick race, customisation, online,
dialogs) are listed by `nfsmw list-screens`.

**Leaving a screen [decomp, verified].** A button press does not act at once: the screen stores the message
and the pressed object, then posts `LEAVE_SCREEN` (`0x587C018B`) to its own package. The package responds with
the leave scripts of its objects (about 600 ticks) and, from a script event, sends `EXIT_COMPLETE`
(`0xE1FDE1D1`) to the game. Only then does the screen act on what was pressed (set a category, queue the next
package). `INIT_COMPLETE` (`0x35F8620B`) arrives the same way when a screen has finished appearing.

**Last button [decomp].** The game remembers per package which button was current when the screen was left and
selects it again when the screen returns (`FEngGetLastButton`); the icon menus store the option index.

## 2. Icon menus (`MainMenu`, `MainMenu_Sub`, `Pause_Main`)

A row of icons that scrolls under a fixed cursor. The package has image buttons named `OPTION_1` .. `OPTION_n`
(the buttons are `AffectAllScripts | IsButton | DontNavigate`; their positions in the file are placeholders):
**11** in `MainMenu.fng`, **12** in `MainMenu_Sub.fng` and **10** in `Pause_Main.fng` **[verified, `nfsmw dump-screen`]**.
The package also has a string `ICON_TITLE` (hash `0x5E7B09C9`) with its shadow `0x0DFB7A2E`, the region
`ICON_SCROLL_REGION` and the master `OPTION_MASTER`.

**Setup [decomp].** Each option has a texture, a name label and a description label. The screen puts two *book
ends* (an empty option with the texture `END_OF_SCROLLER`) before the first and two after the last option, so
slots 1..n hold `2 + options + 2` icons. Option `i` is given the image `OPTION_i`, the texture of the option (the
image's texture is swapped, `FEngSetTextureHash`), and an x offset: the offsets add up as
`offset(i+1) = offset(i) + width(i) + spacing`, spacing -5, widths from the package (64). The first real option
is selected. The decomp takes the centre `(cx, cy)` from `ICON_SCROLL_REGION`, which is then hidden. **Ours:** the
position of `OPTION_MASTER` instead: the cursor brackets sit on it, and the region is 17 units to the left of it in
`MainMenu.fng` (7 in `MainMenu_Sub.fng`), which left the selected icon off-centre in its brackets.

**One strip for every icon row (ours).** The original runs every `OPTION_n` carousel through the one
`IconScrollerMenu`. The rewrite does the same: `icon_strip.rs` owns the slots, the window, the scroll ease, the row
fade, the placement and the hit test, and a screen gives it a face per icon (texture, picture rectangle, drawn size,
enabled) and decides what a selection or a click means. The icon menus (`icon_menu.rs`) and the Blacklist overview's
sheet row (`career_screens/reputation_overview_row.rs`) are both on it. A disabled icon is capped at alpha 150
(`disabled.rs`; disabled texts use 110).

**Windowing (ours).** A menu may list more icons than its package has objects. The strip is the book ends and the
icons as above, and the row shows a window of the package's objects over it (`icon_window.rs`). The window is
centred on the selection, `base = min(max(selected - slots / 2, 0), strip - slots)`, so the three icons either side
of the selection always have objects (the scroller's edge is about three pitches from the centre). Sliding the window
rebinds each object's texture and picture rectangle and moves the scroll by `(base_new - base_old)` pitches
(`scroller.rs`, `Scroll::shift`), so no icon moves on screen. A strip that fits the objects is not windowed, which
is every menu but the main menu: its eleven objects hold the eight icons of 2.1 plus the two book ends each side
(12 strip entries), so the main menu slides by one. Before this the row warned and dropped the icons that did not fit.

**Every tick [decomp].** The scroll value moves towards `-offset(selected)` over 0.2 s with a cubic ease. For
each icon, with `x = cx + scroll + offset`:

```
scale = 0 outside [cx - W/2, cx + W/2]   (W = 350, the scroller width)
      = 1 within 1.5 of cx
      = linear to 0 at the edges:  left (x - (cx - W/2)) / (W/2), right ((cx + W/2) - x) / (W/2)
x'    = x + width * (1 - scale)^3  if x < cx,  x - width * (1 - scale)^3  otherwise    (pulled towards the centre)
the icon is hidden when x' lies outside the region, else centred at (x', cy) with size = original * scale,
its top edge moved so it scales about its middle
colour = idle colour * scale + fade colour * (1 - scale)      (white 0xFFFFFFFF -> 0x00FFFFFF; the pause
                                                              menu uses 0xFFFFAE40 -> 0x00FFAE40)
```

**Messages [decomp].** `PAD_LEFT` / `PAD_RIGHT` select the previous / next option that is not greyed out
(no wrap; **ours:** every icon can be selected, the ones without a screen yet (Online) are
greyed and do nothing on accept, except that the package's input lock is released again with `0x8CB81F09`), then refresh the header: the label of the option is set on `ICON_TITLE` and its shadow, and at the first
or last option the package gets `END_PAD_LEFT` (`0xD7118934`) / `END_PAD_RIGHT` (`0xB9B17747`) (the arrows
fade). `BUTTON_PRESSED` on the selected icon stores the press and posts `LEAVE_SCREEN`; the pause menu has no
leave message, so its icons run the event handler's `FORWARD` script themselves (the same 600 ticks, ending in
`EXIT_COMPLETE`). Back is the package's own: in `MainMenu_Sub.fng` the package answers `PAD_BACK` with its
leave animation and sends `PAD_BACK` to the game when it is over, followed by `EXIT_COMPLETE`; in the pause
package `PAD_BACK` reaches the game at once and the game runs `FORWARD`. On `EXIT_COMPLETE` the stored press is
applied (`React`) and the screen is switched. The icon menus remember the option they were left on (this rewrite
keys that by the package and which menu it is, since the pause package is two menus).

**Host input corrections (implementation-derived).** A native pointer click identifies the visible carousel
object, maps its slot through the current window base, and selects that logical icon before activation.
Changing only the package focus leaves the controller's selected icon unchanged and can open another screen.
When a pushed dialog or profile list closes, the retained carousel restores its reaction flag and clears its
pending leave action as well as restoring the icon fade. Otherwise a No or Back answer can leave the carousel
visible but unable to accept another selection. Safe house Resume, the alias manager's dialogs/lists and career
New/Load overlay the retained menu directly, like pause Quit/Restart, without playing its leave scripts:
No/Back preserves its header and brackets as well as its icons. Installed-package regressions cover direct and
windowed icon clicks, visible headers, and repeated safe house/Alias Save questions after cancellation. These are
host correctness rules, not new
claims about original executable behaviour.

**Footer and profile-list presentation [observed screenshots].** User-supplied PC reference screenshots
(2026-10-23) show Back followed by Accept in the original option-category, career and safe house carousels.
These native footers publish both actions in that order; the Remastered list keeps its existing footer policy.
The reference profile list shows the alias name without a text prefix: the package supplies selection colour
and brackets. Keep alias strings unchanged and run the installed row's highlight/unhighlight scripts.
Rows are text objects with authored column widths, not focus buttons. Pointer bounds use the column and
measured font height; the one-unit string transform is not a hit rectangle. Highlight scripts route the
package's row-specific cursor messages, moving its selection brackets.
Native pointer picking resolves the occupied slot on the current page before selecting or loading its alias;
empty slots consume the click without loading the previously selected alias. Keyboard/controller Accept reads
one gesture from the pad layer, so a row asset's own BUTTON_PRESSED response cannot suppress the selection.

The package leaves a stand-in icon (`OPTION_MASTER`, the image at the cursor) that the scroller hides, and
it positions the icons itself every frame: the package's scripts also write colours and sizes, so the host writes
its values after the update.

**Row transitions [decomp, verified].** The scroller grows in over nine 60 Hz frames, with a linear factor from
zero to one. The package sends `EXIT_STARTED` (`0x84378BEF`) when its leave script starts; the scroller then
shrinks out over nine frames and remains at zero until the screen switches. Multiply the geometric scale above
by this factor before setting size and colour, keeping each icon centred at `(x', cy)`. This applies to every
slot, including book ends; it does not replace the navigation ease or the package's header/footer animations.
The event and its timing were verified in the main, category and pause packages; the nine-frame duration comes
from the reference scroller. **Ours:** use elapsed seconds at the same 0.15 s duration, and preserve the current
factor if exit interrupts entrance or repeats, avoiding a jump back to full size. The embedded Controls icon
uses the same slot animation as the installed icons.

### 2.1 Main menu entries [decomp]

Challenge Series opens `ChallengeSeries.fng` ([challenge-series.md](challenge-series.md)). My Cars opens the normal
garage ([garage-frontend.md](garage-frontend.md)); Online remains disabled. `MainMenu.fng` options, in order,
as `(name label, icon texture)`: Career (`0x5815A2B5`, `0x03704F3D`),
Challenge Series (`0xCC8CB746`, `0x9A962438`), Quick Race (`0x54020A7A`, `0x4E6FBB02`), My Cars
(`0x1AFD5BE6`, `0xB0C46023`), Online (`0x74FB90A8`, `0x20563D30`), LAN (`0x4D78216D`, `0xD13A2E66`), Alias Manager
(`0xBCB18F38`, `0x6B303856`), Options (`0x19A8C0AF`, `0x3058FE37`). **Eight entries** (`icons(Kind::Main)`).
**[decomp]** `UIMain::Setup` (the non-Mike-Mann branch) adds Alias Manager only when `IsMemcardEnabled` is nonzero;
whether the PC build sets that flag is **[unconfirmed]**, so the rewrite shows the entry greyed out. The label
`0xBCB18F38` reads "Alias Manager" **[verified]** (`nfsmw strings`). The same decompiled setup adds no Online or LAN
entry to the main menu **[decomp]**. Online and LAN are listed anyway, disabled, at the user's request **[ours]**.
Online takes the label `0x74FB90A8`, "Online" **[verified, `nfsmw strings`]**, and the install's picture
`Main_Icon_Online` (texture `0x20563D30`, the hash of that name **[ours]**, a network of computers, checked on screen).
The option list's Online texture `0x5A5E5137` drew the gears of the Options icon, so it is not used. LAN takes the
main-menu LAN label `0x4D78216D`, "lan" **[verified]** (the install has no capitalised LAN label; "LAN main menu" `0x03F6F2BF` is not used), and the install's picture `Main_Icon_LAN`
(texture `0xD13A2E66`, **[ours]**, checked on screen). Both are greyed out (alpha 150) and do nothing on accept
(`act()` has no arm for them). The title is the label `0xB24AAE58` on `TITLE_GROUP` (`0xB71B576D`). With no profile loaded the
screen hides the player name group (`0xFB80CDAC`) and the game-statistics hint (`0x8913E195`). A `Quit` button (the
object `0xC0A32823`, label `0xC51FBF74`, keyboard hint `Q`) lives in the package and answers a mouse release
with the message `0x62799A4C` for the game **[verified, name unknown]**; this rewrite has no mouse, so the `Q` key
sends that release to the button and the screen quits the game.

Selecting Options sets the game mode to options and switches to `MainMenu_Sub.fng`; `PAD_BACK` there returns to
`MainMenu.fng`.

### 2.2 Option categories [decomp]

Found in the dump of `MainMenu_Sub.fng` (`nfsmw dump-screen`): twelve option buttons `OPTION_1`..`OPTION_12`
(name hashes `0x4B4E3888`..`0x4B4E3890`, `0xB51549B8`..`0xB51549BA`). The rewrite now lists the original's
categories in order: Audio, Video, Gameplay, Player (label `0xF760EABE` "Player"), Controls (`0x04DC6DB5`), and,
outside the pause menu, EA Trax (`0xDCFB6B36`) and Credits (`0x0905101F`). Player is enabled and opens its
implemented Transmission, Camera and Gauges controls; see [Player options](player-options.md). EA Trax opens the
jukebox and Credits opens the credits screen (`Credits.fng`, `credits.rs`), which goes back to the categories on Back.
The labels are what the game displays (`nfsmw strings`) **[verified]**.

**Controls split (ours).** The Controls icon opens the keyboard list (`Options_PC_Controller.fng`, `controls_screen.rs`)
directly, with the original's controller picture. Control Options (a title we chose; the original has no such entry)
sits beside it and opens the sensitivity and camera rows (`Options.fng`, category Controls): Deadzone Mode to Invert
Camera Y, then Save Alias and Load Alias. That list has no Keyboard Bindings row (`InputSetting::ALL`). Both icons leave
with the usual animation. Back on the key list returns to the categories (`Command::Switch`), which restore the icon
they remember, so the key list sits on no screen beneath it. The pause menu has seven icons: Audio, Video,
Gameplay, Player, Controls, Control Options and Discord. Player is available in both routes, matching the
original category order and the user's request to put the camera setting on its Player screen. This replaces
the earlier host choice to omit Player from pause while that screen had no implemented controls.

**Online is not listed (decomp, ours).** The original adds Online only in online mode, which this build never is, so
the offline list is Audio to Credits, without Online, plus Control Options and Discord (below). The greyed-out Online
entry (texture `0x5A5E5137`, `OMOnline`) is therefore gone; the main menu lists it instead (2.1). `MainMenu_Sub.fng`
has twelve option objects. With nine entries and the four book ends the strip is 13 entries long, so the row windows
over the objects (see "Windowing" in section 2). The pause menu's seven entries window too wherever its package has
fewer objects than the strip has entries (eleven).

**Discord category (ours).** Discord Rich Presence has its own category after Control Options, with its own icon: a
white Discord mark (`assets/discord/discord.png`, Simple Icons, CC0; see NOTICE), bundled and registered as texture
`0xE0C0_0002` (`ui/discord_icon.rs`, next to the input atlas `0xE0C0_0001`). Its header reads "Discord". The icon
opens `Options.fng` (pause: `Pause_Options.fng`) with category `Discord`, which shows the two toggles: Discord Rich
Presence (`discord`) and Show Time Played (`discord_time_played`, dimmed while Discord is off). These rows left the
Gameplay category (`options_rows.rs`); the toggles themselves are unchanged. The mark is drawn whole (full-image uv)
and at 0.65 of its slot (`DISCORD_DRAW_SCALE`, `icon_items.rs`, [ours]): at full slot size the unpadded mark read
bigger than the package pictures. The uv stays whole; shrinking the uv would enlarge the glyph instead.

**Icon textures.** `UIOptionsMain::Setup` adds each category as `OMxxx(texture, label, 0)`:
`OMAudio(0xF37AF144, 0xE76CD783, 0)` and `OMPlayer(0xD708EFEF, 0xF760EABE, 0)` **[decomp]**. The Audio pair is the one
the rewrite already draws, so the first argument is the texture: **Player's icon texture is `0xD708EFEF`** (set in
`icon_menu.rs`; `nfsmw view-screen MainMenu_Sub.fng --options --ui-script "wait 1;right;right;right;wait 1"` shows its
picture, dimmed). The earlier note that `0xD708EFEF` is a button name hash was wrong.
The same reading gives Online texture `0x5A5E5137` (`OMOnline`, win32 online mode) **[decomp]**; the rewrite does not
list Online (see above). EA Trax `0xC52CCBF6` and Credits `0x51009E20` **[decomp, picture verified on screen]** are
applied. `OMController` gives texture `0xA04A7B26` **[decomp; picture verified on screen]**:
`nfsmw view-screen MainMenu_Sub.fng --options --ui-script "wait 1;right;right;right;right;wait 1"` draws a gear-shift
picture in the Controls slot, which the rewrite uses for the Controls icon. Control Options draws the input glyph atlas
(`input_icons::ATLAS`, `Glyph::Controller`), the picture the Controls icon had before the split.

**LAN.** The strings `LAN main menu` (`0x03F6F2BF`) and `LAN options` (`0x19889765`) exist **[verified]**, but neither
`UIMain::Setup` nor `UIOptionsMain::Setup` adds a LAN icon **[decomp]**, so the original has no LAN entry. The
main menu lists a disabled one anyway (2.1), at the user's request **[ours]**; the category list has none.
The main-menu picture is `Main_Icon_LAN` (`0xD13A2E66`), found by hashing the install's `Main_Icon_<name>` naming
(`Main_Icon_Options` hashes to the Options texture `0x3058FE37`).

**Ghidra.** `/speed.exe` has no function names, and the byte search needed to find the code that adds these
entries was not available in this pass. The hashes above come from the decompiled source; the Player texture was
checked on screen (a wrong hash would draw no picture).

[decomp] `MainMenu_Sub.fng` as `UIOptionsMain`: options Audio (`0xE76CD783`, icon `0xF37AF144`), Video (`0x8A006328`),
Gameplay (`0x4DF98FB2`), Player (`0xD708EFEF`), Controller (`0xA04A7B26`), then EA Trax and Credits when not
opened from the pause menu. The title is `options` (`0x4ECA678C`). Pressing one sets the category and switches to
`Options.fng` (in game: `Pause_Options.fng`); `PAD_BACK` returns to `MainMenu.fng` (in game: `Pause_Main.fng`).

### 2.3 Pause menu [decomp]

`PAD_START` while driving pauses the game and shows `Pause_Main.fng`. Its options for a free-roam
session: Resume Free Roam (label `0x01BD185C`, icon `0x12BB5EA2`), the career quit entries (label `0x3C14C420`
"Jump to Safe House", icon `0x4C9E34E6`), Tuning (when available) and Options (label `0x2B5A03A8`, icon
`0x520DE4E3`). This rewrite shows Resume Free Roam, Options and "Quit to Main Menu" (label `0xE950B7AF`, the
challenge series' quit entry, with the same icon) **[ours]**; the original asks for confirmation first (a dialog
this rewrite does not have). The header `0x863404B5` shows
`0x6C839FBE`. `PAD_START` / `PAD_BACK` resume; Resume and Quit act on `EXIT_COMPLETE` (resume = unpause, quit =
back to `MainMenu.fng`); Options switches to the option categories with the pause flag, whose Back returns to
the pause menu and whose Start resumes.

## 3. Option screens (widget menus) [decomp, verified]

Layout rules were read from the code and checked on the packages; where they could not be reproduced the
difference is named.

`Options.fng` and `Pause_Options.fng` have numbered rows. Row `n` is the objects `OPTION_NAME_n` (a string and a
button), `OPTION_DATA_n` (a string), `LEFT_ARROW_n` and `RIGHT_ARROW_n` (images), and for sliders `BASE_SLIDER_n`
and `FILLBAR_SLIDER_n` (images), with `TITLE_MASTER` and `DATA_MASTER` fixing the row's title and data columns. The
screen shows at most 9 rows (10 from the pause menu). Rows are added in order; a row is a *toggle* (a title and a
data string that cycles through values) or a *slider*.

- **Selection.** `PAD_UP` / `PAD_DOWN` select the previous / next enabled row with wrap-around; the selected row's
  objects play `HIGHLIGHT` (`0x249DB7B7`), the previous row's `UNHIGHLIGHT` (`0x7AB5521A`); disabled rows play
  `DISABLE`. When there are more rows than fit the list scrolls and the cursor object plays `POSn` for the row
  slot. `PAD_LEFT` / `PAD_RIGHT` / `BUTTON_PRESSED` act on the selected row.
- **Toggle row.** Left or right flips or cycles the value; the title and the data string are language labels
  (`FEngSetLanguageHash`). Booleans show `On` (`0x417B2604`) / `Off` (`0x70DFE5C2`).
- **Slider row.** A value in 0..1 moves by 0.1 per press (`min`, `max`, `increment`). The fill bar width is
  `range * (value - min) / (max - min)` (range = the width of the slider base, 164), and its right UV edge is
  set to the same width; the data string shows the integer value. The value is applied at once (audio volumes
  update the mixer on every press).
- **Accept and back.** The package answers `PAD_ACCEPT` with its leave animation; `PAD_BACK` reaches the game, which
  asks to confirm when something changed (a two-button dialog, restoring the old values on yes) and otherwise posts
  `LEAVE_SCREEN`. On `EXIT_COMPLETE` the options are marked dirty (saved later) and the previous screen is shown.
  `PAD_BUTTON5` offers the defaults. This rewrite has no dialogs: changes apply at once and stay, so back is accept,
  and the defaults hint is hidden **[ours]**.
- **Layout** (parent coordinates). The title column is the box of `TITLE_MASTER` and the data column the box of
  `DATA_MASTER`; row `k` of the view sits `k` heights below the top of the title column. The title is aligned in
  its column by its own justification (the packages right-align it), the data string likewise (centred), the arrows
  are centred on the data column's two edges, the slider base is centred under the data column, 9.5 below the row's
  top, and its fill bar starts 2 in and 12 up. The original centres the text box on the column's left, middle or
  right *edge* (`FEngSetCenter`), which makes right-aligned titles overshoot the column by half their width; this
  rewrite aligns them inside the column **[ours]**. The package's scripts write positions too (the arrows' scripts
  hold a placeholder position), so the rows are put back after every update.
- **Highlight.** The focus goes to the row's title button; the package answers `BUTTON_HIGHLIGHT` with the
  highlight script and, through the row messages, shows the arrows and moves the cursor group (`POSn`). The data
  string and the slider objects get `HIGHLIGHT` / `UNHIGHLIGHT` from the screen.
- **Header.** `HEADERTEXT` (`0x42ADB44C`) gets the category title label (Audio `0x3932C2E4`, Video `0x48478029`,
  Gameplay `0x01CCE8C2`, Player `0xC055165F`; from the pause menu `0xB1426DFA`, `0xD94EA03F`, `0x3936D9F8`,
  Player `0xD9DC2F12`). Controls and Discord have no
  label: their header is text (`Control Options`, `Discord`).

The original audio rows are: Sound Effects Volume (`0xFD487543`), Car Volume (`0x218E4B08`), Speech Volume
(`0x9E5FB82A`), Menu Music Volume (`0x418E681D`), Game Music Volume (`0xDF21EAC2`), Interactive Music
(`0xA3DBB390`), EA Trax mode (`0xDCFB6B36`) and, in the front end, Audio Mode (`0x2881AB87`); video has
Widescreen (`0xD3588630`); gameplay has Autosave, Game Moment Camera (`0xF26A5CBF`), Car Damage (`0x1582ADFF`),
Rearview Mirror (`0x85A6CE05`), Units (`0x01E19173`; Imperial `0xFBD74FC5`, Metric `0xAF70E736`) and the map
settings; the Player screen starts with Transmission, Camera (`0xF6CCDC5F`) and Gauges (`0xAC148579`).

### 3.1 What this rewrite shows [ours]

The original settings are not the settings of this rewrite, so the rows are mapped, keeping the original
screens, labels and behaviour where a label exists:

| Category | Row | Label | Setting |
|---|---|---|---|
| Audio | Master Volume | `0x26782C3E` | `master_volume` (slider, 0 to 100 in steps of 10) |
| Audio | Sound Effects Volume | `0xFD487543` | `sfx_volume` |
| Audio | Engine Volume | `0xA2B1F888` | `engine_volume` |
| Audio | Menu Music Volume | `0x418E681D` | `music_volume` |
| Audio | Speech Volume | `0x9E5FB82A` | `speech_volume` |
| Video | Vsync | `0x6CEB9CB6` | `vsync` (On / Off) |
| Video | Frame Limit | none in the language table, plain text | `max_fps` (Unlocked, 30, 60, 120, 144, 240) |
| Video | Performance Overlay | none, plain text | `show_metrics` (Off, Basic, Advanced) |
| Player | Transmission | `0xD31407E7` | `transmission` (Auto `0x8CD532A0` / Manual `0x317D3005`; left and right both toggle) |
| Player | Camera | `0xF6CCDC5F` | `camera_mode` (existing original and custom camera selections) |
| Player | Gauges | `0xAC148579` | `hud` (On / Off) |
| Discord | Discord Rich Presence | none, plain text | `discord` (On / Off; ours) |
| Discord | Show Time Played | none, plain text | `discord_time_played` (On / Off, dimmed while `discord` is off; ours) |

The **HUD** category (`Category::Hud`, after Player; `frontend/hud_options.rs`) holds one row per `[custom_hud]` key,
the map's orientation (`minimap`) and the speed units, in the order of `docs/specs/custom-hud/menus.md` section 4; the
values and steps are in [hud-settings.md](../hud-settings.md). Its rows write `Settings::hud_options` and record their
keys in the `[custom_hud]` table of the changes. The icon borrows Gameplay's picture; the name is the catalogue's
`menus-hud`. A list look carries the page's note (`hud_options::note`) as the list's message.

A row can be disabled by its setting's `enabled` rule (`frontend/options.rs`). A disabled row stays selectable and shows its value dimmed (alpha 110), but left and right do nothing and nothing is saved. Frame Limit is disabled while Vsync is on.

Changes apply at once. The config file layer is written when the screen is left (accept or back), when the pause
menu is closed and when the game is left: only the keys that were changed, so `game_dir` and unknown keys stay.

## 4. Game flow

**Boot [decomp].** `BootFlowManager` shows, in order: `LS_EAlogo.fng` (movie `ealogo`, cannot be skipped),
`LS_PSA.fng` (movie `psa`, accept or start skips), `MW_LS_AttractFMV.fng` (movie `attract_movie`, skippable; shown
when the splash times out, not in the normal order), `MW_LS_Splash.fng` (the title screen), then `MainMenu.fng`.
A movie screen shows its movie as an FEng movie object (the object whose name hash is `0x58BCF5B6` /
`0x72CF9F38`), tells the game `0xC3960EB9` when it ends, and the game switches to the next boot screen. The
package sends `INIT_COMPLETE` about five seconds after it appears (the text fades in meanwhile); then
`PAD_ACCEPT` / `PAD_START` go on. The game hides the high-definition group, the mouse hints and a console licence
line, and gives the prompt group the label `0x9B580A55` ("Press START"). The original starts the attract movie
after 30 s without input; this rewrite does not. The original opens the widescreen package (`WS_MW_LS_Splash.fng`) when the screen is wide and queues
`CURRENT_GEN_WIDESCREEN` (`bStringHash`, `0xCB835EE3`) to it, which fades its 900 x 480 art in. This rewrite does the
same for any window wider than 1.4:1 (the 4:3 package is for the rest), so the art fills the window instead of leaving
bars at its sides. A mouse click counts as accept on the boot movies and the title screen. The memory-card screens between splash and main menu (`MC_Main_GC.fng`) do not apply on the PC.

**Free roam [decomp, ours].** The career starts in free roam in the original; this rewrite has no career, so
Career starts free roam with the stock car. Free roam starts behind the loading screen
(`WS_Loading.fng`) and shows the world only once it is ready; see [loading-screen.md](loading-screen.md).

**Quick Race [decomp, ours].** The Quick Race menu opens Quick Play or Custom Race ([quick-play.md](quick-play.md),
[custom-race.md](custom-race.md)).

## 5. Quick Race screens

`Track_Select.fng` (`UIQRTrackSelect`) and `Track_Options.fng` (`UIQRTrackOptions`) are the Custom Race's track list and options
since the Custom Race was built: their logic is `custom_race/track_select.rs` and `track_options.rs`, and the whole chain (mode
select, track list, options, car select, Showcase, transmission) is in [custom-race.md](custom-race.md). The earlier screens that
stepped through every race of the install and kept four options in the `quick_race` config key are gone; that key is ignored.

The package facts the screens rely on (kept from the first reading): the track list's event handler is the object `0x47FF4E7C`
(its `FORWARD` script `0xDE6EFF34` sends `EXIT_COMPLETE` after 600 ticks); the accept group is `0x73DCB662` and the back group
`0xC0A32823`; Track_Options' own `Options_Event_Handler` is `0x6B7A94E9`, its `Defaults` group `0xD6463100` sends `0xC519BFC4`.

Message hashes were checked against `bStringHash` of the upper-cased names (`PAD_BACK`, `PAD_ACCEPT`,
`LEAVE_SCREEN`, `EXIT_COMPLETE`, `MOUSE_LEFT_RELEASED`, `PAD_LEFT`, `PAD_RIGHT`, `PAD_UP`, `PAD_DOWN`).
Object names are not recovered (they are only known as hashes; the names above come from the string texts of
the dump, not the object names).

## 6. The list looks [ours]

Two looks draw the menus as lists from the entries the screens publish (`ScreenLogic::list` into the `MenuListCell`):
Remastered and Custom ([ui-themes-custom.md](ui-themes-custom.md) section 10). The FEng packages keep running underneath;
only the drawing and the input of the list change. `frontend/logic/look.rs` decides it:

- `Cx::list_look`: Remastered or Custom, when no mod theme took the menus over. The icon menus, the option pages and the
  dialogs ask it.
- `Cx::screen_list_look(ListScreen)`: the other list screens, per screen. Remastered draws them all; Custom draws the key
  list (`Controls`), the EA Trax playlist (`Jukebox`), the credits (`Credits`) and the Blacklist (`Blacklist`, PR 11b),
  and leaves the marker cards (`Markers`) and the rap sheet pages (`RepSheets`) to their packages.
- **The Blacklist's flow** moves as one: the list of rivals and the bio (`SafeHouseRivalChallenge.fng`,
  `SafeHouseRivalBio.fng`, both on the `Blacklist` slot), the safe house's Blacklist entry (it opens the list rather than
  the overview) and the reputation overview. The overview has no slot of its own, so no list look draws its list, but
  it takes the list's input whenever the Blacklist is a list: its Challenge entry is where a boss race starts from, and
  a list of rivals that led back to the overview's package could not reach it.
- The truth table (screen by style, and every screen false under a mod theme) is the test in `look.rs`; under
  Remastered every list screen answers as it did before PR 11b.
- **Clicks** are tested against the look that draws the list (`frontend/screens/geometry.rs` `ListGeometry`): the Custom
  menus' `MenuFrame`, its dialog, its Blacklist rows (`custom/menu/blacklist.rs` `row_at`) and its footer. The LAN page
  is not a stack screen: its click map (`frontend/lan/present.rs` `ClickMap`) uses the Custom places when the Custom
  theme provides the `LanMenu` slot (`ClickMap::in_custom`), the Remastered ones otherwise (the Original LAN page stands
  where the Remastered one does).

## How to check it

1. `nfsmw view-screen MainMenu.fng --screenshot out.png`, then with `--ui-script "wait 1;right;right;wait 0.5"`: the icons
   scroll, the title follows the selection.
2. `nfsmw view-screen Options.fng` for the rows (the screen needs its category: `--category audio`).
3. Compare with the running original at the same window size (not done): icon sizes and positions after a scroll,
   row positions of the option screens.
4. `nfsmw view-screen MainMenu_Sub.fng --options --ui-script "wait 1;right;right;right;right;accept;wait 3"` shows the
   keyboard list (Controls). With one more `right` and `accept` instead, the sensitivity rows show (Control Options).
5. `nfsmw view-screen MainMenu_Sub.fng --options --ui-script "wait 1;right;right;right;right;right;right;wait 1"` shows
   the Discord icon selected in the strip (titled "Discord"). With `accept` added before the last wait, the Discord
   sub-screen shows its two toggles (`nfsmw view-screen Options.fng --category discord` opens it directly).

## 4. Findings: credits, EA Trax buttons and the mouse (2026-10-17)

Found while making the Credits icon and the EA Trax jukebox work on a click, with the Ghidra project `/speed.exe` open.

**Pictures.** The EA Trax and Credits icons drew nothing: their textures were 0 (section 2.2). They now use `0xC52CCBF6`
and `0x51009E20`, checked on screen.

**Credits screen (`Credits.fng`, 63 objects, no button flags).**
- The Credits icon opens `Credits.fng` with `CreditsLogic` (`credits.rs`). Back (`PAD_BACK`) returns to the categories.
- The package starts with its buttons off. Back does nothing until the logic posts `INPUT_ENABLE`. Verified by test.
- The roll text is the object `CreditsArea` (name hash `0xE6093486`, the object the package shows as "DEFAULT STRING").
  Its text is the install's `CREDITS/NA_ENGLISH.TXT` (UTF-16LE). The original roll is driven by scripts of the package
  (`0x587E5F53`, `0x0F8F52C3`, 5.2 s and 7.2 s); they do not start in this port, so the text sat still.
- The renderer does not clip text (`UiNode::clip` is always `None`), so a long roll ran out of its box.
  **Ours:** the logic shows a window of 12 lines, moved down the box at 2 lines a second, with the object held at the box
  centre (`BOX_CENTRE_Y`). The original's clip rectangle and roll timing are not read yet.
- The visible Back button is the footer group `0xF8107` ("Back" with a triangle). It has no mouse answer. The package's
  Back group `0x110749` (answers the mouse with `PAD_BACK`) is hidden when the screen opens. **Ours:** a click inside
  the visible Back's rectangle (`BACK_CENTRE`, `BACK_HALF`) goes back to the categories.
- **Not resolved:** the black area behind the credits, where the menu car should show. Hiding the package's full-screen
  panel (`0xEB4CF244`, name of the 1200 x 516 `BasePoly`) changed nothing in `view-screen`, so that panel is not the
  cause. The real-game capture for this was not completed (the scripted run timed out in screenshot mode).

**Mouse on packages without button flags (generic, `libs/blackbox-feng`).** The EA Trax and Credits packages have no
object with the button flag (`IS_BUTTON`, bit 28), so a click never reached them: `button_at` found nothing. Their
objects do answer `MOUSE_LEFT_RELEASED` in the package data (for example the jukebox's Re-order group `0x104EC5` sends
`RE_ORDER`, the Back group `0x104E99` sends `PAD_BACK`).
**Ours:** `Runtime::mouse_target_at` finds the nearest visible object under the click that answers the release, or its
nearest ancestor that does. The screen posts the release to that object (`Screens::release_at`). A button still takes
the click first, as before (`focus_button_at`).

**EA Trax jukebox footer (ours).** The footer labels are not buttons either. The jukebox takes a click on a footer
label by its place: Back (`PAD_BACK`), Accept (`PAD_ACCEPT`), Re-order (`RE_ORDER`) and Preview (`PREV`), each a box
of 40 x 14 units from the label's origin. Leaving saves the jukebox, so a click on Back is not in the tests.

**Sources.** Ghidra: `"Credits.fng"` is pushed at `0x00532DF4` and `0x00545683`; `"CreditsArea"` is only a string
(`0x0089DB18`), not a referenced symbol. `speed.exe` has no function names, so the roll code was not located.
