# Front-end lifetime boundaries

Scope: UI-024, UI-028, UI-033–035 and the initial My Cars loading state of UI-018.
Existing PC evidence is in [career menus](../provenance/career-parity-menus.md),
[race restart](../provenance/race-restart.md), [safe house](career-safehouse.md)
and [front-end audio](frontend-audio.md). These robustness policies do not establish full original-game parity.

## Gameplay effects and police speech

**[ours]** A renderer focus filter belongs to its scene. Both a newly initialized scene and a restored parked menu
start with the default filter. Saved motion-blur settings remain unchanged. A driving scene supplies its next filter
when it updates. This covers radial blur and speedbreaker vignette, including repeated gameplay/menu round trips.

**[ours]** A scene change stops the active police speech line, clears its dispatcher, and resets chase speech,
weather-edge, 911 and pursuit-music input state. Loaded speech recordings, tuning, event mapping and volume stay usable.
Menu effects and licensed songs keep their separate players. A full speech reset also forgets the active dispatcher
entry, completion notices and pursuit timing; a stopped old line cannot repopulate history in the next session.
The console's history/queue reset retains its former narrower scope.

Pausing the same world preserves speech state. It freezes police-machine processing before notices or completion
events are consumed, avoids draining roam events, and holds pending 911 delivery until resume. The music context
continues following the frozen pursuit. Dispatcher playback remains held by its existing pause system.

## Pending menu transitions

**[ours]** A screen whose logic owns a pending exit can refuse host input. Refusal gates pad processing, raw object
posts, pointer selection, wheel edits and Start-device delivery. Native exit scripts and completion messages keep
running. An icon carousel refuses input after its accepted exit until the destination or a retained-screen return
restores control. It must not restart the exit or request automatic feedback for a rejected repeated gesture.
Settings pages also refuse host input while leaving, so generic pointer focus/release fallback and raw object posts
cannot bypass their native package input switch.
Track selection also gates its host-started Accept animation, whose native response does not disable package input.
Garage carousels reject supplemental pad actions during exit. Garage dialogs with a pending activation refuse pointer
answers until their scheduled completion; a second click cannot replace the accepted choice.
Text edits also obey pending-state refusal, but remain independent of the navigation release latch. A name prompt
can receive characters or Backspace while its opening action is held; Accept and Back still require that release.

Showcase camera directions have no automatic navigation sound. The filter affects only semantic cardinal/diagonal
pad direction sounds on Orbit screens; explicit sound messages and Back remain available.

## Pause background

**[verified, installed PC package]** `InGameBackground.fng` is separate from `Pause_Main.fng`. Its ten objects include
a central `BasePoly` image and mirrored side gradients under group `0x2716CDBF`, plus top/bottom gradient groups.
The package supplies its own colour tracks, alpha, entrance timing and geometry. No additional opacity is chosen.

**[ours]** Load it once beneath the pause stack. Keep it visible beneath a covering tuning page and Help/dialog
overlays; their parent menu is still hidden by the normal covering-page policy. Clearing the pause stack unloads it.
The native pause/options packages themselves have no equivalent full-screen backing. Rendered comparison and exact
reference transition timing remain unverified.

## Career pause shortcut

**[exe]** Career free roam below prologue stage 16 offers Jump to Safe House. The original also has race variants;
their event-flag conditions are documented in the race-restart provenance. Those race variants remain unsupported.

**[verified, installed language table]** Free-roam confirmation uses label `0x6925D0BE`. Pursuit confirmation uses
`0x15A1B5A9` and warns that pursuit progress is lost. Both are runtime text from the user's install.

**[ours]** The shortcut is absent outside active career free roam at stages 1–15. It stays unavailable during a co-op
session because safe-house guest handling is not implemented. Acceptance rechecks context before unloading the world.
It preserves live driven-car heat, reverts only unbanked milestones, and clears pursuit statistics/summary. It does not
award an evade, charge a bust, add a strike or pay bounty. Completed milestones remain complete. The normal safe-house
menu then owns the flow; Resume Free Roam uses the existing last-safe-house marker/start-point path.

Cancel retains the pause carousel and its selection. Repeated acceptance cannot repeat the world transition.

## Initial My Cars load

**[ours]** My Cars shows the install's native loading screen while its scene loads. The screen continues updating.
Back cancels after the entry gesture has been released. A loader or initial-preview failure returns to the main menu
with a notice instead of entering a garage with no usable car. This does not establish car-browsing latency targets.

## Verification limits

Focused regressions cover filter reset, active/queued speech cleanup, chase-session reset, repeated exit input,
safe-house context/confirmation/destination, unbanked milestone discard and loading cancellation. Headless renderer,
synthetic sink and installed-package tests establish their own scopes. Physical input, audible comparison, rendered
transition acceptance and a complete drive/menu cycle still require playtesting.
