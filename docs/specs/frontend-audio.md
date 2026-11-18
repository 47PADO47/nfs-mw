# Front-end menu sounds

Sources: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0,
reference only), `FEMenuScreen.cpp`, `EAXFrontEnd.cpp`, `AemsDef.hpp` and
`SND_GEN/FE_AEMS.h`. PC 1.3 binary inspection independently confirms the
common mapping and dispatch boundary. No source or decompiler code is copied.

## Dispatch

**[verified, PC 1.3]** The menu sound messages map to trigger IDs:

| Messages | Trigger |
|---|---:|
| SOUND_UP, SOUND_DOWN | 0 |
| SOUND_LEFT, SOUND_RIGHT | 2 |
| SOUND_ACCEPT | 4 |
| SOUND_BACK | 5 |
| SOUND_BUTTON0 through SOUND_BUTTON5 | 7 |
| SOUND_DLGBOX_IN | 10 |
| SOUND_DLGBOX_OUT | 11 |
| SOUND_INIT | 28 |
| SOUND_MAIN_MENU | 88 |
| SOUND_MAIN_SUB | 89 |

The hash names here use the FEng uppercase hash. These messages must go to
sound playback as well as any game logic interested in the pressed object.
The PC dispatcher recognizes 142 hashes. Deliver every authored sound post
through the stock lookup, then through its screen's sound filter. An input
fallback can use the port's semantic bindings when there is no corresponding
authored cue. An authored post that the screen suppresses still takes
precedence over equivalent fallback feedback.
Unknown messages and releases are silent. The port's runtime emits PAD_* names;
its host maps these semantic equivalents to the same common triggers. A mouse
activation maps BUTTON_PRESSED to trigger 4. Coalesce equivalent input
fallback events from the same package in one frame.
The six semantic PAD_BUTTON0 through PAD_BUTTON5 events map to trigger 7, as do
their SOUND_BUTTON counterparts. Other auxiliary buttons have no generic mapping.
Direct pointer paths request feedback after an accepted action. Disabled icons,
misses, row selection without an edit and settings already at their limits stay
silent. Successful direct settings edits request directional feedback once.

**[verified]** Triggers below 23 use `PlayCommonSample` in
`GLOBAL/FE_COMMON_MB.abk`. Higher triggers use the front-end graphs described
below in `FE/FE_MB.abk`, with 23 subtracted from the trigger. Four class words are
ID, volume in 0..32767, pitch (4096 = normal), and azimuth (0 = centre).
Run the authored graph; IDs are not raw sample indexes. Keep an instance until
its sound finishes. Set its destructor only after its first tick has run.
The existing interpreter handles the bank's sample choice, pitch and duration.
Keep parsed module banks cached with the audio system. Repeated navigation creates
an independent module instance without rereading or reparsing the bank.

### Pursuit, results, milestones and car selection

**[exe]** Package sound messages go through the generic trigger table, then through
the owning screen's sound filter. An unrecognized message starts with no trigger.
The screen can keep, replace or suppress the trigger before the audio player sees it.

| Sound message | Package | Trigger | Bank slot | Sample index |
|---|---|---:|---|---:|
| `8F7385AB` | `PostRace_Pursuit.fng` | 110 | Front-end 87 | 30 |
| `4A805994` | `PostRace_Results.fng` | 2 | Common 2 | 2 |
| `91970159` | `InGameMilestones.fng` | 21 | Common 21 | 17 |
| `B6471CC6` | `Car_Select.fng` | 90 | Front-end 67 | 22 |
| `4ADDE92C` | `Car_Select.fng` | Silent | No asset | |
| `B11FB113` | `Car_Select.fng` | 147 | Front-end 124 | 63 |
| `50B3FA79` | `Car_Select.fng` | 135 | Front-end 112 | 51 |
| `44D43738` | `Car_Select.fng` | 145 | Front-end 122 | 61 |

