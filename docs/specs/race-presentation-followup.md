# Race intro and photo presentation

## Intro actors

Race intro camera tracks frame the authored primary car. Bind channel `car1`
to the human lead, including a custom grid where that person starts farther back.
Swap the actor of physical grid slot zero into the primary car's former channel;
leave the other assignments in grid order. This is a cinematic permutation only.
Physics placements and the randomized race starting order remain intact at GO.
Keep racers without an authored channel visible. Stage them relative to the
primary actor in unused grid positions; move a fallback position behind occupied
actors when needed. Restore physical poses on completion or skip. This fallback
is a host policy for incomplete animation tracks and extended fields, not a
recovered native algorithm. Ground placement retains contact clearance.

## Finish photo

The installed photo screen begins its cinematic after 0.75 seconds, then starts
its entrance after another 0.75 seconds **[PC reference, recorded sequence]**.
The first interval retains the driving camera while the car and world continue
moving with neutral player input. Freeze simulation at the camera cut, capture
the car transform there, and play the finish camera in real time. The host uses
a smooth simulation ramp from normal speed to a stop over that first interval;
the exact native post-finish vehicle controller remains unverified. Never hold
an unchanged driving image for the whole interval.
Hood/bumper views keep their normal body visibility until that cut, so the
delayed camera does not reveal the inside of the player's car.
At the entrance, start the package's handler, selected result group and picture
`APPEAR` scripts once. The frame appears while the text and footer animate in;
do not use the separate 1800 ms fade-out script or overwrite animated alpha
on subsequent frames. Show the footer on `INIT_COMPLETE`. Other result
groups stay hidden. Input becomes available when the 500 ms entrance completes.
The selected group's zero-length appearance leaves its host alpha at zero, so
hold that static parent opaque while retaining child animation. The frame and
text must reach visible held states, rather than fading back out.
Replacement controller/keyboard prompts inherit the native footer backing's
fade (normalized to its authored alpha 180), including its initially hidden state.

At sequence start, request the installed `game_camera` moment stream at the
listener. Keep its authored mixer and recording selection. At entrance,
request the authored `FX_Camera` sound in
`IG_GLOBAL/FX_Camera.abk` once. It uses eight class words, selector zero and
SFX volume. **[PC static]** This is misc sound 2, separate from menu acceptance
and the speedbreaker's `FX_UVES` class. Keep the existing audio master/SFX buses.
Do not replace absent data with an invented sound.

Returning from standings to the existing photo should retain its completed
entrance. Leaving the photo for standings keeps the photo underneath so its
artwork remains visible behind the result table. Suppress the covered photo's
footer, including native captions and replacement prompts, and its result text.
The winner/time/cash text must not show through standings rows. Restore these
groups when standings close. Only the active result screen offers buttons.

## Countdown audio delivery

The audible 3-2-1-GO effects must use the original recording. A PC runtime
capture identifies the start selector as speech event `0x4000`, with scene in
parameter 1 and section in parameter 3. Use the installed `0x4023` phrase bank
for start scenes. For `IntroNis06`, the complete and countdown/end sections are
takes 18 and 19. The previous 34-take bank was the arrest/end bank and did not
contain the required cue. Positive decoded output alone does not establish cue
identity; regression coverage must also check the native bank/take selection.

The host selects the installed
countdown/end section for the same scene at the countdown transition, after
normal completion as well as a skip or restart. Prefetch both sections while
loading the scene so this switch does not wait for decoding. Release the
request on race teardown. Each new playback request has a serial, including
a restart of the same scene and section. Audio must replay that request even
when it does not observe an empty request between two starts.
Exact original audio/camera clock alignment remains
an independent validation limit.

## Evidence and limits

See [provenance](../provenance/race-presentation-followup.md). Installed tests
must check visible frame alpha over time, input gating, primary actor identity,
grid restoration and positive audio output. Structural tests alone do not
establish perceptual parity for every camera, vehicle or race type.
