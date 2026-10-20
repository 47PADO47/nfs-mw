# Provenance: the free-roam 911 call

Date: 2026-10-20. Specs: [ai-pursuit-heat](../specs/ai-pursuit-heat.md) section 5.3,
[ai-pursuit-speech](../specs/ai-pursuit-speech.md) section 8, [ai-pursuit-cops](../specs/ai-pursuit-cops.md) section 1.

The code was written from those specs. The behaviours below were learned by reading the public CC0 reconstruction
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) for understanding; nothing is copied or translated, and no
decompiler names appear in the repository.

| Behaviour | Read in | Notes |
|---|---|---|
| The speech counts the cost to state of every smackable the player collides with (`CTS911`) while no pursuit is active or searching; the same collision callback counts traffic hits whose intensity reaches `MinIntensityTrafficSmash` (`TrafficHits911`); both are zeroed whenever the pursuit state is active or searching | `Speech/SoundAI.cpp` (the collision handler, the pursuit sync) | The intensity is `clamp((closing speed - low) / (high - low))` over `PlayerSmashSpeedRange` in mph; the rewrite feeds the closing speed of the two bodies. The original counts each collision callback; the rewrite counts a prop or a car again only after a second without contact. **[inferred]** |
| The report is requested when `CTS911 >= CTSFor911` or `TrafficHits911 >= NumCiviHitsFor911`, in free roam, with cops enabled, no pursuit and no 911 already active, and not in a D-day race | `Speech/SoundAI.cpp` (the main update) | Already in `blackbox_pursuit::speech` (`sync_player.rs`); the world now supplies the notices and the level. |
| When the report line has been said: the 911-active flag, the perpetrator's 911 timer set to `Lifetime911` (the larger of the old and the new), and `LockoutCops(false)` | `Speech/Observer.cpp`, `Speech/SoundAI.cpp` (`Force911State`), `AI/Activities/AICopManager.cpp` (`LockoutCops`) | `Effect::Force911` leaves the machine, `Audio::take_911_call` hands it on, `TrafficWorld::start_911_call` clears the lockout and starts the timer. |
| The perpetrator's timer runs down every update; a patrol cop that sees a target whose timer is above zero keeps it, with no speed or heat condition | `AI/Common/AIVehicle.cpp` (`AIPerpVehicle::Update`), `AI/Common/AIVehicleCopCar.cpp` (`WatchForPerps`) | `Sighting::has_911` is the timer. The 65 mph, recent hit and heat rules were already in. |
| The speech is reset (flags, counters) when the cops are restarted | `AI/Activities/AICopManager.cpp` (`ResetCopsForRestart`), `Speech/SoundAI.cpp` (`ResetPursuit`) | The rewrite sends the machine its restart notice when the cops are locked out again after an evade or a paid ticket, so a second call can be made. The original resets on a restart, not on an evade's `LockoutCops(true)`; whether it can call 911 twice in one session is not established. **[inferred]** |

Rewrite-only: when the speech never answers (speech off, no audio device, the report line never finishes) the world
makes the call itself after 20 s of the thresholds holding, so the cops are not locked out for lack of a sound card.
The original has no such path (the call is made by the speech). A development shortcut `pursuit 911` makes a call at once.

The `COST_TO_STATE` of the installed smackables is read from the `smackable` collections (166 of 181 have a cost; the
highest is 3000); the heat rows' `CTSFor911`, `NumCiviHitsFor911` and `Lifetime911` equal the heat spec's table
(installed tests). Not verified in a running original: the thresholds as a play experience, how often the original's
collision callback fires on a continuous contact, and the 911 call in the original's HUD or radio by ear.