**[install]** The pursuit script `2CB4DBD0` posts eight cues, one every 60 script
ticks from 0 through 420. Preserve each cue when several arrive in a single
frame. **[exe]** The generic alias `D792FF31` also selects trigger 110.

**[exe]** Race results suppress `4A805994` and `9AFA53A7` with at most one
racer or on the standings page. They also suppress those two messages and
`7B6B89D7` if object `57EFB2FB` has its HIDE script set.
**[install]** Two responses in the results package send `4A805994`, in
answer to `5073EF13` and `D9FEEC59`.

**[exe]** The pursuit and milestones scroller filters leave the listed cues
unchanged. The car selector only filters vertical navigation sound messages.
The generic table leaves `4ADDE92C` without a trigger and the car selector
preserves that result.

**[install]** The nine milestone scripts `001B20C2` through `001B20CA` on
`D4FEBDB2` each post `91970159` at tick 0. In car select, `5A8E4EBE` posts
`B6471CC6` at tick 1000 or 2200 and `4ADDE92C` at tick 1000, depending on the
object that runs it. On `2A187249`, `0013C37B` posts `B11FB113`, and
`CB1AA27F` and `E046BB3E` post `50B3FA79`, all at tick 0. Object `E1E1B4D9`
posts `44D43738` in response to Showcase transition `89D0649C`. The car lot's
direct Showcase switch requests this same cue after it accepts the car.

The trigger names in `AemsDef.hpp` identify the shared recordings. Trigger
110 is Rap Sheet Move Bar Up, 2 is Common Left, 21 is Map Zoom In, 90 is
Impounded, 147 is Car Select Enter, 135 is Customize Exit, and 145 is
Car Select to Showcase. A screen can reuse another screen's recording.

**[exe] [install]** `SafeHouseMilestones.fng` posts `1CFAD52B` for its
matching nine scripts. It plays common trigger 20 (sample 16). Its entrance and
exit post `FA274177` and `BA0159B3`. They play front-end triggers 128 and 129
(samples 44 and 45). Pause opens the in-game milestone sheet and the safe
house opens its own twin.

### Screen sound filters

**[exe]** A screen may replace the candidate even when the generic table has
no trigger. Use the screen's current state when the authored sound post
arrives. Conditions read from the PC handlers include:

| Screen | Sound rule |
|---|---|
| Pause | Suppress Back when the caller was post-race. |
| Blacklist overview | Suppress accept when the shown rival was defeated or the career is complete. |
| Race sheet | Replace accept with trigger 7 until the player activates an available race on this sheet. |
| Rival bio | The raw Back pad message is silent. The authored Back cue uses the generic trigger. |
| Milestones and bounty | In-game screens suppress accept. The bounty filter has a trigger 7 branch for locked entries. The PC bounty constructor only adds available places and initializes them unlocked. |
| SMS reader | Suppress the replay cue for a message without a voice. |
| Save and alias | Suppress horizontal feedback when no button is visible. |
| Icon scroller and widgets | Horizontal feedback requires a move by the icon strip or the selected widget. A widget menu without a selection preserves the cue. |
| Array scroller | A direction reads and clears the movement latch. SMS excludes horizontal directions. Challenge Series and My Cars consume the latch but preserve the original candidate except for Back, which returns 5. |
| Splash | Suppress cues before the screen is ready. |
| Dialog | A horizontal cue requires a focused button whose name differs from the cached button. Back requires a dismissible dialog. |
| Car Select | Suppress vertical cues in career, customization and car lot modes. |
| Paint | Suppress paint-type cues 41 and 42 for rim paint or a selected remap part with fewer than two colors. |
| Performance | Replace accept with 49 for Engine, 53 for Transmission, 50 for Suspension or Brakes, 52 for Nitrous, 54 for Tires and 51 for Induction. |

