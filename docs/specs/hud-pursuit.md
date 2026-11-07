# The pursuit HUD

The current integration is recorded in [pursuit-system](../pursuit-system.md).
The detailed radar and signed meter contract is
[hud-pursuit-state](hud-pursuit-state.md). The chase simulation is described in
[ai-pursuit](ai-pursuit.md) and [ai-pursuit-heat](ai-pursuit-heat.md).

## Sources and evidence

- **[verified]** Installed `HUD_SingleRace.fng` in `GLOBAL/InGameB.bun`:
  object identities, parentage, original sizes, pivots, scripts and messages.
- **[decomp]** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0,
  `Frontend/HUD/FePursuitBoard.cpp`, `FeRadarDetector.cpp` and `FeHeatMeter.cpp`.
  The implementation follows the behavior contract; no decompiled code or
  extracted game data is shipped.
- **[binary]** The PC radar update and target feeder confirm the two backing
  states, 300-metre 3D range, 45-degree forward cone and per-update LED ramp.
  The pursuit-board update confirms the signed segment formulas and anchors.
  Matched gameplay captures and timing/listening comparisons remain necessary
  before claiming original-game visual and behavioral parity.

## Bound objects

| Object/hash | Current behavior |
|---|---|
| `PursuitBoardGroup` / `0xde89e070` | Board shown while the pursuit feed exists, including the arrest display. |
| `PursuitTime` / `0xfc39cb0a` | Elapsed time in `mm:ss.cc`. |
| Cops group / `0xe0b1430b` | Live count, damaged count and destroyed count. **[verified]** The package fades the group: the meter's `APPEAR` and the cool-down group's exit (`0x92975065`) raise its `APPEAR`; the meter's exit into the cool-down raises its `LEAVE`, so it is gone while the cool-down bar and its hiding icon (`0x38074901`, authored over the engaged count) show. The host must not hold its alpha. Ground engagement counting still needs original comparison. |
| Engaged / `0x814918ca` | Ground units reported by the world; no template `99`. |
| Damaged / `0x5fa70d4c` | Distinct damaged police vehicles in the current pursuit. |
| Destroyed / `0xa16f9f1e` | Police vehicles destroyed in the current pursuit. |
| Meter group / `0x7b422ba3` | Signed busted/evade meter with authored entrance and cooldown transition tracks. |
| Five meter images / `0xe0e0169b`, `0x8eeebd33`, `0x47a4e2a9`, `0x8e3653a6`, `0x30f39a6f` | Piecewise widths, fixed outer edges and flashing states from the signed display value. |
| `Cooldown_Meter_Group` / `0x84a226ec` | Shown on actual cooldown state, independently of fill fraction. |
| Cooldown fill / `0xcf817638` | Installed width times cooldown progress, keeping its left edge fixed. |
| Hiding bar / `0x5c2a4f20` | Its appear and leave by the board's rule; its messages fade the "HIDING" group, label and icon ([pursuit-hiding](pursuit-hiding.md) section 5). |
| `Heat_Meter_Group` / `0xc46a80a9` | Retained vehicle heat, including outside a pursuit. **[binary]** The HUD update feeds the meter the player vehicle's live heat every frame, in and out of a chase. That heat starts at `max(1.0, record heat)` in the career (`career-safehouse-2.md` 8), so career free roam reads the car record through that floor and shows x1 for a cooled, cleared or new car. |
| Duplicate heat strings / `0xa81a2028` | Both string objects updated to the integer multiplier. |
| Radar backing / `0x839e7d77` | Standalone layout selected through authored `APPEAR`; hidden when the mirror supplies the backing. |
| Mirror backing / `0x9ee06631` | Mirror layout selected with `APPEAR`; retained during pursuit when enabled. |
| `RADAR_GROUP` / `0x062743f5` | Authored `INIT` in mirror mode or `APPEAR` standalone. Suppressed during active pursuit. |
| `Radar_DirectionArrow` / `0x24196ad4` | Relative target direction, hidden without a target and during active pursuit. |
| `Radar_Icon` / `0x6ccbbbf1` | Authored idle state instead of distance-dependent host alpha. |
| LEDs / `0x69aa01e7`, `0x9f59065a` | Both installed mask windows updated; zero width without a target. |
| Backup backing / `0x8c20a763` | Plays the backup row's scripts from the backup ETA (below, "Backup timer"). |
| `BACKUP_TIMER` / `0xbee44775` | The backup ETA in `mm:ss.cc` while a backup is on its way. |
| Bounty row / `0x8674e6d4`, number `0x875e92eb` | The chase's pending bounty, shown with the board in every chase (below, "Bounty row"). |
| `CTS_GROUP` / `0xd60fc075`, number `0x3ff5f33c`, title `0x64247241` | The cost-to-state flasher (below, "Cost to state"). |
| `GENERIC_ICONS`, `TOTAL_INFRACTIONS_TEXT`, `INFRACTION_GROUP_1`..`4` | The infraction notifier (below, "Infraction notifier"). |
| Total bounty / `0xea903012`, number `0x9b0ac8ca`, title `0x41a55ecf` | The career's total bounty flasher (below, "Total bounty"). |
| `CASH_GROUP` / `0x3f8644ca`, number `CASH` / `0x0013c45e`, title `0xb32a0463` | Held hidden: the original never drives it (below, "Cash flasher"). |

