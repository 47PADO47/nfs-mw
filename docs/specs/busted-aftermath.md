# After the ticket: the impound notices, the career car select and the return to free roam

What follows a payment on `Infractions.fng`: the notice that the car is impounded, the **career car select**
(`Car_Select.fng` in its career, not lot, mode), the release of an impounded car, the game over, and the safe house
menu that ends in the respawn. The sequence before the ticket is [busted-sequence.md](busted-sequence.md); the payment
rules (cash, strikes, markers) are [career-economy-flows.md](career-economy-flows.md) section 6; the heat meter shown
on the car select is [career-cooling.md](career-cooling.md).

- **Evidence tags:** **[decomp]** read in `speed.exe` (Ghidra, read only; bytes and constants, not decompiled text);
  **[community]** taken from the public CC0 reconstruction `dbalatoni13/nfsmw` (understanding only, nothing copied),
  which decompiles the same game and was not re-checked in `speed.exe` for that point; **[verified]** checked in the
  install files; **[unconfirmed]** a guess.

## 1. Summary

1. Both payment buttons end by switching to `Car_Select.fng`. The argument says which animation the car select opens
   with: `0` after a marker, `0x100` after cash (the new strike appears), `0x200` after the impound notice (the
   impounded stamp). **[decomp]**
2. When the payment impounded the car, a one-button notice comes first. Its message is `MW_BUSTED_STRIKE_LIMIT` when the
   strike limit was reached, else `MW_BUSTED_INSUFFICIENT_FUNDS`. Closing it switches to the car select with `0x200`.
   **[decomp]**
3. The car select (career mode) lists the owned cars. Choosing a free car sets it as the current car and goes to the
   safe house menu (`MainMenu_Sub.fng`). Back is refused while the car the player came with is impounded or waiting for
   its release. An impounded car asks to be released instead of being chosen. **[community]**
4. Resume Free Roam in the safe house menu loads a free-roam world with the current car at the respawn marker. **[community]**
5. Free roam outside a career has no cars, cash or markers: the payment resumes driving as before.

## 2. The payment buttons **[decomp]**

The infractions screen handler (`speed.exe` `0x005a15b0`, undefined as a function in Ghidra; read as bytes) answers
`BUTTON_PRESSED` (`0x0c407210`) and tells the buttons apart by the object name hash: `0xb8a7c6cd` pay with cash,
`0xb8a7c6cc` pay with a marker. For cash:

1. `cash = (fine > cash) ? 0 : cash - fine`, strikes + 1 and the decay counter 0, the unserved infractions are served,
   the car's heat is 1.0 (these are in [career-economy-flows.md](career-economy-flows.md) 6.1).
2. `strike_limit_reached = strikes >= limit`; `not_enough_cash = fine > cash_at_open` (never for a marker).
3. If the limit was reached: the message `0x78f0e298` and impound reason 2; else if the cash was short: `0x1ecffa6e` and
   reason 3. Either way the car becomes impounded (`strikes = limit`, countdown 5) and a one-button `OK` notice
   (`0x417b2601`) with that message is shown; its answer message is `0x34dc1bec`.
4. Otherwise the handler switches package at once: marker, argument `0`; cash, argument `0x100`.
5. On `0x34dc1bec` (the notice closed) it switches to `Car_Select.fng` with argument `0x200`.

The two message strings are in the install's strings table **[verified]**: `0x78f0e298` "Impound Strikes have reached
the limit for this vehicle. It will now be impounded by the police. Select another car from your safe house if one is
available. TIP: You will receive an SMS message when your car can be released."; `0x1ecffa6e` "You do not have enough
cash to pay your fines. Your vehicle has been impounded and your remaining cash has been confiscated."

How the car select reads the argument (`0x100` as "show strike", `0x200` as "show impounded") is **[community]**: the
reconstruction takes one byte of the screen argument as an animation type with the values nothing, show strike and show
impounded. The byte the reconstruction names does not obviously match the bit the handler sets; **[unconfirmed]**.

## 3. The career car select **[community]**

`Car_Select.fng` serves three modes. The car lot zone opens it in lot mode (buy, sell: [career-car-lot.md](career-car-lot.md)).
The safe house menu's Car Select and the bust open it in **career select mode**:

- **Rows.** The career's owned cars, impounded ones included, in the stable's order. The shown car starts at the current
  car. Left and Right step; the package plays the carousel. The price and cash lines of the lot are hidden.
