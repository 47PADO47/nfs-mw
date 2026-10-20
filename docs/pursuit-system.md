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
| Police radio | Reported at the time: no audible chase radio in the playtest. Since then the original's speech machine is ported ([ai-pursuit-speech](specs/ai-pursuit-speech.md)): see "Police radio" in the table of implemented systems below. Nothing has compared the result with the original by ear, and the cue lookup/queue tests do not prove audible speech. |
| Police driving | Poor pursuit decisions and incorrect police vehicle physics/damage remain. The short single-unit fixture does not establish competitive AI or collision/damage parity. |
| Pursuit breakers | No working pursuit breakers were available in the playable test. Existing event-system code is not evidence that the complete pursuit interaction works. **Addressed in code, see "Pursuit breakers and hiding spots" below:** the structures were placed and collapsed, but the cops in their zone were only braked, not destroyed as the original does, and nothing told the player where a breaker is or that one fired. The cinematic moment is not played. |
| Hiding spots | No working hiding spots were available in the playable test. The existing hiding/cooldown state does not establish authored spot discovery, presentation or gameplay integration. **Addressed in code, see below:** the zones were found and fed the evade, but without the original latch (a spot entered in sight never hides) and with no HUD mark or listing. |
| Sirens | Ground police siren playback was not implemented in the tested build. It is now wired ([cop-siren-audio](specs/cop-siren-audio.md)) but has not been playtested or compared by ear. |
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
| Spike strips were invisible, sampled only at the current tire patch, and ignored elevation | The installed strip model and collision box share the contact footprint (procedural geometry only while the model is not loaded). Consecutive grounded patches sweep across it, including elevation; another road deck and a reset/teleport cannot puncture through it. |
| EA Trax controls could start songs under pursuit music | All pause/resume/next/previous/direct-play paths respect pursuit soundtrack ownership. Paused-song state and history survive; the EA Trax HUD card is suppressed while its song is held. |
| Chase radio cues used names absent from the installed table | Requests now use the documented installed `SpeechID` values. Every chase cue is checked against the installed table and dispatcher. This fixes request routing; sentence assembly by parameters is implemented (`Audio::say_with`), the flows that supply the parameters are not. |
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
| Roadblocks/support | Installed layout tables; the `RequestRoadBlock` lottery (8 to 12 s timer, armed flag, per-heat odds from the install) with the manager's latch that holds back new pursuit cops; heavy-support SUV roadblocks; whole-or-nothing layouts on a plain, long cop-road segment with occupancy and on-road checks; installed sawhorse bodies and spike-strip models; the 60 m "roadblock sees you" rule, the dodge, the 20 s cheat rule that recruits one car, follow formation while whole, release when no car is left and repeats; ram and leader requests | Live-play parity (nothing was run in a game here). Stealing out-of-view cops for the car count, sawhorses breaking into fragments, the cops-needed-to-evade count, roadblock statistics reaching the rap sheet. Roadblock cars are retired by the rule for unseen traffic, not the original's cop rule. |
| Spikes | Installed strip model and collision box (6.5 m wide), the original's tire box (0.15 m along the axle) swept along the step, grounded and deck test, puncture/blowout handling and repair, strips tied to their roadblock's life | Exact original box routine (not in the sources), strip trigger/simulated hand-over and animation. The original punctures only racer-class cars: cops and traffic are not spiked and there are no AI racers in free roam. The helicopter never drops spikes (statistic only). |
| Helicopter | Spawn by the wave's turn (timer, minimum support delay, `copheli` weight, search-mode roll), road spawn point 250 m behind and 20 m up, fuel, search around the last known place with the heat row's radius, pursuit-level sight switch, hidden zones and tunnels, height sheet, skid hit, exit, minimap blip, status for the speech (sight, fuel, mode, health), rotor fed with the body's real heading and velocity, breaker destruction | The sight-cone minimap object and rotor-wash particle, `AvoidCamera`, mod options that take several values, and a comparison in a running game. There is no spotlight, gun or spike drop in the original. |
| Radar | Authored layout/opacity/state transitions, arrow/LEDs/laser; native 300 m cone selection, range normalization, 0.1 floor and per-update ramp | Speed cameras, sweep click, original full-body roll-dependent bearing basis and live rendered calibration against the original. Radar remains available without the rear-view mirror. |
| Pursuit HUD | Heat multiplier, timer, signed five-segment pursuit meter/cooldown, police/damaged/wrecked counts, hiding mark (authored fade scripts) | Heat arc masks, backup ETA, bounty, cost to state and complete infraction announcements. The engaged count currently counts live pursuit/roadblock units rather than the stock engagement-radius rule. |
| Music | Installed interactive pursuit sets, intensity steering, crossfades and return to licensed songs; guarded EA Trax transport | Matched original transitions/mixing, ending cues and all original scoring contexts. Fade timings include existing guesses. No listening-parity claim. |
| Police radio | The original's speech machine runs against the chase ([spec](specs/ai-pursuit-speech.md), code `libs/blackbox-pursuit/src/speech` and `audio/chase/radio`): the opening exchange with its parameters (the unit's battalion and call sign, the offence from the last infraction, the car's type, stock paint colour and speed in steps of 20, the heading held for 3 s and the road's name, one or several suspects), a spotter opening when the player blew past a cop, a restart as "unit rammed" after hitting a cop; the strategy commentary (formation announced, a lone cop's own plan, calls to position, the outcome, "suspect outrun", backup requests and dispatch's answers with the arrival time band); lost contact (lost visual, lost suspect, dispatch's break away, the quadrant lines, regained contact as re-initiate or spotted, "time expired" once the pursuit is over); heat jumps and jurisdiction shifts; the arrest bullhorn (the helicopter's when close) and the busted level; cops leaving (bail-out wording by health, heavy traffic or damage, a denial from another primary, "unit disabled"); the roadblock call and dispatch's answer, the roadblock warning and outcome; dead air (dispatch asks for an update); the helicopter's own lines. Voices: six for the ordinary cops and one for the cross car, one voice per cop, shuffled, a closer cop taking over a farther cop's voice as the original does; call signs from the battalion's pool. Tested: 72 unit tests of the machine on scripted chases, 30 of the adapter (tables, parameters, snapshot) and the weather cue, 5 of the world feed; installed tests: all 112 lines have an event in the install, and a scripted chase timeline of 64 requests assembles to audio with the real recordings. Console `speech flow` | Not heard against the original. The world now tells the radio, through an event log ([spec](specs/ai-pursuit-speech-world.md)): crashes into walls, props, traffic and cops (the ram classified as rear-end, T-bone, head-on or side swipe, and whether it was deliberate), a cop hitting traffic or a wall, jumps (told on landing), a passing cop, tunnels, a gas station blast, a roadblock built, dodged, hit and its tires burst, the ram and leader support calls and refusals, the backup's arrival time, rolls, hard braking and U-turns (from the pedals and the road's direction); a cop wrecked by a breaker is called in as disabled. Tested: 21 world tests with synthetic worlds, the notice mapping, and an installed scripted chase whose world events all assemble to audio with the real recordings. Not raised: the free-roam 911 call (no feed outside a chase), the off-road places (the track's "Speech" map is installed, its id-to-place table is not read), AI racers (none in this world); the original never raises the helicopter joining a roadblock, the guardrail, train and spin-out lines. Dispatch's "no" to a heavy or leader support request has no recording (as in the original's data). A cop's sight is the world's sight test applied to its own range (no second ray cast), and "in view" is that within 150 m. The helicopter's quadrant lines, `HoldOn`, `DeadHeliSilence`; a jump is told on landing, the road's direction is the nearest segment's centre line, the gas station blast is taken as the player's. The sirens' speech focus input remains missing. No radio click, low-pass, panning or busted muffling (they need the dynamic mixer's speech object and the static stitch bank) |
| Sirens | Police light bars and helicopter rotor are wired; ground-car sirens play through the original `SIREN` AEMS module: a per-car state machine (yelp/wail/scream/die from `pursuitlevels`), four voices within 135 m, the lagged voice follower, the cop-car mixer state for distance and Doppler, off when the light bar is dark and die when wrecked ([spec](specs/cop-siren-audio.md)). Tested with unit tests and an installed headless render | Not compared by ear with the original. No speech focus (the first 5 s and 5 s without contact stand in), no speech-line input to the pressing branch, and `UpdateSiren` itself was only read from the decompilation, not located in `speed.exe`. Patrol cars sit in the voice slots silently, as inferred for the original. |
| Damage | Installed visual zones, overlays, window swaps, sequenced detachable parts; spike-only handling impairment | Original paint scuffs/deformation and a matched damage-effects comparison. Normal body damage has no health bar or generic handling penalty. Damage/heat remain session-local. |

## Pursuit breakers and hiding spots

Specs: [pursuit breakers](specs/pursuit-breakers.md), [pursuit hiding](specs/pursuit-hiding.md). Evidence tags are in
the specs; nothing below is a live-play or rendered comparison with the original.

| Area | Now | Still missing or unverified |
| --- | --- | --- |
| Breaker data | 16 installed event engines open a cop zone (5 to 24 m, 5 s); the track places 116 of them in 48 sections (9 gas stations, 3 donut/tyre shops, 4 gazebos, 57 scaffold supports...). The census is an installed test | Per-structure trigger rules beyond the gas station and the sequences' own filters (each engine's responses are read from the data at run time) |
| Breaker chain | Streamed city: ramming a gas pump (or the player's Elise driving into one at 14 m/s) detaches it, the pump's sequence collapses the roof and opens its zone; pillars are permanent and do nothing | Other structures are covered by the earlier trace tests, not by streamed-world drives |
| Cop reaction | Every police car inside a zone, including later entrants, is destroyed (as `ApplyBreakerZones`), shown with full body damage and counted once as a wreck (rep, cost to state, destroyed counter). Cars outside are untouched | The helicopter is not destroyed in a zone; destroyed-cop speech; breaker music |
| Cinematic moment | The scene (`WMGasStation`...) is read and named in the breaker history | Playing it: the NIS animation codec is open |
| Diagnostics | Console `pursuit breakers` (resident breakers nearest first, zone, supports, scene, open zones, last events) and `pursuit hiding` (the track's 34 spots nearest first, heights, whether the player is inside, latch state) | |
| Hiding | The 34 installed HIDDEN zones (elevation = physics ground height, verified) hold the player by its feet within 1.25 m; the original latch (0.05 s unseen on entry, never when seen on entry), scopes, per-cop sight latch, evade multiplier for cars or helicopters; HUD mark fades in/out with the package scripts. A parked player in the installed overpass spot escapes a no-cop heat 1 chase in 14.6 s against 25.0 s in the open | Whether the original shows the mark for the quick latch; the mark's rendered look; helicopter latch against live heli sight |

Why the playable test showed no working breaker: the chain from a hit to a zone worked, but its only effect was to
brake cops that happened to be within 5 to 24 m, the pillars of a gas station cannot be knocked over (only the pumps
start its collapse) and nothing located or announced a breaker. Cops are now destroyed, and the console lists them.
Whether a human playing through a route finds the same result is not established here.

## Enhancements and development controls

Traffic lights, optional traffic avoidance, police health bars, manual EA Trax
pause/previous/custom playlists and configurable pursuit mods are rewrite
features. Their defaults are not evidence of original-game parity. In particular,
the original has no functioning traffic lights; `set traffic_lights off` disables
that extension.

Use the console (`F12`) to inspect `traffic status`, `pursuit status` and
`music status`. `pursuit 1` through `pursuit 10` start forced development chases;
`pursuit off` clears their units and strips. `pursuit breakers` and `pursuit hiding` list the resident breakers and
the hiding spots. `roadblock` tests layout placement
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
- `scenes::world::ai::pursuit::tests::roadblock`, `scenes::world::ai::pursuit::tests::air`, `blackbox_pursuit::roadblock::{request, standing}`, `blackbox_pursuit::spikes`
- `installed_roadblock_props_have_bounds_and_models`, `installed_level_rows_match_the_roadblock_and_helicopter_tables` (ignored; need the game)
- `scenes::world::events::tests::severe_corner_impacts_damage_the_body_without_puncturing_tires`
- `audio::radio::tests::pursuit`, `hud::pursuit_tests`
- `installed_free_roam_pursuit_uses_the_real_patrol_car_and_heat_rows`
- `installed_pursuit_hud_drives_heat_counts_and_radar_without_template_numbers`
- `installed_police_drive_stop_search_and_reacquire_on_real_roads`
- `installed_arrest_locks_player_input_until_the_ticket_is_paid`
- `scenes::world::ai::pursuit::tests::installed_breaker`, `installed_hiding`, `props::scenery::tests::census`,
  `hud::police_install_tests::installed_hiding_mark_fades_in_under_the_cooldown_bar_and_out_again`,
  `real_install_hidden_zone_elevations_are_ground_heights`, `installed_breaker_profiles_match_the_documented_zones_and_name_their_moments`
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
