# Career text, part 2: messages, tips, tutorials and the shop unlock table

Continues [career-text.md](career-text.md). This page has the SMS id table (the 150-slot inbox of the save), the loading and
engage tips, the tutorials and help screens, and the reading of the save's "57 counters" as the shop's part-unlock tiers.
Method, tools and the label hash scheme are in the [provenance file](../provenance/career-text.md). The same rules apply:
**[exe]** read in `speed.exe`, **[verified]** read from the install or the saves, **[unconfirmed]** a guess. Only short
labels, names and ids are quoted; messages and tips are described in one line of the author's words.

## 1. The SMS system

### 1.1 Labels and storage **[exe + verified]**

- Each message has three labels: `SMS_MESSAGE_<n>` (the body), `SMS_MESSAGE_<n>_FROM` (the sender) and
  `SMS_MESSAGE_<n>_SUBJECT` (the list caption). The screens are `SMS_Mailboxes.fng` (the list) and `SMS_Message.fng`.
  Fixed captions: `SMS_MAILBOXES`, `SMS_FROM`, `SMS_SUBJECT`, `SMS_TEXT`, `SMS_VOICE`, `SMS_INCOMING`, `SMS_MOBILE_MESSAGE`,
  `SMS_REPLAY`.
- The save keeps 150 four-byte slots ([career-save.md](../formats/career-save.md) 3.2). A slot's id byte is its own index
  when the table has `SMS_MESSAGE_<index>`, otherwise 0xFF. The table has **95** messages (ids 10 to 137 with gaps), and
  the three saves have exactly the same 95 valid slots. **[verified]**
- Slot states: 0 not delivered (or deleted), 2 delivered and unread, 4 read (set when the player opens the message in the
  inbox). Delivery sets state 2 and gives the slot the next sequence number (the saved counter). Messages sent with
  `EAddSMS` are queued and all delivered together on the first update in which the game is in free roam with no race,
  overlay, call speaking or pursuit; the ids 10 to 14, 95 and 96 are the exception: `EAddSMS` delivers them at once and
  starts a call, but no script sends them that way (they arrive as `ECellCall` and are only written into the inbox in the
  five text languages). The full rules are in [career-phone.md](career-phone.md). **[exe + verified]**
- The text/voice distinction: the tip says people send both text messages and cell-phone messages (voice). Messages
  from `Rockport PD` and the empty sender `-` (id 122) are system notices.
- Ids 110, 111, 120 to 130 and 132 to 137 are treated as non-story messages: they have their own tab in the inbox, no
  voice line, and the message screen hides two widgets for them **[exe]**: tutorials, impound notices and two extra
  Earl/Razor texts. 13 of the 95 messages are never sent in this build (16, 21, 32, 47, 50, 74, 97, 98, 105, 110, 111,
  122, 129); see career-phone.md section 5.3.

### 1.2 Who sends what

The sender names are Rog (45 messages, the contact who briefs the player), Mia (26), Razor (11), Cross (4), Vic (3), Ronnie (2), Earl (2),
the police (1) and one message with no sender. Bin entry and rival unlock messages are in the rival
table of [career-text.md](career-text.md) section 2. The `Subject` column is the list caption.

