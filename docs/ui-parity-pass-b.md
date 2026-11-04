# UI repair pass B: navigation and shared presentation

## Repair candidates

- **My Cars return camera (UI-057):** returning from customization keeps the current camera pose.
  Loading the overview room no longer starts the main-menu entry animation. The overview requests
  its installed screen move from the retained pose. Fresh entry and missing-pose fallback retain
  initialization. Repeated requests do not restart the transition.
- **Settings footer ownership (UI-001):** the shared presenter suppresses the complete native
  settings-footer contents, retaining the animated strip. A native script restoring a caption
  cannot create a second hint merely because its translated text differs. Keyboard and controller
  prompts still use the existing binding-aware presenter.

Both candidates had failing regressions on both reference installations before their source fixes.
These changes reuse the existing camera mover and footer suppression helpers.

## Audit coverage by repair ticket

| Ticket | Evidence and remaining acceptance |
| --- | --- |
| B1 Return state | Repeated EA Trax save/Back; invalid-name recovery; Vinyls parent return; customization/Showcase return; each Blacklist category return. Keep actual long-session playtest acceptance open. |
| B2 Camera return | Room-switch regression checks pose continuity and eventual overview target twice. Existing tests retain fresh intro and deferred part-camera behavior. Full rendered motion and initial wall clipping remain separate checks. |
| B3 Footer ownership | Settings enter/Back/re-enter and device switching, plus controlled GPU captures. Existing gauge, EA Trax, tuning Help and controls footer checks remain regression controls. This does not certify every transition frame in a full gameplay capture. |
| B4 Selection/scroll | Actual Graphics/HUD selection through the final row, wrap and reopen; existing EA Trax mode editing, career grids, control fields and repeat-rate tests. Physical XInput cold-launch hold remains open. |
| B5 Widgets | Existing slider, centered tuning, spoiler-art, cart-alignment and Help tests are regression controls. Reference appearance and all visual states remain open; no speculative widget rewrite. |
| B6 Cleanup/audio | Transition-spam rejection, scene focus reset and scheduled speech teardown. Strengthened speech check requires the next session to play a new line, not merely queue it. Full live pursuit-to-menu listening acceptance remains open. |

## Validation boundaries

The private validation record retains exact source/binary identities, commands, results and controlled
GPU captures. Native builds exclude the deprecated Bevy renderer. No playable package is updated.

The initial broader speech audit also exposed unrelated installed-data assumptions: a sentence
parameter assertion on both installs, and a recorded-line duration assertion on the modded install.
These baseline failures remain recorded. They do not invalidate the focused teardown checks, and
this pass does not claim that the complete speech test suite passes.

User acceptance stays open. A passing scripted route does not close a physical-input report or prove
matched original-game visuals, audio or timing.
