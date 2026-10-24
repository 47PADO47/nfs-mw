# Garage front-end flow

Normal sessions use the installed FEng packages and the shared front-end runtime.
See [garage-game-ui.md](garage-game-ui.md) for the original package and flow contract.
The former normal egui garage screen has been removed.

This contract extends the existing runtime customization and car assembly contracts.
It describes the remake's front-end integration; it is not evidence of original
screen timing, camera placement, prices, unlock requirements or save encoding.

## Entry and return [ours]

- Main Menu / My Cars opens an installed-car customization session. It uses the
  original My Cars grid and crib platform without loading the streamed city.
- A career auto shop opens a customization session for the current owned car.
  The world remains resident and frozen while the installed customization-shop or
  backroom platform is displayed. It resumes when the session closes.
- F6 remains a diagnostic shortcut. Main Menu / My Cars and world shop markers
  are normal entry points and do not require the shortcut.
- Closing My Cars restores the main menu. Closing a world session returns to the
  same world. Garage input must not also pause or drive the world.

## Editing [ours]

Browse installed vehicles and their available body parts, appearance choices,
performance packages and tuning. Use the existing install readers and renderer.
Unavailable choices retain an explanation instead of silently applying stock.
No installed files are changed. Vehicle and part identities come from the install,
so an installed mod car can participate without a source-code name list.

My Cars keeps independent slots with stable identities under the loaded alias.
The default capacity is twenty slots; its persisted capacity can be extended for
mods. Add duplicates the current draft into a new slot. Save updates the selected
slot. Delete hides imported native slots with a per-alias tombstone and removes
remake slots. The old folder-key file is migrated without changing its contents.
Stock and My Cars choices retain separate identities through single-player and
split-screen race starts, including two custom slots of the same car model.

Normal My Cars checkout confirmations use the profile's quick-race unlock levels and installed
bin awards. Existing fitted parts are retained. Locked new selections may be
previewed, with their lock reason shown before confirmation.

Preview edits and confirm them through the scene's transaction interface. Cancel
restores the selection at entry. Career purchase validation and persistence belong
to the world/career transaction backend, not the menu widgets.

Performance tuning opens its original pause-menu screen. Accept stores all three
profiles, Start stores and resumes, and changed Back asks before discarding.
A stock Quick Race car can be tuned without creating a new My Cars slot.

Keyboard, mouse and controller actions use the existing action layer. Opening and
closing consume the gesture so a held Accept or Back does not affect the screen
under the garage. The menu preview may orbit independently of driving controls.

## Validation boundary

Synthetic tests cover normal menu dispatch, scene transaction forwarding and
return behavior. Installed-asset checks must separately establish that the selected
car and its customizations assemble and render. Original-game visual comparison,
economic values and complete customization inventory require their own evidence.

## Free-roam entry and exit [static executable]

Original menu-gate permissions block auto shops during events and during all
pursuit phases, including cooldown. Pause tuning is a separate operation and
remains available in the original eligible race contexts.

The original exits front-room career customization by starting career free roam
again. The remake keeps the resident world, then places the car at the entered
shop's installed `respawn` marker, with its authored heading. Co-op guests follow the lead.
Checkout keeps the garage open; only the final Done uses this marker. Pause
tuning has no shop marker and leaves the car in place. The original restart
chain and `DoZoneMenuAction` assignment to the visited zone marker are verified
statically in [career-safehouse-2.md](career-safehouse-2.md), section 9.
Matched live career exit captures remain a separate verification step.
