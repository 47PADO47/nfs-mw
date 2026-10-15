# Player options routing

## Evidence and behavior

The original option categories contain Player between Gameplay and Controls in
both the front end and pause menu. Selecting it opens the shared option-row
package with the Player category, rather than a separate camera screen. Its
first implemented controls are Transmission, Camera and Gauges, in that order.
The supplied original-game screenshot corroborates their order. Existing
settings and serialization keys remain unchanged when moving these controls.

The main-menu Player header uses language label `0xC055165F`; the pause header
uses `0xD9DC2F12`. Installed English strings read `player` and `PLAYER`.
The Camera row uses installed language label `0xF6CCDC5F`, `Camera`.
Player retains its existing icon texture and category label. Its menu item must
be enabled, selectable and reachable from both category strips. Leaving the
screen saves changed settings and returns to the category strip with Player
still selected, using the normal package animations and input handling.

Gameplay keeps the existing HUD layout, Radio HUD, Minimap, Rear-View Mirror
and Turn Signals controls. Moving the three Player controls must not change
their values or the remaining controls' behavior. Custom camera values stay
available through the existing setting; this routing change does not change
camera cycling or the user's stored choice.

Only implemented controls appear. Race information, split time, score and
leaderboard controls from the original Player screen remain absent until
their corresponding behavior exists. Single-player routing does not expose
inactive player-selection triggers. This is a menu-category correction, not
a claim of full original settings coverage.

## Validation

Synthetic checks assert the Player control order and unique category ownership.
Installed-package regressions navigate to Player through both category strips,
check the installed header and row strings, change Camera with left/right,
verify the existing settings partial, leave through Back, and verify save and
return selection. Existing Gameplay HUD-layout tests follow its new first row.
