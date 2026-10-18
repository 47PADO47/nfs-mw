# Pursuit arrest and signed meter corrections

This supplements [ai-pursuit.md](ai-pursuit.md) and
[busted-sequence.md](busted-sequence.md). It separates the original's observed
source rules from the remake's fallback while arrest NIS playback is missing.

## Reference rules

**[decomp]** The CC0 reference's `AIPursuit::OnTask` changes to busted once its
timer exceeds 5. It disables the live pursuing cars' emergency lights and gives
them the stop-short goal. `AIVehicle::OnTask` overrides a busted perpetrator's
input: no throttle, no steering, no nitrous, full brake and handbrake.
`AIActionStopShort::OnTask` applies the same brake controls to cops.

Eligibility uses the pursuit's tracked in-sight state, including its seven-second
lost-contact grace. An immediate visibility-only gate changes original behavior;
it is no longer silently applied by the base world update. The separate horizontal
distance and 1.5-metre height gates still apply during that grace.
The manual-reset exception is also stock by default: the flashing reach is
90 metres and skips the speed gate. FairBusts changes that only when its config
explicitly enables the optional enhancement.

**[decomp]** The NIS message is delayed by three simulation seconds after the
bust. That delay is separate from the NIS animation's duration. Arrest scene
playback, placement and the original career transition remain incomplete here.
The remake uses the no-NIS fallback: three seconds of held controls, followed by
the infractions ticket directly. There is no additional invented overlay delay.
The fallback retains the stopped police and holds the player's controls until
the infractions screen has accepted payment. A completed ticket must
release those controls, clear the terminal busted flag and meter, and permit
another pursuit. Removing retained cops on payment is a remake fallback; it is
not an original-game fade or return-to-career implementation.

**[decomp]** The five-part pursuit bar consumes a signed value, not just the
arrest timer. With immediate contact, distance and speed determine the orange
warning region. A distant unit drives the value toward escape, and increasing
unseen time drives it from -0.5 toward -1. The red region blends the timer with
that warning value. `TimeUntilBusted` returns -1 during displayed cooldown when
there is no remaining bust timer. The display clamps at 0.99 until busted.
See `ai-pursuit.md` section 4.4 for the formula. Cooldown visibility follows its
state flag, including its first zero-valued display update.

**[decomp, native corroboration]** The variable-task scheduler accumulates frame
time, advances its frequency counter per frame, and invokes a variable callback
with the accumulated time when the counter reaches one. The pursuit logic runs
every fourth frame and the timer integration every frame: the increment held
between decisions is one quarter of the logic delta. Thus the existing steady
+1/-2 effective timer rates are retained. This proves the scheduling conversion;
it does not provide a matched frame-by-frame gameplay capture.

## Regressions to cover

- Held throttle, steering, nitrous, reset and shift buttons cannot move or reset
  the arrested player, and Speedbreaker cannot extend the fallback delay.
- Live police remain present, brake fully and stop flashing during arrest.
- Payment clears transient arrested/busted state and removes fallback police;
  rejected payment leaves the ticket and controls held.
- The bar has positive proximity warning before a timer starts, transitions
  negative when sight is lost, and returns 1 only once the timer has busted.
- A new chase after payment starts with an empty timer and a fresh camera edge.

The reference is evidence of source behavior, not a matched gameplay capture.