| Id | From | Subject | Id | From | Subject |
|---|---|---|---|---|---|
| 10 | Mia | Oil slick. | 69 | Rog | Kaze's a nut job. |
| 11 | Razor | To crack the Blacklist. | 70 | Mia | Real buzz goin'. |
| 12 | Rog | Hey night timer! | 72 | Rog | Ming wants to roll. |
| 13 | Rog | Nice run. | 73 | Rog | #6, how does it feel? |
| 15 | Mia | I'll meet you there. | 74 | Vic | Free ride. |
| 16 | Rog | People call me Rog. | 76 | Rog | Pencil neck. |
| 17 | Razor | Thanks for the wheels! | 77 | Razor | #5 on the Blacklist. |
| 18 | Rog | If you need parts... | 79 | Rog | JV is popular, take him out. |
| 19 | Mia | About Razor... | 80 | Rog | Razor's runnin' scared. |
| 20 | Rog | Sonny's lookin' for a run. | 81 | Razor | NARC. |
| 21 | Rog | The regs for racing the Blacklis.. | 82 | Mia | Stay on top of your game. |
| 22 | Rog | Some info you might need. | 84 | Ronnie | I'm gonna wipe you all over the.. |
| 23 | Rog | Pursuit milestone info. | 85 | Mia | Cracked the top 3. |
| 24 | Vic | Stay out of my way and we're cool. | 86 | Razor | Turning the tables. |
| 25 | Mia | Straight up. | 88 | Rog | Toru's waiting for you. |
| 26 | Mia | Inside scoop on the cops. | 89 | Rog | How does it feel? |
| 27 | Rog | Pursuit breakers. | 90 | Razor | You're sellin' us out. |
| 29 | Rog | You're buildin' hype. | 91 | Mia | You're nearly there. |
| 30 | Rog | Taz didn't see that comin'. | 93 | Rog | Race between you and Razor is lo.. |
| 31 | Mia | Camden Beach. | 94 | Razor | It's time! |
| 32 | Vic | Making life interesting for ya. | 95 | Cross | Make a deal. |
| 33 | Rog | Mia's making some cash. | 96 | Mia | Way out. |
| 35 | Rog | You wanna see the ocean? | 97 | Rog | Impounded. |
| 36 | Rog | Vic's ready to race. | 98 | Rog | Busted. |
| 37 | Mia | Nice job. | 100 | Cross | No special treatment. |
| 38 | Mia | Cross has your file. | 101 | Cross | Cuttin' you some slack. |
| 40 | Mia | Stay on target. | 102 | Cross | Pin stripes. |
| 41 | Rog | Izzy wants to go for it. | 105 | Rog | Backroom markers. |
| 42 | Rog | Nice job finishing Izzy. | 110 | Earl | My Streets. |
| 43 | Mia | Police chopper info. | 111 | Razor | Rip you up. |
| 46 | Rog | Big Lou wants to lock a race. | 120 | Rockport PD | Impound release notification. |
| 47 | Mia | Limited fly zones. | 121 | Mia | Hurry up. |
| 48 | Razor | Things are gonna get rough. | 122 | - | Released From Impound |
| 49 | Rog | Someone's spreading rumors about.. | 123 | Rog | Working your way up the Blacklist. |
| 50 | Mia | Stick to the plan. | 124 | Mia | Basic rules for evading the poli.. |
| 52 | Rog | Baron's dying to race you. | 125 | Rog | New cars and parts. |
| 53 | Mia | Great run. | 126 | Rog | Taz put out the challenge. |
| 54 | Mia | Top 10. | 127 | Rog | Get more than one car. |
| 55 | Earl | Whole lot of green. | 128 | Rog | Use pursuit breakers wisely. |
| 57 | Rog | #9 doesn't think he can take you. | 129 | Mia | Checkout the Rap Sheet. |
| 58 | Mia | Way to go! | 130 | Rog | Speedtrap race pointers. |
| 59 | Razor | That Rog guy. | 132 | Mia | Getting  your heat down. |
| 60 | Rog | Razor's paying people off to tak.. | 133 | Rog | Tip on roadblocks. |
| 61 | Mia | This Blacklist crew is organized. | 134 | Rog | Earning extra cash. |
| 63 | Rog | Jewels is ready to race. | 135 | Rog | Performance tuning might help. |
| 64 | Ronnie | Hey tough guy. | 136 | Rog | Next level of police. |
| 65 | Mia | People aren't taking Razor's bri.. | 137 | Rog | The regs for racing the Blacklis.. |
| 66 | Razor | It really is you against the wor.. |  |  |  |

Groups (from the captions and the rival table): 10 to 27 the prologue and first rivals' briefing (rules of racing a
Blacklist member are ids 21 and 137, the same caption); 28 to 94 the climb, with one message per rival at its entry and
unlock; 95 to 102 Cross (the deal offers and his threats); 97 and 98 reactions to impound and bust; 105 the backroom
markers; 120 to 122 the impound release notice; 123 to 136 the help series (rank climb, evasion basics, new cars and parts,
owning several cars, pursuit breakers, rap sheet, speed-trap races, cooling heat, roadblocks, extra cash, performance
tuning, the next level of police). **[verified captions; the sending rules for ids outside the bin table are not
traced, [unconfirmed]]**

## 2. Tips

