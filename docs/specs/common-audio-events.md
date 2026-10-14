# Common speedbreaker and pursuit-breaker effects

The Speedbreaker follow-up distinguishes visual UVES pulses (input 1, class ID 0)
from player Speedbreaker begin/end (inputs 3/4, class IDs 2/1). The original
internal message name also contains "pursuit breaker". See
[the full mode spec](speedbreaker-audio.md); prior activation-only wiring is
superseded by that verified dispatch.

Sources: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw)
(CC0-1.0), `SFX_Common`, generated `FX_UVES` class, main sound-object enums;
independently checked against matching PC message producer and complete
process-update instructions. Provenance:
[common-audio-events.md](../provenance/common-audio-events.md).

`FX_UVES` is not the tyre-landing controller. Tyre landings use collision
stitches. This class belongs to global common speedbreaker/pursuit events.

| Semantic event | Message input | FX_UVES ID | Main object4 volume slot |
|---|---|---|---|
| Speedbreaker activation | 1 | 0 | 2 |
| Pursuit-breaker start | 3 | 2 | 3 |
| Pursuit-breaker end | 4 | 1 | 4 |

The global bank is `IG_GLOBAL/FX_Misc_MB.abk`. Class words are ID, volume,
width, pitch offset, intensity and stop. Constructor values are the selected
ID followed by five zeros. Process update changes volume only, from the
corresponding authored main-state mixer output. Bounds: ID0..2,
volume/width0..32767, pitch offset−16383..16383, intensity0..1040, stop0..1.
Do not invent speedbreaker global pitch slowdown from this class.

Messages pulse the corresponding object input32767. A speedbreaker message
creates its graph only when the previous speedbreaker graph is absent.
Pursuit-breaker start/end messages replace their own previous graph. Each
family has an independent lifetime; finished graphs are removed. Main input
words0..4 are cleared after processing, while authored mixer event envelopes
continue. Retain pulses until a controller tick has consumed them.

The remake uses its shared fixed60Hz controller clock with retained fractional
debt, bounded work and deferred within-frame sample starts. Rendering at a
higher FPS must not duplicate events or shorten envelopes. Missing bank/class
or mixer is reported as unavailable; no unrelated raw sample is substituted.

Validation requires nonzero installed class starts and PCM for all three IDs,
finite output, graph release and stable timing/levels across render rates.
This verifies implemented selection/lifecycle, not original global audio
time dilation or callback/DSP equivalence. Camera shot/radar effects and
world ambient/static callers remain separate families.