**[exe] [install]** Car lot purchase answers use the shared dialog cue.
The dialog button posts `7B6B89D7` in response to `BUTTON_PRESSED`, selecting
common trigger 4. The two purchase callbacks update the stable and cash.
They have no separate purchase trigger.

### Rap sheet and mix state

**[exe] [install]** Triggers 103–109 and 112–118 use
`PlayFrontEndSample_RS` with the same subtraction of 23. A new request for
that class replaces its held instance. The two movement cues, 110 and
111, use the ordinary independent graph. Accept during the first or second
login animation stops its held trigger, 103 or 104, if it is still playing.

**[exe] [install]** The common player reads FEHUD volume slot 1 and the
front-end player slot 2. The installed PC map adds slot presets of -4.22 dB
and -5.33 dB to a shared -10 dB channel. It connects the channel to the
sound-effects setting and the FMV state.
The common player consumes an audio slider preview value on its next cue
and multiplies the gain in Q15. It also checks audio availability, the
FEHUD output block and a retail debug-only flag. The remake uses the
authored base, its SFX and master buses, and a one-shot preview multiplier.
The stock master-volume curve and controller outputs outside this path
need further work.

### Photo and movie sound states

**[exe] [install]** The named `SoundPause` reasons are no-op calls in the PC
binary. Photo finish sets the cinematic fade state at construction, clears
it at the shutter transition and enters the Pause sound state. Destroying
the screen clears Pause. The boot movie screen sets FMV on entry and clears
it at completion or an allowed skip. Movie completion also clears Pause
in the player. The state-action table gives Pause `0x3F` and FMV `0x7F`.
The first three actions pause Pathfinder music, speech and NIS/SFX while
their action bits are set. A channel resumes when every reason asserting
its action clears. Other bits drive authored mixer controls. The installed
map attenuates player car, other cars, traffic, ambience and collision
outputs by 100 dB in these states. FMV also suppresses FEHUD cue output,
while photo Pause leaves it available. The cinematic fade uses an authored
59-tick attack at 60 Hz before the shutter switches to Pause.

The remake routes world effects through a dedicated track and mutes it
and the engine track while the photo pause or movie is active. Menu cues
remain available during the photo. The movie soundtrack plays independently
of the world mute. The original's pre-shutter fade and individual stream
pause/resume timing still need separate implementation.

**[ours] Diagnostic correlation.** An opt-in trace records one `menu_trigger` per serviced trigger, with its global
ID, mixer group, bank, class and class words. A successful load records the actual module instance; failed or cached
unavailable loads have no successful instance. An authored zero recording emits `voice_silent` for that instance.
Its `success: false` records the existing no-voice return, not a decoder or backend failure.
These events do not identify the UI request that queued a trigger, isolate mixed PCM, or establish perceptual parity.

## Scope and limits

The FEHUD base gain and the installed module routing are covered by offline
checks. The original dynamic mix and remaining screen-specific gates still
need comparison. Car drive-on sounds and a matched original-game
audio trace remain open. The installed modded
executable differs from the clean PC 1.3 analysis image; no address is used
against the modded running process.

## Authored silent triggers

**[verified, installed PC banks]** The common graph copies class ID directly to
all three players' sample selections. Its 23-entry group has a zero sample index
at trigger 6 (Start/Pause). The front-end graph also selects directly by class ID;
its 137-entry group has zero at slot 5 (trigger 28, SOUND_INIT) and slot 65
(trigger 88, SOUND_MAIN_MENU). Trigger 89 selects slot 66, which references sample
21. The common audible triggers 0, 2, 4, 5, 7, 10 and 11 reference samples
1, 2, 3, 4, 5, 8 and 9 respectively. A zero index is authored padding; no
replacement cue is invented for these events.

The installed playback regression checks the graph's sample references as well
as rendered output. It requires audio for nonzero references, silence for these
reserved entries, and natural completion for every supported cue at 30, 60 and
144 FPS. It aggregates results so one silent event cannot hide later defects.