### 2.1 The engage and loading tips **[exe + verified]**

There are 24 tips; each is a pair `TIP_<KEY>_HEADER` / `TIP_<KEY>_DESC`. The loading screen table in the exe (27 tip
entries, index 0 to 26, of 16 bytes: a key pointer, a context mask word, 0x10, 0x200) lists them as below. Entries 0 to 2
have a zero mask word and a different last word (0x400); their role is unknown. The mask word (for example 0xE000 for the safe house tip) probably
selects the Blacklist ranks or game states in which a tip may appear; it was not decoded **[unconfirmed]**. Entry 27 is
`GAME_TIP_USE_CONTROLLER_CONFIG` (not a career tip).

| Table index | Tip key | Subject (own words) |
|---|---|---|
| 2, 18 | `WORLDMAP` | the world map lists events, shops, safe houses and photo-ticket milestones; use it to plan escapes |
| 1, 5 | `ENGAGE_MECHANIC` | the shop and race icons turn on the engage system near them |
| 0, 26 | `STRIKEOUT` | each bust adds an impound strike to the car; at most 5 with the extra-strike marker |
| 3 | `CUSTOMIZATION_SHOPS` | the One Stop Shop; NEW icons show categories unlocked by the last rival; looks lower heat |
| 4 | `CARLOTS` | every region has a car lot; new cars unlock with each rival; you need cash |
| 6 | `SMS` | texts and cell messages arrive through the engage system |
| 7 | `UNLOCKS` | parts and regions come only from beating rivals |
| 8 | `SPEEDBREAKER` | slows the world down |
| 9 | `ROADBLOCKS` | roadblocks have weaknesses |
| 10 | `SUVRAM` | the heavy SUV units ram |
| 11 | `CROSS` | at high heat Sergeant Cross leads the chase |
| 12 | `HELICOPTER` | the police have a helicopter |
| 13 | `ROLLING_ROADBLOCK` | cops can form a rolling roadblock |
| 14 | `BOX_FORMATION` | cops can box the player in |
| 15 | `PURSUIT_BREAKERS` | key to evading |
| 16 | `HIDING_SPOTS` | hide once the police lose sight |
| 17 | `BUSTED` | pay all infractions for the car, take a strike; a marker cancels the bust |
| 19 | `RIVAL_CHALLENGES` | each rival needs race wins, milestones and bounty before the challenge |
| 20 | `MARKER_REWARDS` | beating a rival gives a choice of two markers, one kind may be the pink slip |
| 21 | `BLACKLIST` | the Blacklist tracks progress; open it from engage or any safe house |
| 22 | `BOUNTY` | bounty comes from milestones, from time in a pursuit (faster at higher heat) and from immobilizing police |
| 23 | `CHALLENGE_SERIES` | a change from the career |
| 24 | `RAP_SHEET` | the police track statistics and rankings; open it from the safe house |
| 25 | `SAFEHOUSE` | several safe houses; switch cars, check the Blacklist, open the police database; a good place to cool down |

(`WORLDMAP`, `ENGAGE_MECHANIC` and `STRIKEOUT` occur twice in the table with different mask words.) **[verified]**

### 2.2 Help screens and tutorials **[verified]**

- `helpscreen_title_<x>` / `helpscreen_text_<x>` for: backroom, carlot, challenge_series, controllersettings, drag_race,
  impound, infractions, ko (knockout), markers, mycarsmanager, nos, onestopshop, quickplay, rapsheet, safehouses,
  shoppingcart, speedbreaker, speedtrap_race, stabilitycontrol, tollbooth_race, worldmap (21 pairs), plus `HELP_BOUNTY`,
  `HELP_MILESTONES` and `HELP_RACE`. The table also has `helpscreen_text_cash_run` and `helpscreen_text_rapshet`
  without a title entry.
- `TUTORIAL_<x>_LABEL` and four lines (`PAGE1_A`, `PAGE1_B`, `PAGE2_A`, `PAGE2_B`) for five in-game tutorials: `PURSUIT`,
  `BOUNTY`, `TOLLBOOTH`, `SPEEDTRAPRACE` and `DRAG` (the exe names them; they are shown by the `PlayTutorial` hook). The
  tutorial subtitle files use labels that are not in the table ([text.md](../formats/text.md)).
