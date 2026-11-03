# UI repair batch R7: My Cars room and the career safe house

Repair candidates from the R6 playtest follow-up. User acceptance remains open.

## Changes

1. **My Cars customizes in the crib.** The original keeps the current platform when Customize opens. Only a return from
   a world zone sets the shop's platform, and the marker toggle swaps the shop and the backroom. My Cars therefore
   customizes in the crib it opened from. The camera eases between the customize poses there. An auto shop entered from
   the world still stands in the shop room.
2. **My Cars opens without a black screen.** The garage used to show no room until its car list had been read, and its
   camera waited for that too. The crib and the camera now come up at once, and only the car waits. The garage also runs
   the room's particles now.
3. **The career safe house is no longer mirrored.**
   - Every drawn solid in the safe house file has a pivot matrix that negates x.
   - The original's platform draw ignores solid pivots, but the remake applied them, so the room was mirrored.
   - With the pivots ignored, the executable's 302.85-degree safe house turn is now used.
   - The crib's pivots are identity matrices, so the crib is unchanged.
   - One file and one angle serve every career safe house.

## Evidence

- The settled safe house render (`Car_Select`) matches the recorded safe house menu: the Castrol banner reads correctly
  under the vent, with the trusses and the tyre stack around it. The main menu render is unchanged.
- In the executable, opening Customize changes the platform only when leaving the backroom.

## Open

- **Physical check.** The My Cars entry, customization and backroom return need checking in play.
- **Backroom.** Its turn is still not compared with a recording.
