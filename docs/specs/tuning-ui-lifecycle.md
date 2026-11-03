# Tuning Help and exit lifecycle

Scope: Original UI tuning, playtest UI-021–023, with the scoped exit-input aspect of UI-035.
This contract uses the installed `Pause_Performance_Tuning.fng` data and remake runtime behavior, not decompiled code.

## Help line slots

The package has seven text slots, each with word-wrap justification and maximum width 270. Native scripts scale the
text objects to 0.8. The renderer wraps in font-layout coordinates before applying that object scale. Manual line
splitting must compare font-layout width with the same maximum width; multiplying by object scale first admits
strings that the renderer wraps a second time, causing lines to overlap adjacent slots.

Preserve the authored row positions, fonts and object scales. Measure with the same resolved font texture and font
scale as rendering. Scroll the resulting lines through the seven slots, clamp both ends, and keep Help read-only.
Changing text width policy does not certify all localization, popup clipping or scrollbar behavior.

## Replacement footer

Closing Help broadcasts the native message that shows its separate PC Cancel, Accept and Help roots again. For
`Footer::GarageTuning` only, presentation must suppress those complete roots on every composition, including the
native child key art. Preserve the shared footer's animation and binding-aware replacement hints. The Help popup's
own Hide hint uses `GarageTuningHelp`: one binding-aware Back/Hide hint at its native popup footer.
Suppress the popup's complete console and PC hint roots; Cancel, Accept and Help belong to the closed-popup state.

## Exit and recovery

The package's LEAVE_SCREEN response starts its leave scripts, plays its exit sound and disables native input. Its
event handler emits EXIT_COMPLETE at the authored end. Clean Cancel, Accept and Start must retain their requested
action until that completion; do not close the screen or save on the first frame and do not use a guessed timer.

While leaving, ignore further edits, Help toggles, clicks and wheel input. Dispatch the pending action once at
completion. Reopening a retained tuning screen after a failed save must restore its own native entrance before
allowing another attempt. Confirming changed tuning's discard closes its dialog and requests the retained tuning
page's same native leave sequence. Dispatch Cancel only after that page's EXIT_COMPLETE. No gameplay-save
eligibility or tuning persistence semantics are changed here.

## Validation

Installed regressions must reproduce double wrapping, PC footer reappearance after Help, and premature clean exit
before repair. Check all seven Help topics and scrolling, controller/keyboard footer composition after Help, and
clean Cancel/Accept/Start with input during the fade. Keep save-error recovery and existing discard controls passing.
CPU text-layout and presented-tree checks are not rendered or physical-controller acceptance.
