# UI repair batch R5: preset livery and reference checks

## Fixed

**UI-056: the main menu's M3 GTR had no vinyls.** A preset (the menu's career-start M3 GTR) carries its livery in
its own part slots, while its customization selection is empty. Car assembly applied a vinyl only when the
selection named one, so every preset car lost its livery.

Preset cars now apply their own vinyl. A missing bitmap logs a warning and keeps the painted car rather than
failing the load. A regression checks that the career-start preset has the vinyl layer and a stock assembly does
not; it passes on both reference installs.

## Verified against recordings, no change needed

- **UI-047 (Junkman in standalone My Cars).** The original's standalone My Cars engine page offers Ultimate and
  Junkman (2/2). The remake's listing of Junkman is correct.
- **UI-007 (EA Trax fresh-profile defaults).** On a fresh profile the remake shows the same per-track modes as the
  original: 01 All, 02-04 Menu, 08 Menu, 09 All, 10 Menu, 11 Racing, under "Sequential". The defaults come from the
  installed `DefPlay` attributes, as in the original.

## Investigated, not fixed

**Main-menu orientation, intro clipping and garage camera moves.**

Settled on the stock and HD cribs:

- The crib's 180-degree room turn was fitted to a different, scaled modded crib, so it does not apply to these
  files.
- The original's car stand gives every platform except the car lot yaw 304.97 with a -0.38 degree tilt; the remake
  uses the same yaw.
- The original's camera builder gives the same eye as the remake.

Even so, the remake's main-menu car faces the opposite side from the original's settled frame, and no room yaw
reproduces the original's background.

The stand routine also turns the car position by a per-platform angle: 134.41, 302.85 or 0 degrees. That has no
visible effect at the origin, but it suggests a per-platform frame that may also apply to the camera. That frame is
the next thing to read.

**Steam, haze and birds.** The crib's emitter solids carry a position and a placeholder mesh only. Which effect
group each one plays is not in the platform file. It must be found in the executable before effects are added;
no group is guessed.

**UI-054 (gauge colour retention).** The recording does not change the gauge style after editing colours, so the
original policy remains unestablished.
