# Custom HUD: HUD audio and haptics, as built

The Custom HUD's own sound cues and controller rumble, as PR 10 built them. The design is
[custom-hud/feedback.md](custom-hud/feedback.md) sections 2 and 3; this page records the choices the code made: which
install sound each cue plays, the volume group, when a cue plays and when the pad rumbles.

- **Sources read:** `crates/nfsmw/src/audio/menu.rs` and [frontend-audio.md](frontend-audio.md) (the front-end bank
  triggers and which of them are audible), `audio/common_effects.rs` and [common-audio-events.md](common-audio-events.md)
  (the `FX_Misc` bank), `audio/oneshot.rs`, `audio/volume.rs`, `audio/plugin.rs`, and `bevy_input` 0.20
  (`gamepad::GamepadRumbleRequest`, `GamepadRumbleIntensity`).
- **Built in:** `crates/nfsmw/src/hud/notify/output.rs` (the cue table, the rumble patterns and the system that plays
  them), `hud/notify/signals.rs` (once per notice), `audio/menu.rs` (`request_hud`), `audio/volume.rs` (`Group::Hud`).
- **Status:** built. **Checked by ear: pending.** Every row of the table below is a first choice from the triggers the
  installed banks are known to play; nobody has listened to them in a race or a chase yet.

## 1. Volume groups

| Group | Level | Notes |
|---|---|---|
| `Hud` (new) | `[custom_hud] hud_volume` (0 to 1, default 0.8) | its own mixer sub-track; the master volume multiplies it like the others (`Volumes::effective`) |
| police dispatch | `speech_volume` (0 to 100, default 90) | unchanged; the Audio options row now reads **Police Dispatch** (`options-police-dispatch-volume`), the key and the saved value are the same |

The HUD group also has its own slot on the wet bus, at the HUD level, though the front-end graphs send nothing wet.

## 2. Which sound each cue plays

Cues reuse the front-end banks' authored triggers through the menu player (`audio::menu::request_hud`): triggers below
23 are `PlayCommonSample` in `GLOBAL/FE_COMMON_MB.abk`, the others `PlayFrontEndSample` in `FE/FE_MB.abk`
([frontend-audio.md](frontend-audio.md)). Only eight triggers are known to be audible (0, 2, 4, 5, 7, 10, 11 and 89), so
for ten cues two pairs share a sound. The `FX_Misc` bank (`common_effects.rs`) holds only the speedbreaker's and the
visual pulse's graphs, which would be mistaken for the speedbreaker, so no cue uses it. No new sound asset is added.

| Cue (`AudioCue`) | Catalogue events | Trigger | The original's message | Checked by ear |
|---|---|---|---|---|
| `Checkpoint` | checkpoint passed, lap, speed trap passed | 0 | SOUND_UP / SOUND_DOWN | pending |
| `Nitrous` | nitrous on | 2 | SOUND_LEFT / SOUND_RIGHT | pending |
| `Escaped` | escaped | 4 | SOUND_ACCEPT | pending |
| `MissedCheckpoint` | wrong way | 5 | SOUND_BACK | pending |
| `CriticalDamage` | tyre punctured, heavy damage, vehicle disabled | 7 | SOUND_BUTTON0..5 | pending |
| `Busted` | busted | 7 | SOUND_BUTTON0..5 (shared: the bad ends) | pending |
| `Escalation` | heat up, helicopter tracking | 10 | SOUND_DLGBOX_IN | pending |
| `ContactLost` | phase Active to Search | 11 | SOUND_DLGBOX_OUT | pending |
| `Cooldown` | cooldown started | 11 | SOUND_DLGBOX_OUT (shared: the chase calming down) | pending |
| `Hazard` | roadblock reported | 89 | SOUND_MAIN_SUB (the front-end bank's sample 21) | pending |

The gear-change cue of the design is not built: the design makes it optional and off by default, and no setting turns
it on. It would be an edge on `HudState.gear` if one is added.

## 3. When a cue plays

- **Once per notice.** A notice's cue is raised when it comes up on the screen (`HudNotifications::tick` compares the
  visible notices' `started` with the last frame's). A repeat folded into a visible notice keeps its `started`, so it
  plays nothing; a notice that waits for room plays when it shows, or never if it goes stale first.
- **Feedback at once.** A kind with no notice (the nitrous) and a chip of the feedback lane (the speed trap) raise their
  cue when the event comes.
- **No stutter.** A cue does not play again within 0.5 s of itself (`signals::CUE_GAP`): three damage notices at once
  play one cue.
- **The Custom HUD's.** Cues play only with `ui_style = custom`, the HUD option on and the HUD shown. Under Original and
  Remastered the raised cues are dropped each frame, so none plays late.
- **On the main thread.** The HUD's output system takes a `NonSendMarker`, because the menu player's queues are
  thread-local and the audio plugin takes them in `FrameSet::Ui`, later in the same frame.

## 4. Haptics

`GamepadRumbleRequest::Add` (`bevy_input` 0.20, a `Message`; `bevy_gilrs` plays it) to the driving player's pad
(`InputPresentation::active_pad`), only with `[custom_hud] haptics = true` (default false) and a pad connected. A pad
without force feedback is skipped by `bevy_gilrs`, which logs it at debug level.

| Catalogue `Haptic` | Pattern | Motors | Length | Events |
|---|---|---|---|---|
| `Light` | `tap` | weak 0.3 | 80 ms | heat up, wrong way, escaped, nitrous on |
| `Strong` | `thump` | strong 0.7 | 200 ms | roadblock, busted, tyre punctured, heavy damage, vehicle disabled |

The design's `double` pattern (two taps 120 ms apart for the roadblock) is not built: the catalogue gives the roadblock
`Strong`. Rumbles follow the same once-per-notice rule as the cues; they are not held back by the 0.5 s guard.

No warning depends on sound or vibration: every cue and rumble comes with a notice, a chip or the nitrous bar's pulse
on the screen.

## 5. Tests

- `audio/volume.rs`: the HUD group is its level times the master, and moves the HUD only.
- `audio/menu/tests.rs`: HUD cues queue apart from the menu sounds.
- `hud/notify/feedback_tests.rs`: one cue and rumble per notice, none on a refresh, a waiting notice plays when it
  shows, the 0.5 s guard, cues only in the Custom style, every cue on an audible trigger, rumble only with `haptics` on
  and a pad.
- `frontend/options_tests.rs`: the speech volume row reads Police Dispatch.
