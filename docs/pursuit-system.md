# Police and pursuits: implementation and parity

Status: **incomplete integration with known gameplay failures**. This contribution
is suitable for review as a draft; passing isolated regressions does not make the
playable pursuit loop complete or establish original-game parity.

This is the current map of the playable free-roam pursuit system. It supersedes
older milestone lists that describe all police as placeholders, or claim the
existing pursuit modules are complete. A feature being wired and tested does
not establish original-game driving, visual or listening parity.

The implementation reads assets from an installed game. It includes a free-roam
pursuit loop; it does not provide the original career progression.

The correction pass compared the public original-source reconstruction with
the installed PC executable and authored HUD animation tracks. Native evidence
corroborates independent patrol counts, traffic/search speeds, tracked-sight
grace, radar selection, the signed meter and busted timer scheduling. Twelve
isolated executions of the original meter routine corroborate its arithmetic.
These are static/instruction-level results, not a live original-game comparison.

## Latest playtest issues

The following issues were reported after testing the correction build. They
remain open; this contribution does not claim to fix them. The isolated tests
below do not override these integrated gameplay observations.

| Area | Outstanding issue |
| --- | --- |
| Cooldown | **Cause found and fixed, not replayed live.** The HUD bar was fed the escape meter (`progress / required`), which already holds the 7 s grace, so it opened about a third full. The board's bar is `1 - remaining / (required - grace)` and opens empty. Unit and world tests cover it; no live comparison. |
| Arrest/payment | **Cause found and fixed, not replayed live.** The ticket screen took the two pad-direction messages its package forwards for the payment presses, so accept did nothing and the world stayed frozen under the ticket (the package posts nothing on a payment press). The payment now follows the pressed button. An installed test drives the arrest hold, the ticket, accept on the real `Infractions.fng`, the resume and driving off. The original also continues to the car select screen, which the rewrite does not have. |
| Police radio | No audible chase radio in the playtest. ID lookup is corrected, but composite sentence playback is missing; cue lookup/queue tests do not prove audible speech. |
| Police driving | Poor pursuit decisions and incorrect police vehicle physics/damage remain. The short single-unit fixture does not establish competitive AI or collision/damage parity. |
| Pursuit breakers | No working pursuit breakers were available in the playable test. Existing event-system code is not evidence that the complete pursuit interaction works. |
| Hiding spots | No working hiding spots were available in the playable test. The existing hiding/cooldown state does not establish authored spot discovery, presentation or gameplay integration. |
| Sirens | Ground police siren playback is not implemented. |
| Spawning | Several causes found and corrected, not replayed live: patrols were only probed in a wedge ahead of the player and never checked after the road snap; wave cops were never recycled except 500 m away; there was no lockout, no finite wave and no surplus removal. See the Patrol population, Search and Backup rows. Forced chase/test setup may still contribute; no live comparison was made. |

## Implementation changes and isolated regressions