The radar backing and group no longer receive the earlier +75/+15 unit shifts.
Those shifts aligned the mirror-oriented initial poses in the standalone view,
leaving the detector floating in the upper quarter of the scene. The authored
standalone entrance places it near the top of the screen instead.
The presenter preserves the standalone backing's authored alpha 200 rather
than forcing it opaque with the old mirror compensation.

The fill geometry is applied again after FEng script evaluation. In particular,
restarting a flashing segment must not briefly restore its authored position or
size over the newly calculated fixed edge.

## Remaining presentation gaps

| Feature | Status |
|---|---|
| Heat arcs / `0x862824c9`, `0x4b2cbe1b` | Bound: the two masks turn with the fractional heat (`hud/police/heat.rs`, installed test in `hud/police_install_tests.rs`). |
| Radar audio/targets | Native ground selection, range, cone and per-update ramp are bound. Detector click and speed-camera selection remain absent; roll-sensitive lateral orientation remains unverified. |
| View mode | The rear-view setting selects the detector layout. Original view-specific rear-view eligibility still needs comparison. |
| Fades/sounds | Radar/meter transforms and transitions run from installed scripts; full HUD event and sound dispatch remains incomplete. |

## Bounty row

**[decomp]** for the board, **[binary]** for its feed, **[verified]** for the objects. The row inside the board holds
the label `BOUNTY >` and a number. On every HUD update of a chase the board plays the row's appear script (when it is not
the one playing) and prints the chase's pending bounty in it with the thousands separator (`%$d`). The pending bounty is
the perpetrator's rep points of the chase so far, normal plus destruction ([ai-pursuit-heat](ai-pursuit-heat.md) section
5.1), fed every update; it is not the car's banked bounty. The row's `INIT` chains to `HIDE`, so it rests hidden.

