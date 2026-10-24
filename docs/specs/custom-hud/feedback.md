# Custom HUD: feedback, HUD audio and haptics

Short, event-driven feedback for the driving mechanics, the HUD's own sound cues and controller rumble. Built in PR 10;
every item is driven by a `HudEvent` ([notifications.md](notifications.md) section 1). No warning depends on sound or
vibration: everything here also has a visual form. Index: [README.md](README.md).

## 1. Visual feedback

| Feedback | Trigger | Where | Look and limits |
|---|---|---|---|
| near miss | `TrafficEvent::NearMiss` for the player | feedback lane above the cluster | a chip `Near miss`; at most one per 1.5 s; repeats inside that window refresh the chip and count up (`Near miss ×3`) without replaying its entry |
| speed trap | the race's speed-trap event (`RaceEvent::SpeedTrap`) | feedback lane | `Speed trap 212 km/h` (units follow `speed_units`), 2.5 s |
| bounty change | a bounty milestone or a bounty jump in the career | feedback lane | an animated delta `+2,500` counting up over 0.6 s, held for `delta_hold` (2 s), then gone; deltas inside the hold add up |
| nitrous activation | rising edge of `nos_burning` | the cluster's nitrous bar | one `pulse` ([telemetry.md](telemetry.md) section 6) |
| checkpoint passed | checkpoint event | the world marker and the strip's checkpoint field | one `pulse` in `success` |
| position change | new place | the strip's position field | one `pulse` |

The feedback lane holds one chip at a time; a newer chip of another kind replaces the older one after it has been up
for at least 0.5 s. There is no permanent stream of numbers. Drafting and drift scoring have no feedback: the
simulation has neither ([not-shown.md](not-shown.md)).

## 2. HUD audio

### 2.1 Volume groups

- **HUD effects:** a new `Group::Hud` in `crates/nfsmw/src/audio/volume.rs` (beside `Sfx`, `Music` and `Engine`), with
  its level from `[hud] hud_volume` (0 to 100, default 80), multiplied by the master volume like the other groups.
- **Police dispatch:** the existing `speech_volume` setting (the police dispatch's radio lines, default 90). It is not
  duplicated; the options row is relabelled `Police dispatch volume` in the menu (PR 10 / PR 11a). Its key and saved
  value do not change.

So the brief's "separate HUD and dispatch audio levels" are `hud_volume` and `speech_volume`.

### 2.2 Cues

Cues reuse sounds the install already has, through the existing triggers: the front-end bank
(`audio::menu::request`) and the `FX_Misc` bank (`audio/common_effects.rs`, `IG_GLOBAL/FX_Misc_MB.abk`). No new sound
asset is added. Which trigger each cue uses is chosen by ear in PR 10 and written in this table; until then every row
is **[unconfirmed]**.

| Cue | Event | Trigger (PR 10) |
|---|---|---|
| gear change | manual shift accepted (optional; off by default in Balanced) | to be chosen |
| nitrous on | rising edge of `nos_burning` | to be chosen |
| checkpoint confirmed | checkpoint passed | to be chosen |
| missed checkpoint | missed checkpoint event | to be chosen |
| critical damage | tyre blown, vehicle disabled | to be chosen |
| pursuit escalation | heat up | to be chosen |
| contact lost | phase `Active` to `Search` | to be chosen |
| cooldown | phase to `Cooldown` | to be chosen |
| escaped | phase to `Escaped` | to be chosen |

Rules: one play per event (the event is an edge, so a held state plays nothing more); a cue does not replay within
0.5 s of itself; a cue that comes with a notification plays when the notification is shown, not when it is deferred.
Cues are brief and distinct from the game's own pursuit music and speech, which keep their levels.

## 3. Haptics

- **API:** `GamepadRumbleRequest` from `bevy_gilrs` 0.20 (already a dependency of `crates/nfsmw`).
- **When:** only with `[hud] haptics = true` (default **false**: the brief asks for rumble only when the player turns it
  on) and only on a pad that reports force-feedback support. A pad without it is skipped silently.
- **Patterns** (PR 10), weak and strong motor intensities and a duration:

| Pattern | Use |
|---|---|
| `tap` (weak 0.3, 80 ms) | heat up, checkpoint missed |
| `thump` (strong 0.7, 200 ms) | tyre blown, vehicle disabled, busted |
| `double` (two `tap`s 120 ms apart) | roadblock ahead |

- The catalogue names the pattern per notification ([notifications.md](notifications.md) section 2). Rumble is sent to
  the driving player's pad; in split screen each player's pad is a follow-up, with the split-screen HUD.

## 4. Tests (PR 10)

- One cue per event edge; the 0.5 s repeat guard; no cue for a deferred notification until it shows.
- The near-miss throttle and count; the bounty delta accumulation inside the hold.
- No rumble with `haptics` off or on a pad without support; the pattern per catalogue entry.
- The `Hud` group follows `hud_volume` and the master volume.
