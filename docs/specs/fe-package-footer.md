# In-game screens: the package footer, the dim panel, the message screen and the inbox key

How the original draws the footers of the screens it opens over the game (the win photo, the race results, the SMS
inbox and message, the car select), what it puts under them, and how its message screen plays a voicemail. Evidence:
**[exe]** read in the executable (method and addresses in [../provenance/fe-package-footer.md](../provenance/fe-package-footer.md)),
**[live]** read in the running original through the front-end screen dump ([../testing.md](../testing.md) "Front-end
screen dump"), with the number of runs. The remake's code: `crates/nfsmw/src/frontend/prompts/package_footer.rs`,
`frontend/screens/pause_background.rs`, `frontend/career_screens/sms_message.rs`, `audio/cell_call.rs`,
`frontend/flow_sms.rs`.

## 1. Two copies of every footer button

A package with a footer holds each button twice, side by side on the same line:

- the **keyboard copy**: a group with object flag `0x8`, holding a caption, a black box (`0xC6D91A27`), a hatched strip
  (texture `0x397A3A31`) and a `PC_key_*` picture (`PC_key_enter`, `PC_key_esc`, `PC_key_1`, `PC_key_2`, `PC_key_3`);
- the **pad copy**: a group with object flag `0x40`, holding a caption, its shadow and a console button picture
  (`Cross`, `Circle`, `Square`, `Triangle`; the pictures carry flag `0x10`, the theme colour).

**[exe]** When a package starts, every object with flag `0x40` and everything below it is made invisible; the call that
makes an object visible again refuses an object with flag `0x40`. **[live, 1 run each]** With the keyboard in use the
keyboard copies are drawn and the pad copies are not: win photo, race results, car lot (`Car_Select.fng`). After a pad
press the pad copies are drawn and the keyboard copies are invisible (they and their children carry the invisible bit):
inbox and message, also when the message was then opened with Enter. Which code switches to the pad copy is not found
(open question 1). The pad copy's pictures are swapped at run time for button art of the action they stand for
(`0xC7162596` Accept, `0xC7162597` Back, `0xC71625AD` button 4, `0xC71625AE` button 5, by the action, not by the console
picture the package names); the texture names are not resolved.

**[live]** The keyboard copies are drawn from the start (none waits for the package's own reveal event), and so are the
other keyboard-flagged objects of these packages (the car lot's two side panels `0x46CB7764`, `0xD276BA2F`), except an
object other than a group whose own invisible bit (bit 0) is set; a group's bit is not tested.

**The remake [ours: presentation].** On these screens the presented tree keeps the package's footer: on the keyboard the
pad copies (flag `0x40`) are taken out, on a pad the keyboard groups that hold a `PC_key_*` picture; on a pad the pad
copy's pictures show the remake's glyph of the bound button (its own atlas, not the original's textures). The runtime
starts keyboard-flagged objects hidden, so these packages show them when they load, before their logic runs. A click on a
drawn button presses its action. Buttons the screen's state rules out are hidden whole: a speed trap's photo has no Race
Stats and Restart, a message that is not a voice message has no Re-Play (**[exe]** the message setup hides both copies),
the results' online buttons (Gamer Card, Feedback) never show, a locked car has Back only, and the career car select has
no Showcase or Sell. The career car select while it offers a strike marker keeps the host's hint row (the package has
no keyboard button for the marker). Every other screen keeps the host's hint row (`prompts::decorate`).

| Screen | Buttons (keyboard group, pad group, action) |
|---|---|
| `InGamePhotoMaster.fng` | Continue `0x6CD6A337` / `0x812A09D4` accept; Race Stats `0xD6463141` / `0x5CED1D04` button 4 (key 2); Restart `0xC0ACB81A` / `0x6A218478` button 5 (key 1) |
| `PostRace_Results.fng` | Exit `0x9502D148` / `0xB36CC7CB` accept; the page `0x3FC72F9D` / `0x5CED1D04` button 4; Restart `0xB9A5851F` / `0x6A218478` button 5 |
| `SMS_Message.fng` | Done `0x523250E9` / `0x1C7FCF8B` back; Delete `0xC1C190EB` / `0xD18B818D` button 5 (key 1); Re-Play `0x914614E5` / `0x2A631207` button 4 (key 2) |
| `SMS_MailBoxes.fng` | Read `0x9502D148` / `0x23AFB3F6` accept; Done `0x523250E9` / `0x1C7FCF8B` back; Delete `0xC1C190EB` / `0xD18B818D` button 5 |
| `Car_Select.fng` | Back `0xC0A32823` / `0x02DDF58C`; Accept `0x73DCB662` / `0x7379349B`; Showcase `0x18A4384F` / `0x4B890CDB` button 3; unused Sell `0xD6463100` / `0xD42F677B`, Load Cars `0x5F34A28C` |

## 2. The dim panel under screens opened over the game

**[exe]** The in-game push of a screen takes a flag that skips the panel. While the game runs (game flow state 6), a
push without that flag records the screen and, unless `InGameBackground.fng` is up, pushes it first, directly under the
screen; only the first such push of a stack records (a second screen over the first pushes no second panel). Closing
pops the stack; the record is cleared when the stack is empty. Of the 44 push sites only the win photo's two pass the
flag. **[live]** The panel is drawn under the inbox and an opened message (1 run each), the results opened from the win
photo (it sits between the photo and the results), the SixDaysLater card, the pursuit result and rewards screens; not
under the win photo or the world map (which is opened another way).

**The HUD [exe + live].** The same in-game push calls every registered HUD (the HUD object registers itself in a list
when it is built) with an element mask of 0, which hides every HUD widget; `HUD_SingleRace.fng` is still walked each
frame but draws nothing. Read live (session `sms-unread3`, 1 run, draw counts per package): under the inbox and the
message `HUD_SingleRace`, `FadeScreen` and `Autosave_Overlay` draw 0 objects, `InGameBackground` its 3 panels. The win
photo skips this push, so its HUD is not hidden (drawn under the photo).

**The panel's width [ours].** The panel's side strips fade from about 200/255 black at x = +/-255 to clear at x =
+/-509. The original draws its whole 640 x 480 front end stretched to the window (install behaviour, not read in code),
so at the window's edge (x = +/-320) the panel is still mostly dark. The remake keeps the FEng aspect for its screens and
shows more width on a wide window; it stretches this one package to the window width, so its edges darken as the
original's do.

**The remake.** The flow marks the stack as opened over the game before it opens a screen over the frozen game (results,
photo's Race Stats, pursuit and rewards screens, the bust ticket, the career's in-game screens, the SMS popup, the inbox
key, the pause menu); the panel goes directly under that screen unless it is the win photo, and leaves when no screen is
above it. The pause menus get it as before. The frozen world hides its HUD (as it already did while paused), which
matches the original under these screens; under the win photo the original keeps its HUD (not matched, the photo
covers it).

## 3. The message screen

- **Voice [exe].** The screen's handler queues an unread voice message's cell call when the package says the screen is
  up (its init message, sent by the package's own handler script 500 ms after the start) and marks the message read;
  button 4 (Re-Play) stops speech and queues the call again; the exit message (`EXIT_COMPLETE`) stops speech and closes
  the screen. **[live, 1 run]** The call was queued 0.43 s after the screen was built, while it was up. There is no
  Accept branch: the message `0xD05FC3A3` the handler answers is the Yes of its delete question (it empties the slot and
  sends Back to the package). Accept on the focused Done button goes through the package's Done button.
- **The remake.** The message is marked read and the voice queued on the init message; the speech queue runs while a
  message screen is up although the game is paused, so the voice sounds at once, and the line stops when the screen
  closes.
- **Scroll bar [live, 2 messages].** While the body fits the window the original keeps the scroll bar's thumb
  `0x3FBB7A4E` and track `0x688A1551` on the package's `HIDE` script (alpha 0). The remake runs `HIDE` on both while the
  body fits and `SHOW` for a longer body.

## 4. The message delivered while driving, and the inbox in free roam

**[exe]** Delivering a pending message while driving creates the show-SMS event with the message's id; it pushes
`SMS_MailBoxes.fng` (the inbox) through the in-game push with that message's slot as argument (an id of -1 pushes the
list alone). The inbox, given a slot, selects that message (its tab by voice or text) and marks itself opened-at-a-
message; when its package reports it is up (the init message, 500 ms in) the next handler call runs the `READ` script on
the inbox's EVENT HANDLER (buttons off at once; at 500 ms `LIST_OUT`, which takes the list and tabs out, and
`EXIT_COMPLETE`) and pushes `SMS_Message.fng` with the slot. So the delivery popup is the message over the inbox:
**[live, 1 run, inbox route]** under the message the inbox draws 12 objects, the carrier logo `0xFFBA97A9`, the four
corner brackets (`0x475A90F5`), the header bar `0x5FFE9A68` and its hatched panel `0xF063760C`, `...INCOMING` /
`MOBILE MESSAGE`, and the footer bar `0x81B4FAD0` with its stripe `0x397A3A31` behind the message's buttons (plus the
event handler string). The message's Back pops only the message (back to the inbox); the inbox's Back closes the stack.
The inbox sends `HIDE_INCOMING` to itself under two global conditions (not read further).

**The remake.** The delivery opens the inbox with the message pending; the inbox opens it on its init message after
running `READ`, as above. Done returns to the inbox, whose Back resumes driving.

**[exe]** The HUD's pad handler, outside races, opens the inbox (`SMS_MailBoxes.fng`, the list) on its right pad action
(`HUDACTION_PAD_RIGHT`); its left action opens the world map, its down action the race sheet. The default keyboard table
gives these actions the scan codes `0x0F` (Tab), `0x32` (M) and `0x30` (B). **[live, 1 run]** The pad's d-pad right in
free roam opened the inbox 0.5 s later, over the panel and the HUD.

**The remake.** `Action::Inbox`, Tab and d-pad right: in free roam outside a race, with a career loaded, the inbox opens
over the frozen game; its Back resumes the game, a message opened from it closes back to the list. The remake's own
Custom HUD map had Tab (map mode) and d-pad right (map orientation): map mode moved to U (d-pad down stays) and map
orientation keeps O only.

## 5. The win photo's texts **[live, 1 run]**

The cash line reads `CASH: 10,000` (the word, a colon, the amount with thousands separators); the race time line reads
`1:27.88 @ 105 MPH`, the time as minutes (two wide), seconds and hundredths. The photo's keys: 2 opens the results
(Race Stats), 1 the Restart question. On a pad the owner's pictures of the original (modded install, Xbox pad) show
button 4 as X and button 5 as Y (Restart, Delete); the remake binds menu button 1 (button 5) to Y first, the left
shoulder second. The footer container keeps its first script (no other is run on it).

## 6. Open questions

1. The code that shows the pad copies and hides the keyboard copies after a pad press, and how the game decides that the
   keyboard is in use again.
2. The names of the run-time button textures `0xC71625xx`.
3. The original's pad copy on the results and the car select (only keyboard captures exist), and the marker offer on the
   career car select.
4. Whether the original keeps the HUD under the pause menus (not captured; the pause menu goes through the same push, so
   probably hidden, as the remake does).
5. The two conditions of `HIDE_INCOMING`; the original's delivery popup itself is not captured (the inbox route is).
6. The panel's darkness: the remake's centre is lighter than the owner's original picture (modded install, other
   lighting); an unmodded picture of the same moment would settle it.
