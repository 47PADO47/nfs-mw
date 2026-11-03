# Controller remapping screen

This is a remake feature for UI-026. The native PC controls packages supply existing rows, fields and header arrows.
It does not certify the original game's complete controller page or its appearance.

The keyboard and controller pages use the same eight native action rows and two binding slots. Remastered and
Custom list views retain their additional HUD rows. Header arrows or the bound MenuTriggerLeft/MenuTriggerRight
actions switch device pages; default keyboard bindings are 9/0. Directional input still selects rows and columns.
Each page shows bindings for its own input family and sends a distinct keyboard/controller capture command.

Controller capture uses the active pad from InputPresentation and then holds that device until completion or
disconnection. It requires all buttons, sticks and triggers to return to neutral before accepting a gesture.
The gesture that opened capture therefore cannot become the assigned button. Deliberate button/trigger/stick input
is captured from physical Gamepad state, including inputs with no prior gameplay binding. Escape cancels. The page,
pointer, wheel and footer actions stay gated while capturing, including the frame capture completes or cancels.
After disconnection, capture returns to the connection prompt and requires neutral input on the replacement pad.

New controller bindings replace the selected action's gamepad slot and remove that physical source from competing
vehicle controls. Keyboard, mouse, menu, camera and contextual Interact/Map bindings are preserved. A physical
button may therefore remain bound in those separate input contexts. Within an action, a source appears once;
digital/analog triggers and PadAxis/PedalAxis aliases identify the same source. Steering buttons keep their row's sign.
A steering axis is stored as one complete signed axis, shown from both directional rows; capturing its negative
or positive direction determines inversion without adding duplicate full-axis contributions.

Explicit remaps save through Bindings::save_to with the existing configuration merge. Screenshot/no-save sessions
keep edits in memory. Save failures follow the existing keyboard warning path; no successful persistence is claimed
when the write fails. Synthetic regressions cover slots, preservation, neutral gating, cancellation and file round
trip. Installed regressions cover native main/pause tabs, fields and header pointer routes. Physical controller and
original-game comparison remain separate acceptance checks.