- **Choosing.** Moving to a car makes it the current car (`SetCurrentCar`). Accept on a free car goes to
  `MainMenu_Sub.fng`. Accept on an impounded or releasable car opens the release choice (section 5) instead.
- **Back.** Restores the car the player came with (when it is not impounded) and goes to `MainMenu_Sub.fng`. If that
  car is impounded or releasable, Back shows `CS_IMPOUND_MUST_CHOOSE_NEW_CAR` ("Your car has been impounded. You must
  choose a new car to race.", `0x630931b6`) and stays, even when a free car is shown.
- **Sell.** Not in this mode; the lot's sell button belongs to the lot.

### 3.1 What the header shows for a car

| Element | Free car | Impounded or releasable car |
|---|---|---|
| Strike pips | `IMPOUND_STATE_1`.. `_5` scripts `SHOW` for the strikes taken, `HIDE` above; the strike limit (3 to 5) sets how many pips exist (`POS1`..`POS3` scripts move the group, `x4` and `x5` groups show for limits 4 and 5); the group `0x75721326` is visible | group hidden, the stamp shows instead |
| New strike | with entry `0x100` the pip of the latest strike plays `ANIMATE`, once | |
| Impounded stamp | `HIDE` (`0xbc7b91f`) | texture `IMPOUNDED_ENG` (a language variant per language) on `0xce18427d` and `0x5b8f2a45`, group `0x64f3a49c` `APPEAR`; with entry `0x200` the stamp group `0xbc7b91f` plays `ANIMATE` once, else `NORMAL` |
| Value lines | "Bounty:" and "Fines Due:" (`0x17574b0e`, `0x915f4d26`) with the car's bounty and unserved fines | "Release Cost:" (`0x9db4df7d`) with the fee, or "Pending" (`0xaefedad9`) while counting down; "Cash:" (`0x073b79e0`) with the cash |
| Accept label `0xb94139f4` | "Accept" | "Release" (`0x281dee8a`) when releasable and the cash covers the fee, "Use Marker" (`0xf9c73cc2`) when an Impound Release marker is held, else "Accept" |
| Heat meter | the meter of [career-cooling.md](career-cooling.md) section 4 | same |
| Marker counts | a visible group (`0x39dc21f9`) shows the Extra Impound Strike markers held, for a free car | group `0xe998fe99` shows the Impound Release markers held |

The object and label hashes are **[community]**; the package objects they name were found in the `Car_Select.fng` dump
**[verified]** (`IMPOUND_STATE_n`, `POS1`..`3`, the stamp, the heat meter and the labels exist in the install).

## 4. Extra Impound Strike marker **[community]**

The car select's button 4 (`PAD_BUTTON4`) offers the marker for the shown car: if it is free to drive, its limit is below
5 and an Extra Impound Strike marker is held, a two-answer question (`0x0cebda20`) asks to use it; yes consumes the
marker and raises the limit by one (to 5). If the limit is already 5 a notice says so (`0xbcae8539`). Nothing is offered
for an impounded car. The rule is `CareerRecord::apply_extra_strike_marker` plus the marker inventory.

## 5. Releasing an impounded car

The choice (`MaybeReleaseCar`, **[community]**):

| Releasable (state 4) and cash >= fee | Impound Release marker held | Offered |
|---|---|---|
| yes | yes | three answers: Use Marker, pay the fee, cancel (`0x4eb9591f`) |
| yes | no | two answers: pay the fee (`0xcad5722e`, "The release fee is %$d..."), cancel |
| no | yes | two answers: Use Marker (`0xed4dd591`), cancel |
| no | no | a one-button notice `0xe96fa0c5` ("This car cannot be released. You do not have enough cash or an Impound Release marker...") |

- **Fee:** `round(0.25 * Cost)` ([career-economy-flows.md](career-economy-flows.md) 6.2).
- **By cash:** the record is freed (strikes 0, countdown 0, state 0) and its heat is 0, the fee is spent. **[decomp]** (the
  handler `0x007ad490` message `0xa0fc39f9`).
- **By marker:** the same record reset and heat 0, one Impound Release marker consumed. The handler applies it
  **without testing the impound state**, so a car that is still counting down (states 2 and 3) is released by a marker;
  only the choice dialog gates what is offered. **[decomp]** (message `0xe845bc1c`). The earlier reading of
  [career-economy-flows.md](career-economy-flows.md) 6.2 (marker only from state 4) was too narrow.
- **Countdown:** five ticks, one per evaded pursuit or career race win ([career-economy-flows.md](career-economy-flows.md) 6.3);
  the state becomes 4 and the SMS 120 is queued.

## 6. Game over **[community]**

When the car select opens, `CalcGameOver` runs (the package posts `0xe0b38195` when its impound animation ends): if the
stable has no car that is free to drive and no Impound Release marker is held, the game-over flag is set
(`career-economy-flows.md` 6.4) and a one-button notice with `0x164bed94` ("All your cars have been impounded by the
police. You've failed to become the Most Wanted.") is shown. Closing it leaves career mode for the main menu
(the career manager state, `MainMenu_Sub.fng` in the original). Cash is not consulted (an open question there).

## 7. Respawn and the return to driving

- The safe house menu's **Resume Free Roam** (existing, [career-safehouse.md](career-safehouse.md)) asks for the roaming
  start autosave and loads free roam: the current car's record heat becomes the new car's heat (floor 1) and the car is
  placed at the respawn marker ([career-safehouse-2.md](career-safehouse-2.md) section 9): the marker of the last visited
  safe house when the flag is set, else of the last visited zone of any kind, else `career_start_<region>`. **[community]**
  for the bust path (the menu request flag is not traced); the marker rules are **[decomp]** in the safehouse spec.
- The chase is over, the cops are gone and the heat was reset to 1 by the cash payment (a marker leaves the heat).

### Rap sheet counts of the chase

The recorder writes the chase's counts into the history ([career-rapsheet-2.md](career-rapsheet-2.md) section 2). The
world can supply: cops involved (cops that came within the engagement radius), cops damaged, cops destroyed, roadblocks
dodged, spike strips dodged, helicopters involved, roadblocks deployed, cost to state, bounty, duration and the number of
infraction kinds. Not counted by the world yet: traffic cars hit, spike strips and helicopter spike strips deployed,
patrol cars, support vehicles and property damage (slots stay 0). The history's raw sums skip a busted pursuit (flows 5.1).

## 8. Rewrite status

- **Rules** (`nfsmw-career`, `economy/aftermath.rs`, `economy/cooling.rs`): the notice and the car select argument for a
  payment, the car select numbers, the back rule, the release choice and release, the strike marker, the heat meter.
  A release marker frees a car in any impound state (`stable/impound.rs` `release_by_marker`).
- **Front end** (`frontend/flow_bust.rs`, `career_screens/car_select.rs`, `car_select_view.rs`): in a career the payment
  opens the notice (when impounded) and the car select; the car select draws the pips, stamp, value lines, markers and
  heat meter, moves the current car, asks to release, offers the strike marker and the sale (button 5, the lot's sale
  rules), and raises the game-over notice (which leaves for the main menu). Leaving goes to the safe house menu, whose
  Resume Free Roam drives again. The safe house menu's own Car Select opens the same mode. Free roam without a career
  resumes driving at once. Installed tests: `installed_career_bust_cash_payment_*` and `installed_career_bust_third_strike_*`.
- **World** (`scenes/world/career`): the bust settles the record (existing, `bust.rs`); `rap_tally.rs` writes cops
  involved, damaged, helicopters, roadblocks deployed and dodged and spike strips dodged into the history; `respawn.rs`
  remembers the respawn marker of each zone the player engaged and, after a paid bust, places the car on the road nearest
  the last safe house's marker (else the last zone's) when the game runs again (installed test
  `installed_paid_bust_puts_the_car_back_at_the_last_safe_house_marker`).
- **Not done:** the arrest NIS; the stamp's language variants; the game over's career-manager mode (it returns to the main
  menu); the carousel's 3D car and loading bar; rebuilding the world on resume (the player car model is the free-roam car,
  whatever the career car is); saving the respawn keys; the `career_start_<region>` fallback (the car stays where it is);
  the Extra Impound Strike marker's and the heat meter's rendered look (no screenshot was taken: only the data and the
  scripts are checked); the stamp and pips were not compared with the original.

## 9. Open questions

- The exact meaning of the screen argument's bits in the car select (section 2).
- Whether the bust path sets the safe house respawn flag (section 7).
- Whether `0xe0b38195` is posted by the stamp animation or by the screen opening.
- The game over's follow-up: the reconstruction switches to the main menu in the career manager mode.