- The end-of-game screen (`GameWon.fng`) uses the `EG_*` labels (Most Wanted warrant, three reward lines: bonus cars, the
  Challenge Series and rap sheet rankings). Achievement labels `ACH_*` (28 achievements, 84 labels) are the online-service
  list: ranked race counts, car counts, career completion, the epic pursuit goals, and one per rival (named by the
  rival's first name, bin 9 by the developers' `Pierre`).

## 3. Other career labels worth knowing

| Group | Labels |
|---|---|
| Rep sheet / reputation | `REP_*` (events, milestones, garage, game complete, next rival, race and bounty headers) |
| Post-race / pursuit | `PRP_*` (pursuit summary), `PRR_*` (race results), `PRS_*` (race stats), `RACE_STATS` |
| Challenge Series | `CHALLENGE_SERIES_DESC_<01..44>_HEADER` (event kind names: pursuit length, bounty, evasion, cost to state, roadblock, trade paint, infractions, spike strip, plus a burger-chain challenge and the tollbooth trials) and `_DESC_<31..44>`; `CHALLENGE_SERIES_UNLOCK_MSG_01/02`; not part of the career save |
| Shopping | `SCART_*` (cart, trade-in, funds, total), `CD_*` (checkout dialogs: not enough money, part already installed, replace cart part), `CT_*` (performance tuning), `CO_*`, `CP_*` |
| Debug | `DEBUG_OPT` |

## 4. The shop unlock table (the save's "57 counters") **[exe + verified]**

[career-save.md](../formats/career-save.md) 3.2 listed 57 entries of 8 bytes as "hint counters". They are not hints. The
index is a **part category id** and each entry holds the highest unlocked **tier** of that category for the career (first
4-byte half) and for the other modes (second half).

Entry (per half): `{u8 tier, u8 new, u8 views, u8 0}`.

- `tier` is the highest unlocked tier; a part of tier `t` is available when `t <= tier` (also when the career-completed
  byte is set, when settings flag bit 14 is set, or when the debug flag is on).
- `new` is the tier just unlocked, or 0xFF when no NEW badge is pending. A reward sets `tier` and `new` of both halves
  to the reward's tier. Each time the shop shows the badge, `views` goes up; past 3 it resets `new` to 0xFF and `views`
  to 0. This is the NEW icon that `TIP_UNLOCKS` and `TIP_CUSTOMIZATION_SHOPS` describe.
- A fresh career has `{0, 0xFF, 0, 0}` for the first half and `{0, 0xFF, 2, 0}` for the second, as the two fresh saves
  show; the late-game save has `{tier, 0xFF, 1, 0}` in both halves with the tier at or near the maximum. **[verified]**

Category ids (from the exe's part-name table) and the maximum tier the game defines for them:

| Id | Part group | Max tier | Id | Part group | Max tier |
|---|---|---:|---|---|---:|
| 4 | tires | 3 | 14 | hood | 6 |
| 5 | brakes | 4 | 15 | roof scoop | 6 |
| 6 | chassis (suspension) | 3 | 17 | custom gauges | 4 |
| 7 | transmission | 4 | 18 | window tint | 4 |
| 8 | engine | 4 | 23 | paint | 3 |
| 9 | induction (turbo, supercharger) | 3 | 40 | vinyls | 6 |
| 10 | nitrous | 3 | 43 | numbers | - |
| 11 | body kit | 4 | 44, 46, 48 | decal slot 1, 2, 3 | - |
| 12 | spoiler | 5 | 50 | decals | - |
| 13 | rims | 6 | | | |

The performance package descriptions (`PD_<GROUP>_<tier>_<line>`) match the first rows: brakes 4 tiers, engine 4,
transmission 4, and chassis, nitrous, supercharger, tires and turbo 3; each group also has a `_JUNKMAN` description (the
Junkman's custom package). The reward kinds that feed this table are the "backroom" kinds of the reward table: the part
name picks the category and the reward's own number is the tier. A `backroom` reward for decals with number 1, 2 or 3
picks the slot 44, 46 or 48. **[exe]**

## Open questions

1. Which game events send each SMS id outside the bin table, and the meaning of slot state 4.
2. The mask word of each loading tip.
3. Whether the second half of a shop entry is the quick-race table or the online table.