The rewrite (`hud/bind.rs`, fed by `PursuitHud::bounty` from the police's `rep_points`) writes the number, plays the
appear and holds the row at full alpha with the board. Tests: `hud::pursuit_tests` (synthetic) and
`hud::pursuit_install_tests` (the real package).

## Cost to state

**[decomp]** for the element, **[binary]** for its feed, **[verified]** for the objects. `CTS_GROUP` holds a title and a
number over a flasher panel; its `INIT` chains to `HIDE` and its appear and leave scripts last 600 ms. The HUD feeds it
the perpetrator's cost to state on every update of a chase. A value above the one it holds replaces it and keeps the
total up for 120 HUD updates; a zero clears the held value without showing it; any other value is ignored. While the
total is up the title takes the language label `HUD_PURSUITCTS` and the number prints the total with the thousands
separator; the first update up plays the appear, the first update after plays the leave. The game builds the element
only in the standard single-player HUD and enables it in a career race context or a pursuit race while the player's
score option is on.

The rewrite (`hud/cost_to_state.rs`) runs the 120 updates as 2 s **[inferred]** (the HUD's update rate was not timed)
and shows it in every chase: the rewrite has no score option, and its free roam outside the career counts the cost as
the career does. Tests: `hud::cost_to_state` (the rule) and `hud::pursuit_tests` (the binding).

## Total bounty

**[decomp]** for the element, **[binary]** for its feed, **[verified]** for the objects. A group beside `CTS_GROUP`
holds a title ("TOTAL BOUNTY:"), a number and an info panel; its `INIT` chains to `HIDE` and its appear and leave last
600 ms. The game builds the element only in the standard single-player HUD of a career race context, and enables it as
it does the cost to state (the score option, in a career context or a pursuit race). It holds the last total it was
given, starting from 0 when the HUD is built. A total other than the one it holds replaces it and keeps it up for 120
HUD updates; the same total does nothing. While it is up the title takes the language label `HUD_TOTALBOUNTY` and the
number prints the total with the thousands separator (`%$d`); the first update up plays the appear when the group is not
playing it, the first update after plays the leave when it is.

The total comes from one place **[binary]**: when the cop manager removes a finished pursuit of the player, it banks the
chase's points into the car's record and gives the HUD the career's total bounty, the stable's sum over its cars plus
the bounty of the cars sold (`Stable::total_bounty`). That happens at the end of every chase, evaded or busted (a bust
banks nothing, so its total is the same and shows nothing). A speed trap or a milestone that adds bounty between chases
shows at the next chase's end, if the total then differs. Whether a race's chase reaches the call is not resolved.

The rewrite (`hud/total_bounty.rs`, fed by `HudState::total_bounty`) keeps the total the last chase's end left
(`scenes/world/career/pursuit.rs`) and gives it every frame in career mode; the flasher shows a change of it. The 120
updates run as 2 s, as for the cost to state **[inferred]**. The rewrite's HUD outlives a world session, so a total
equal to the one the HUD last showed does not show again after the world reloads **[inferred]** (the original builds a
new element). Tests: `hud::total_bounty` (the rule), `scenes::world::career::hud` (career mode only) and
`hud::pursuit_install_tests` (the real package).

## Cash flasher

**[verified]** for the objects, **[binary]** for its absence. Between the total bounty and the cost to state the package
holds a third flasher, `CASH_GROUP`: the title "CASH:" (`0xb32a0463`), the number `CASH` and an info panel, with the
same `INIT` to `HIDE` rest and 600 ms appear and leave as the other two. Nothing in the PC executable names the group or
its title, by hash or by text, and the CC0 HUD sources build no element for it. The number's name `CASH` is written only
by front-end screens. The group therefore rests hidden for the whole game: cash won in a race or a milestone is not
flashed on the HUD.

The rewrite keeps it out of the layout, so it stays hidden. Test: `hud::pursuit_install_tests`.

## Infraction notifier

**[decomp]** for the element and its feed, **[binary]** for the labels, **[verified]** for the objects. The notifier
sits under the heat meter's top group (`0x157c5b37`): the generic infraction icon `GENERIC_ICONS` with a ring group
around it, the count `TOTAL_INFRACTIONS_TEXT`, the `INFRACTIONS_HEADER_GROUP` (two "INFRACTIONS" strings, a text and its
shadow) and four row groups `INFRACTION_GROUP_1` to `_4` of two strings each. Every part's `INIT` chains to `HIDE` (zero
alpha). A row's appear lasts 2.4 s and chains to its leave (160 ms), which chains back to `HIDE`: a row times itself
out. The icon's appear (300 ms) chains to a looping pulse; it raises two messages, one that runs the rings' appear and
one that fades in the count and both header strings. Its leave (300 ms, then `HIDE`) raises the rings' leave and the
message that hides the count and plays the header's leave.

The game reports an infraction to its infraction manager (in free roam; in a race only when a race-status test passes,
which is not resolved). An infraction whose bit the chase already holds changes nothing on the HUD. A new one sets the
bit and sends the HUD a message that, when delivered, takes the infraction's language label and gives its text to the
notifier:

- the first row whose script is `HIDE` plays its appear and every string under it takes the text (no row shows it while
  all four are out);
- the icon plays its appear from the start, even while it is up;
- the count prints the number of different infractions the chase holds (`%d`).

On each HUD update, when no row is out of `HIDE` and the icon plays its appear or its pulse, the icon plays its leave.
The manager clears the bits when a pursuit starts.

The row labels **[binary]**, by infraction bit: 1 speeding `0x1f87313a` "Excessive Speeding", 2 racing `0x6df08aff`
"Speeding", 4 reckless driving `0x21857819`, 8 assault `0x66432b02` "Ramming a Police Vehicle", 16 hit and run
`0x3e1b41f3`, 32 damage to property `0x98ac2bae`, 64 resisting arrest `0xc03aa693`, 128 off road `0x50f83c6c`. The label
names do not match their texts for the first two (`INFRACTION_SPEEDING` reads "Excessive Speeding").

The rewrite (`hud/infractions.rs`, fed by `PursuitHud::infractions`, the police's infraction log of the chase) tells
each infraction of the log it has not told yet, in the log's order, and starts again from none told when a chase starts
or the log loses an infraction (the next chase's). It sets the rows' label rather than printing the text, so a language
switch follows. Several infractions of one update all read the final count **[inferred]**: the original prints the count
as each arrives. The infractions the police record are those of `ai-pursuit.md`; a LAN client's board carries none (the
global block has no log). Tests: `hud::infractions` (the rule) and `hud::pursuit_install_tests` (the real package's
rows, count, icon and their timing out).

## Backup timer

**[binary]** for the update, **[verified]** for the objects. The board has a backup row under the main board: a
backing, and a group holding the label "BACKUP:" (language label `HUD_BACKUP`) and the timer string `BACKUP_TIMER`.
The game drives only the backing. The backing's appear script sends, half-way through, the message that plays the
group's appear; its leave script sends the group's leave; its looping flash script sends the flash that the group and
the timer answer. Each HUD update during a pursuit the board receives the pursuit's backup ETA in seconds:

- ETA 0 or less: the backing plays its leave script;
- above 0 and at most 10 s: its flash script;
- above 10 s: its appear script;

each started only when it is not the one playing. The backing's appear slides its texture in from below and its leave
slides it back out, onto the texture's transparent rows. Its colour stays at full alpha, so only the texture's clamp
hides it ([feng-image-repeat](feng-image-repeat.md)). While the ETA is above zero the timer shows it in the pursuit
timer's format (`mm:ss.cc`; the original rounds to 1/4000 s first). While the board shows the cool-down, the row is not updated.
The ETA is the pursuit's: set to `BackupCallTimer` when the backup is called, counting down while the player is seen,
slightly negative when the backup arrives, and zeroed on entering cool-down ([ai-pursuit-heat](ai-pursuit-heat.md)).

The rewrite (`crates/nfsmw/src/hud/backup_timer.rs`, fed by `PursuitHud::backup_eta`) follows this, with one
**[inferred]** guard: the leave script is played only on a row shown in this chase, so a row that rests hidden at the
start of a chase is not played out. A stock-game capture confirms the ETA that feeds the row (the call timer of the
heat at the call, zeroed on entering cool-down, a fresh full timer when the chase resumes; see
[ai-pursuit-cops](ai-pursuit-cops.md) section 4), so after a cool-down the row is driven by the new call. It did not record
the board itself: whether the original's leave on a hidden row shows anything, and what the row shows during a
cool-down, remain open. A co-op guest's board has no ETA yet (the global block does not carry it).
Tests: `hud::backup_timer_tests` (synthetic) and `hud::pursuit_install_tests` (the real package's cascade to the label
and timer).

The board group still has host-managed alpha. This is not a claim
that every original HUD message, fade, counter animation or sound is reproduced.

## Verification

The installed HUD regression checks both duplicate heat labels and live
counters, standalone and mirror transforms, repeated state changes, stale target
suppression, 4:3/16:9/ultrawide centering, and the five meter segments retaining
fixed edges after their scripts run. Synthetic tests cover the signed threshold
values and explicit cooldown phase. These checks establish asset binding and
state integration; a matched original route/capture remains a separate check.