| Problem | Result |
| --- | --- |
| Police-looking traffic kept cruising after a chase started | Nearby live patrol units become pursuit drivers in place, keeping their identity, speed and damage. Distant patrols keep roaming. |
| Patrol count depended on civilian density | Installed heat-row `NumPatrolCars` sets a separate police target. Positive legacy `cop_share` enables it; zero disables it. Normal patrol speed is 35/55 mph; search uses installed search speeds. |
| Patrol detection always used heat 1 | Session vehicle heat governs detection and the next pursuit's starting heat. At heat 4 or higher, being seen suffices; below that, speeding or a recent witnessed vehicle hit triggers it. |
| Every searching cop stopped at the last observation | Tracked sight retains the original seven-second grace. Ordinary searching units then navigate traffic lanes; reacquisition switches the same physical unit back to pursuit navigation. Search replacements use the patrol cap. |
| Elevated sightings were rejected indiscriminately | Sight uses full-3D range/cone and resident world occlusion. The 1.5 m height gate belongs to arrest proximity. Hiding knowledge is latched per cop. Wrecks cannot report sightings. |
| The helicopter could see the player while the escape timer advanced | Helicopter sight contributes to shared visual contact. Its visibility ray uses sight collision policy, including decks and buildings. |
| Radar placement and detection were guessed | Removed invented layout offsets; authored animation tracks and opacity drive standalone/mirror variants. Native targeting selects the nearest live ground cop within 300 m and a full-3D ±45 degree forward cone, without a separate deck/sight-ray gate. LED timing follows the native per-update cycle. |
| Enabling the mirror could crash before displaying the radar | The reduced view now returns an SDR RGBA8 UI image even with an HDR main output, restoring the main output and display mapping after submission. |
| Busted meter and arrest were broken | The signed proximity/timer blend drives five authored segments. Arrest overrides driving/reset/NOS, cancels Speedbreaker and stops cops with lights off. The isolated payment fixture clears arrest, camera and cooldown latches; the integrated post-payment failure above remains open. Stock rules are default; FairBusts requires explicit enablement. |
| Heat froze at the first missed sight ray | Heat continues during tracked-sight grace and pauses in cooldown. |
| Police compared model/display coordinates against cop physics coordinates | Sensing and pursuit targets now use the player's current body position, forward and actual velocity. Model pivots and display interpolation no longer move the arrest/radar height and range inputs. |
| Heat and pursuit counts remained authored template text | Session heat updates both duplicate labels and the reference multiplier object. Current ground units, unique damaged units and wrecked units replace template counters. |
| Ordinary corner impacts punctured tires | Contact damage affects visual zones and event sequences. Only a strip punctures the player tires. |
| Box-sized damage squares/crack lines floated around the car | Procedural bounding-box marks are removed. Installed corner overlays, window damage textures and detached parts remain. |
| Roadblock cars lost their layout offsets when snapped to a lane | A road navigator follows the road ahead. Its tangent and uninset edges select and orient a layout; each car retains its lateral/longitudinal placement. Roadblock cars have police damage/collision identity and emergency lights. |
| Spike strips were invisible, sampled only at the current tire patch, and ignored elevation | Visible procedural strip geometry shares the contact footprint. Consecutive grounded patches sweep across it, including elevation; another road deck and a reset/teleport cannot puncture through it. |
| EA Trax controls could start songs under pursuit music | All pause/resume/next/previous/direct-play paths respect pursuit soundtrack ownership. Paused-song state and history survive; the EA Trax HUD card is suppressed while its song is held. |
| Chase radio cues used names absent from the installed table | Requests now use the documented installed `SpeechID` values. Every chase cue is checked against the installed table and dispatcher. This fixes request routing; composite speech playback remains incomplete. |
| Pursuit score entries all pointed into section 1 and could play a single ending bar | Score entries are discovered from the four actual pursuit sections. Installed regressions require each to sustain 300 bars at low, middle and high control, with decoding checks. Missing sections fail clearly. |
| Another pursuit could interrupt an arrest/ticket | Automatic and console pursuit starts wait for the arrest and ticket. Paying resets session heat to 1. |

## What exists and where parity stops

