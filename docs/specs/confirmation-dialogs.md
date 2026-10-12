# Confirmation dialogs and game strings

Status: implemented. The translated-string lookup (`ui/strings.rs`), the confirmation model and its screen logic
(`frontend/confirm.rs`), `Command::Confirm` (opened by `Screens`) and the pause menu's "Quit to Main Menu" all
exist. The main menu's own Quit still quits at once (section 6). Read from `speed.exe` (v1.3,
the stripped build in Ghidra, image base `0x00400000`). Addresses are as Ghidra shows them. Tags follow
[frontend-menus.md](frontend-menus.md): **[decomp]** read from decompiled code, **[verified]** checked against
the install data, **[ours]** this rewrite's choice.

## 1. Translated game strings [decomp, verified]

- `FUN_0056bb80(this, label)` takes a label hash and returns the text of the loaded language. Callers pass a
  fallback label when the first lookup fails: `FUN_00598c00` retries with `0x9bb9ccc3` ("FENG: Default string
  error.") when the requested label is missing.
- The table is `LANGUAGES/English.bin` (the chunk `0x00039000`, see [text.md](formats/text.md)). The rewrite
  reads it with `blackbox_text::StringTable`.
- **[ours]** `ui::game_string(assets, label)` is that lookup. `ui::Text::Game(label)` and
  `ui::Text::Custom(text)` are the two forms a screen takes.

Check a label with the CLI: `nfsmw strings 0x075cac3b` prints the text. `nfsmw strings "quit"` finds labels by
text.

## 2. The quit confirmation [decomp, verified]

The original does not quit straight away. Pressing the Quit button sends its release message `0x62799A4C` to the
package's screen logic, `FUN_005a4820`. That function checks four things, and when they hold it raises the
dialog:

- the game mode is `3` (`DAT_00925e90 == 3`),
- the screen is not a loading or movie package (`FUN_005864b0`: not `EA_TRAX_JUKEBOX`, `MC_MAIN`,
  `FEANYMOVIE`, `LS_PSA`, `LS_EALOGO`, `MW_LS_ATTRACTFMV`, `MW_LS_SPLASH` or `LOADING`),
- the screen's own flag at `+0x28` is set,
- the screen is the top of the stack.

It then calls `FUN_00598c00` with these labels:

| argument | value | English text |
|---|---|---|
| message | `0x075CAC3B` | "Are you sure you want to quit? Any unsaved changes will be lost." |
| yes | `0x70E01038` | "Yes" |
| no | `0x417B25E4` | "No" |

Other labels in the same call (`0x34DC1BCF`, `0xBE34E004`) are the answers' messages; their exact roles are still
open (see section 5). The rewrite maps the message and answers to `ui::Text` and runs the chosen command on yes.

**Which dialog package:** `FUN_005983c0` picks the package from the dialog's name string: `animating` gives
`OL_Dialog.fng`, `3button` gives `OL_Dialog_Stacked_Buttons.fng` (or the given name when it is `3buttons_stacked`),
and anything else gives `Dialog.fng` [decomp]. `FUN_005983c0` copies the dialog's 0x90-dword parameter block into
the global dialog slot `DAT_008f5128`, then opens the package through `FUN_00542530` (or `FUN_005257f0` when the
platform flag `DAT_008f5328` is not 4). The quit path ends up in `Dialog.fng`, which is installed in
`FRONTEND/FrontB.lzc` with 3 buttons and these resources: `FONT_MW_BODY`, `FONT_MW_TITLE`, `Generic_Alert.tga`,
`Generic_Info.tga`, `BaseStripe.tga`, `BasePoly.tga`, `Crosshair_Corner.tga` [verified, `nfsmw dump-screen Dialog.fng`].

## 3. The API the rewrite gets

- `ui::Text::{Custom, Game}` with `resolve(&UiAssets) -> Option<String>`. `Custom` is for a caller that brings its
  own wording; the game's own dialogs use `Game`.
- `frontend::confirm::Confirm { message, yes, no, on_yes }`, with `Confirm::new(message, on_yes)` (game Yes and No)
  and `Confirm::quit_to_menu()` (the original quit question). `on_yes` is the command the screen runs on yes.
- A screen asks with `Command::Confirm(confirm)`. `Screens` opens `Dialog.fng` on top with `ConfirmLogic`
  (`open_with`, the logic chosen by the caller), and the answer pops the dialog and sends `on_yes` on yes.

## 4. How the rewrite shows the dialog [ours, verified in the game's package]

The package's objects, by name hash (`nfsmw dump-screen Dialog.fng`):

| object | name hash | role |
|---|---|---|
| message text | `0x1E2640FA` | the question |
| Button 1 text / holder | `0xF9363F30` / GUID `0xF70E8` | the middle answer (Yes here) |
| Button 2 text / holder | `0xFB8B67D1` / GUID `0xF72C2` | the right answer (No here) |
| Button 3 text / holder | `0xFDE09072` / GUID `0xF731F` | the left answer, hidden |

- Each holder has a `BUTTON_PRESSED` response that sends the game `0x1B91EBF4` (the answer message) and focuses
  its own text. A press is therefore one message; which answer it was is the package's focus
  (`Runtime::current_button`). [decomp of the package's responses]
- The package sends the game Left (`0x9120409E`), Right (`0xB5971BF1`), Back (`0x911AB364`) and `INIT_COMPLETE`
  (`0x35F8620B`). It does **not** send accept: accept goes to the focused holder.
- The package's own left and right do not move the focus between the answers in the runtime (a probe with the
  focus on No stayed on No), so `ConfirmLogic` moves it on Left and Right with `Runtime::focus_button`.
- The package sets its own focus after `start`, when it sends `INIT_COMPLETE`; the logic sets No again then. So
  No is selected when the dialog opens and a stray accept cannot quit.
- Back answers No.

Tests: `frontend::tests::the_pause_menu_resumes_opens_the_options_and_quits` (No keeps the pause menu, Left then
accept quits) runs against the install with `NFSMW_GAME_DIR`.

## 5. Open questions

- Where the Yes and No buttons sit on screen (Yes in the middle, No on the right) is the rewrite's choice. The
  original's two-answer layout is not checked against a screenshot yet.
- The answer's focus shows no highlight; the package's own highlight script runs only when the package moves the
  focus itself, so the highlight may be missing on the answers this logic moves.
- The original quit call also carries `0x34DC1BCF` and `0xBE34E004`. `0xBE34E004` sets the flag
  `DAT_009257EC` in `FUN_005a4820`; its readers are not traced yet.

## 6. Main menu Quit [decomp, not verified in play]

The main menu's Quit sends the same release message `0x62799A4C` as the pause menu's Quit. `FUN_005a4820` raises the
question only when the game mode is `3` (in play). In the menus the mode is not 3, so the original most likely quits
at once, and the rewrite's main-menu Quit (`QuitGame`) is left as it is. To confirm: press Quit in the main menu
in the original and check it does not ask.

## 7. Verification and status

- **Layout, in play [verified]:** `nfsmw play --drive --ui-script "wait 3;start;wait 2;right;right;accept;wait 3"
  --screenshot … --screenshot-delay 12` renders the question with the original text, Yes on the left, No on the
  right, focus on No, and the middle button hidden. The title is empty: the original's quit call passes an empty
  header (`&pszFaceName_00890978`), so `Confirm::title` defaults to `None` and the header text is cleared. Before
  this, the package's placeholder "populate me" showed.
- **Behaviour, by test:** `frontend::tests::the_pause_menu_resumes_opens_the_options_and_quits` (No keeps the pause
  menu; Left then accept quits) against the install.
- **Open:** `0xBE34E004` and `0x34DC1BCF` (section 5) are not traced.
- **Main-menu Quit [unverified]:** the main menu's Quit is a mouse-release message (`QUIT_CLICKED`), and the
  script language has no key token for it, so it can't be driven in play here. The decompile reading (section 6)
  is not confirmed by running the original.
