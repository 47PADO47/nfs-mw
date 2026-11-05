# Pursuit HUD: the takedown line

What the original HUD tells the player when a cop car is wrecked in a chase. Provenance:
[../provenance/hud-pursuit-takedown.md](../provenance/hud-pursuit-takedown.md). Evidence tags: **[exe]** read in the
original PC 1.3 executable, **[install]** read in the player's install, **[inferred]** concluded from those.

## 1. Trigger

- The pursuit board receives, every HUD update, the chase's count of cop cars wrecked, the vehicle key of the latest
  one, its destruction bounty and the destruction multiplier it was wrecked at **[exe]**.
- The board keeps the count it saw last. A count above it starts the line; a count equal to it does nothing; a count
  below it (a new chase) only replaces the kept count **[exe]**. The kept count is replaced in every case, so two wrecks
  between two updates make one line, of the latest car **[exe]**.
- A wreck is counted only while a chase is on, and its bounty is the car's destruction points at the chase's heat
  (`DestroyCopBonus` points of the car's class) times the multiplier **[exe]**. The multiplier is 1 for a wreck after the
  destruction window ran out and one more, up to 3, for a wreck inside it; each wreck starts the window again
  (`DestroyCopBonusTime`, [ai-pursuit-heat.md](ai-pursuit-heat.md) section 5.1) **[exe]**.

## 2. Text

The line is one language label per cop vehicle, with the bounty put in **[exe]**:

| Vehicle | Label | English text **[install]** |
|---|---|---|
| `copmidsize` | `0xF49A550A` | `Civic Cruiser^Combo Bounty: %$d` |
| `copghost` | `0x902311DA` | `Civic Undercover Cruiser^Combo Bounty: %$d` |
| `copgto` | `0x9BFD379F` | `State Pursuit Vehicle^Combo Bounty: %$d` |
| `copgtoghost` | `0x65E97524` | `State Undercover Vehicle^Combo Bounty: %$d` |
| `copsporthench` | `0x4EE07213` | `Federal Pursuit Vehicle^Combo Bounty: %$d` |
| `copsportghost` | `0x4ED00512` | `Federal Undercover Vehicle^Combo Bounty: %$d` |
| `copsuv`, `copsuvpatrol` | `0x9BFD6AD3` | `SUV Heavy^Combo Bounty: %$d` |
| `copsuvl` | `0x1BAAC57F` | `SUV Light^Combo Bounty: %$d` |
| `copcross`, `copsport` | `0x8FE02B9F` | `Cross^Combo Bounty: %$d` |

- `^` breaks the line and `%$d` is the bounty (points times multiplier) with the thousands separator. The word "Combo"
  is in every label, so a single wreck also reads `Combo Bounty` **[install]**.
- Any other vehicle (traffic, the helicopter, a mod's car) has no line; nothing shows **[exe]**. A label the language
  table lacks shows nothing either **[exe]**. With the rewrite's bounty fixes on (the default,
  [ai-pursuit-heat.md](ai-pursuit-heat.md) section 5.5) a downed helicopter takes the `copsporthench` line.

## 3. Presentation

- The line goes to the HUD's generic message (`Hud_GenericMessage`, the group `0xE0BA07EC` whose child `0x32A7A521`
  plays the scripts and holds the two strings `0xDA768EA6`), the same object as the race's lines **[exe]** **[install]**.
- The group plays its script `0x8AB83EDB` (2400 ms) **[exe]** **[install]**. The race's lines in the rewrite keep the
  3200 ms script `0xA19BB14C` ([race-mode.md](race-mode.md) section 10).
- The icon beside the group (`0x6DD754EC`, `Cops_TakenOut_Icon.tga` as authored) takes the `Cops_TakenOut_Icon`
  texture and plays its script `0x0013FF94` (2400 ms), whose first key cues the glow (`0xCAEC9D04`) to play its own
  2400 ms script **[exe]** **[install]**. A line without an icon puts the icon at rest (`0x0016A259`) **[exe]**.
- Priority: the generic message takes a line only when its priority is at least the one of the line it shows; a line's
  priority ends when its script has played **[exe]**. The takedown has priority 2; the race-over lines have 5 and the
  other callers 1 to 5 **[exe]**. So a takedown does not cut the race-over line, and a second takedown replaces the first
  and starts its scripts again **[exe]**. The rewrite gives all of the race's lines precedence over the takedown (which
  caller has priority 1 was not mapped), and a new takedown starts the line again.

## 4. The rewrite

- `scenes/world/ai/police/rep.rs` keeps the chase's latest wreck (count, vehicle name, points, multiplier) beside the
  destruction points it banks; a new chase clears it.
- `scenes/world/feed/takedown.rs` diffs the count each frame as in section 1 and holds the line for 2.4 s;
  `hud/takedown.rs` picks the label and fills it (section 2).
- `hud/race.rs` plays the line's scripts (section 3). The Custom HUD draws the same line in its banner under the strip
  ([../decisions/0013-custom-hud.md](../decisions/0013-custom-hud.md)); its notices and chips are unchanged.
- Split screen: the original sends the line to the first player's HUD **[exe]**; the rewrite shows the lead's chase on
  the lead's HUD.

## 5. Open

- Whether the board's update can skip a frame of the count (it is called from the HUD's own update) was not timed.
- The look of the takedown icon and glow was read from their scripts, not captured from the original's screen.
