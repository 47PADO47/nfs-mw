# UI repair batch R1: tuning refusal and Showcase direction

Two repair candidates for the [Pass A+B follow-up](ui-parity-pass-ab-followup.md). User acceptance remains open.

## UI-060: Performance Tuning hang

The screenshot shows the pause menu after Accept on Performance Tuning: the header and caption remain, while the
icon has faded and input is off. That is the pause menu's leave state.

`open_tuning` runs only after that leave. It returned without any feedback when:

- a garage session was still recorded;
- or the scene no longer reported tuning available.

The menu then never regained control.

A route regression showed a second problem. Even the existing failure alert did not recover the menu. After it
closed, the pause menu was on top but ignored input: a package that has played its leave does not take input again
in place. The reference recording reloads `Pause_Main` on every return to it.

Changes:

- Every refusal reloads the pause menu, with freshly computed entries, beneath the existing "tuning screen could not
  open" alert. The refusal reason is logged.
- A garage session whose screens all closed without the normal close path is released before tuning starts. Its
  unconfirmed selection is rolled back through the scene's cancel.

The exact preceding route from the report was not reproduced. These changes remove every quiet refusal and the
unrecoverable alert path. They do not prove which refusal occurred in the playtest.

## UI-061: Showcase vertical direction

A targeted reference recording of the modded PC Showcase was used. Holding the right stick Up lowered the camera
toward the car's side, and holding Down raised it.

The remake's right stick already matched that. D-pad, left stick and keyboard Up raised the camera instead, so
the vertical inputs disagreed with each other. All of them now lower the camera.

Camera inversion still applies only to the right stick and mouse look. Whether the original tilts on D-pad or left
stick at all is not recorded; this candidate makes them consistent with the recorded right stick.

## Checks and limits

Regressions:

- **UI-060:** a full pause route offers Tuning, then refuses it. The test checks for the alert, a reloaded pause
  menu after dismissal, and Back resuming the game.
- **UI-060:** a stale-session release with rollback.
- **UI-061:** right stick, D-pad and left stick Up all lower the camera.

All three failed with the fixes reverted and pass with them, on both the modded and unmodified installs. The
neighbouring tuning, inspection, pause, garage flow and safe-house tests pass.

Four icon-menu tests fail identically with and without these changes:

- the three historical failures listed in the [handoff](ui-parity-handoff.md);
- the EA Trax save-return route test.

Formatting and guards pass. Native Clippy reports no sites in the changed files.

Physical-controller confirmation, the reported in-game route and a rendered comparison remain open. No playable
build is updated by this record.
