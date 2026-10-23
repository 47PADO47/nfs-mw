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
Unknown messages and releases are silent. The port's runtime emits PAD_* names;
its host maps these semantic equivalents to the same common triggers. A mouse
activation maps BUTTON_PRESSED to trigger 4. Deduplicate equal trigger IDs from
the same package in a frame, including explicit SOUND_ACCEPT, PAD_ACCEPT and
BUTTON_PRESSED announcements of one gesture. This policy is port-specific.
The six semantic PAD_BUTTON0 through PAD_BUTTON5 events map to trigger 7, as do
their SOUND_BUTTON counterparts. Other auxiliary buttons have no generic mapping.
Direct pointer paths request feedback after an accepted action. Disabled icons,
misses, row selection without an edit and settings already at their limits stay
silent. Successful direct settings edits request directional feedback once.

**[verified]** Triggers below 23 use `PlayCommonSample` in
`GLOBAL/FE_COMMON_MB.abk`. Higher triggers use `PlayFrontEndSample` in
`FE/FE_MB.abk`, with 23 subtracted from the trigger. Four class words are
ID, volume in 0..32767, pitch (4096 = normal), and azimuth (0 = centre).
Run the authored graph; IDs are not raw sample indexes. Keep an instance until
its sound finishes. Set its destructor only after its first tick has run.
The existing interpreter handles the bank's sample choice, pitch and duration.
Keep parsed module banks cached with the audio system. Repeated navigation creates
an independent module instance without rereading or reparsing the bank.

## Scope and limits

Initial implementation covers navigation, acceptance, back, dialog and main
menu transitions. The port uses its SFX/master buses and full-scale module gain;
the original FEHUD mix-map gain and screen-specific overrides remain unverified.
Rap-sheet sustained sounds, screen-specific effects, car drive-on sounds and
an audible matched original-game comparison remain open. The installed modded
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