| Area | Implemented | Missing or approximate |
| --- | --- | --- |
| Patrol population | Installed wave models, independent heat-specific patrol target and traffic lane driving; patrols probed on any side of the player in the 150 to 400 m window, rejected inside no-cop-spawn zones; the out-of-play recycle rule (8 s / 10 s settling, beyond 275 m or outside the forward cone, 3 s for wrecks, 375 m protection for a chase); the 60 s session lockout and 120 s lockout after an evade or a paid ticket | Inactive cop pool (cars are created and dropped, not pooled), 911 dispatch, the over-budget priority removal and the exact camera base (the chase camera eye stands in). The 40 m spacing is applied; the original's 15 m/5 m same-lane rules are not. `cop_share` is a rewrite enable switch. Traffic must be enabled. The session lockout means no patrols for the first 60 s of free roam (decomp, not seen live). |
| Heat | Floating-point growth outside cooldown, heat-selected waves/formations/support, 0.95 escape retention by default, paid-ticket reset; the career bin caps the chase heat (bins 15/14: 2 ... bins 4-1: 5) and scales the level 1 to 5 climb by `ScaleEscalationPerBucket`; a Bartender `TimePerLevel` still wins | Paint cooling (`fecooling`, not read anywhere). Per-car heat persistence exists in `nfsmw-career` (record heat, unload cap 5, spawn floor 1) and is fed to the chase; the rewrite has no multi-car garage flow to exercise it. Free roam without a career permits heat up to 10, which exceeds normal career access. |
| Detection | Speeding threshold 65 mph; recent hit; heat 4 recognition; patrol 150/50 m front/rear limits, pursuit-row ranges, world obstruction and per-cop hiding latch | Complete offence detection, 911 reports, description matching and individual unit knowledge. Hit/throttle and road-edge rules contain documented approximations. |
| Driving AI | Road paths, adaptive PID, player-performance matching, avoidance, reverse recovery, formations, collapse, pit/box-in/rolling-block aims and support vehicles | Original driving/action arbitration, pursuit competitiveness and recovery tuning have not been validated on matched routes. Several tactic gates remain inferred. |
| Search | Tracked grace, traffic-lane searches, replacement cap, physical-unit reacquisition, and surplus cops (over the cool-down's `NumPatrolCars`) sent away two at a time to drive off with lights out until recycled | Original search coverage, sector dispatch, individual observations and matched-route recovery tuning. Per-cop sight for the flee rule is approximated by the chase's contact plus the front range. |
| Escape and bust | Signed pursuit meter, cooldown/hiding, near/slow bust, control lock, stopped cops, fallback ticket (payable from the real ticket screen) and payment cleanup | Authored arrest NIS, impound/strikes, career money and saves. The fallback is a three-second hold then ticket. On an evade the original locks the cops out, which removes every active cop on its next pass; the rewrite does the same (decomp), so there is no visible disengagement to add for an evade. Other ends (the console `pursuit off`) also remove ground units at once. |
| Backup | Heat-selected wave types, inter-spawn delays, a finite cops-to-evade budget (`FullEngagementCopCount`, engaged cops that leave count as evaded, never fewer than one to evade, never fewer than the cops engaged), the backup called at `NumCopsToTriggerBackup` and arriving after `BackupCallTimer` while the player is seen, `backup_eta` in the audio feed and the status line | A backup ETA element in the pursuit board (the stock board has none; only the speech reads it), dispatch callouts, and roadblock cops recruited into the chase. Replacements during visible contact remain an approximation. |
| Roadblocks/support | Installed layout tables, on-road placement, spike layouts, SUV/leader support requests and timers | Sawhorse/barrier props, site visibility/occupancy checks, full dispatch scheduling and repeated roadblocks. The current director permits one successful heavy roadblock per chase. Partial layouts can remain when some cars cannot stand. |
| Spikes | Visible footprint, swept tire/deck test, puncture/blowout handling and repair | Installed prop model, exact prop collision shape, strip animation, AI tire punctures and exact original swept-box equivalence. Tooth mesh and box dimensions are procedural. |
| Radar | Authored layout/opacity/state transitions, arrow/LEDs/laser; native 300 m cone selection, range normalization, 0.1 floor and per-update ramp | Speed cameras, sweep click, original full-body roll-dependent bearing basis and live rendered calibration against the original. Radar remains available without the rear-view mirror. |
| Pursuit HUD | Heat multiplier, timer, signed five-segment pursuit meter/cooldown, police/damaged/wrecked counts | Heat arc masks, hiding indicator, backup ETA, bounty, cost to state and complete infraction announcements. The engaged count currently counts live pursuit/roadblock units rather than the stock engagement-radius rule. |
| Music | Installed interactive pursuit sets, intensity steering, crossfades and return to licensed songs; guarded EA Trax transport | Matched original transitions/mixing, ending cues and all original scoring contexts. Fade timings include existing guesses. No listening-parity claim. |
| Police radio | Installed speech dispatcher requests pursuit opening, lost/regained contact, heat jump, roadblock, collision and disabled-unit events by ID; direct recording banks can play | Composite sentence assembly is missing, so a queued event need not be audible. Also missing: complete car descriptions, 911 conversation, tactics/ram outcomes, accurate per-speaker placement, terminal outcome lines and dispatch sequencing. |
| Sirens | Police light bars and helicopter rotor are wired | Ground-vehicle siren AEMS playback/state machine is not wired. Lights do not imply audible sirens. |
| Damage | Installed visual zones, overlays, window swaps, sequenced detachable parts; spike-only handling impairment | Original paint scuffs/deformation and a matched damage-effects comparison. Normal body damage has no health bar or generic handling penalty. Damage/heat remain session-local. |

## Enhancements and development controls

Traffic lights, optional traffic avoidance, police health bars, manual EA Trax
pause/previous/custom playlists and configurable pursuit mods are rewrite
features. Their defaults are not evidence of original-game parity. In particular,
the original has no functioning traffic lights; `set traffic_lights off` disables
that extension.

Use the console (`F12`) to inspect `traffic status`, `pursuit status` and
`music status`. `pursuit 1` through `pursuit 10` start forced development chases;
`pursuit off` clears their units and strips. `roadblock` tests layout placement
once the car is on the road. `repair` resets visual damage and punctured tires.
Forced commands do not replace testing a naturally witnessed offence.

## Verification and acceptance

Synthetic regressions exercise promotion with identity/velocity preservation,
retained-heat recognition, live/wreck/occlusion sight rules, tracked grace/search,
reacquisition, helicopter contact, arrest/payment/control gates, spike sweep/deck
geometry, roadblock spacing, collision damage without punctures, HUD duplicate
labels/counters and soundtrack transport ownership. Installed regressions load
the real patrol geometry/physics/heat rows and real HUD objects; the existing
interactive-music regression enters all four installed pursuit sets and decodes
their streams.

The installed moving-route fixture drives a physical police car against a
kinematic target on installed lanes: driving, stopping, separation into search
and reunion. Its isolated single-unit wave does not validate stock population,
player dynamics, full-route robustness or a matched original comparison.

The installed arrest fixture uses the real player/cop physics and world clock,
holds throttle/steering/NOS/reset during arrest, consumes the fallback ticket,
pays it and verifies that the player can move again.
`frontend::bust_resume_tests` goes further: it wraps the world in the paused
scene, polls the ticket as the front end does, presses accept on the real
`Infractions.fng`, and checks the world, HUD, chase and car afterwards. It was
red before the ticket fix. It is a headless test, not a play session.

Relevant modules/tests:

- `scenes::world::ai::pursuit::tests::free_roam`
- `scenes::world::ai::police::tests::free_roam`
- `scenes::world::events::tests::severe_corner_impacts_damage_the_body_without_puncturing_tires`
- `audio::radio::tests::pursuit`, `hud::pursuit_tests`
- `installed_free_roam_pursuit_uses_the_real_patrol_car_and_heat_rows`
- `installed_pursuit_hud_drives_heat_counts_and_radar_without_template_numbers`
- `installed_police_drive_stop_search_and_reacquire_on_real_roads`
- `installed_arrest_locks_player_input_until_the_ticket_is_paid`
- `installed_bust_ticket_and_payment_return_to_driving_through_the_front_end`
- `scenes::world::ai::pursuit::tests::{lockout, placement, recycling, budget}`
- `every_pursuit_set_can_be_entered_and_its_first_bar_decodes`

For original-game acceptance, compare the same car, route, heat, traffic and
timing across: unprovoked roaming, a witnessed offence, a blocked/overpass
sighting, loss and reacquisition, a complete hidden escape, a slow boxed-in
bust, a roadblock/spike pass, and pursuit music/radio entry and exit. Long-route
driving and listening comparisons remain separate from source/data tests.

Spec and source record: [free-roam integration](specs/pursuit-free-roam.md),
[provenance](provenance/pursuit-free-roam.md). Related component specs remain
linked from [AI drivers](ai-drivers.md).
