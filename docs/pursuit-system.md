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
| Arrest/payment | **Cause found and fixed, not replayed live.** The ticket screen took the two pad-direction messages its package forwards for the payment presses, so accept did nothing and the world stayed frozen under the ticket (the package posts nothing on a payment press). The payment now follows the pressed button. An installed test drives the arrest hold, the ticket, accept on the real `Infractions.fng`, the resume and driving off. In a career the ticket now continues as the original does: an impound notice when the payment impounded the car, the career car select, the safe house menu and the respawn marker ([busted-aftermath](specs/busted-aftermath.md); installed tests `installed_career_bust_*`). Free roam without a career still resumes driving. Not replayed live. |
| Police radio | Reported at the time: no audible chase radio in the playtest. Since then the original's speech machine is ported ([ai-pursuit-speech](specs/ai-pursuit-speech.md)): see "Police radio" in the table of implemented systems below. Nothing has compared the result with the original by ear, and the cue lookup/queue tests do not prove audible speech. |
| Police driving | Poor pursuit decisions and incorrect police vehicle physics/damage remain. The short single-unit fixture does not establish competitive AI or collision/damage parity. |
| Pursuit breakers | No working pursuit breakers were available in the playable test. Existing event-system code is not evidence that the complete pursuit interaction works. **Addressed in code, see "Pursuit breakers and hiding spots" below:** the structures were placed and collapsed, but the cops in their zone were only braked, not destroyed as the original does, and nothing told the player where a breaker is or that one fired. The cinematic moment is now played as the original does (its ICE camera cut, see below); not seen in a live game. |
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
| Patrol population | Installed wave models, independent heat-specific patrol target and traffic lane driving; patrols probed on any side of the player in the 150 to 400 m window, rejected inside no-cop-spawn zones; the out-of-play recycle rule (8 s / 10 s settling, beyond 275 m or outside the forward cone, 3 s for wrecks, 375 m protection for a chase); the 60 s session lockout and 120 s lockout after an evade or a paid ticket | The active-car budget (8, the Bartender `MaxCops`) gates the wave; queued spawn requests (console `pursuit cop <car>`) take the lowest-priority cop off the road when they would break it and hold the patrol pass; the spawn window is measured from the camera eye (the simulation lens, also the recycle base); patrols and the search cop keep the original's 15 m / 5 m lane spacing, chase spawns 40 m; the wave's spot comes from the seek-ahead walk with the 200 m disc fallback ([provenance](provenance/ai-pursuit-cop-fleet.md)). `cop_share` is a rewrite enable switch. Traffic must be enabled. The session lockout means no patrols for the first 60 s of free roam (decomp, not seen live). |
| 911 call (free roam) | With no chase, the damage cost of the props the player's car hits (the install's `COST_TO_STATE`, once per touch) and the hard traffic hits (intensity from the closing speed over `PlayerSmashSpeedRange`) are counted and told to the speech machine, which says the dispatcher's report when `CTSFor911` or `NumCiviHitsFor911` of the session heat's row is reached; when the line has been said the player's 911 timer runs `Lifetime911` seconds (45 or 180), the cop lockout clears and a patrol that sees the player starts a chase at any speed (`has_911`). The counters clear while a chase or the arrest runs; an evade or paid ticket restarts the speech and allows another call. Rewrite fallback: the world makes the call after 20 s when the speech never answers. Console `pursuit status`, `pursuit 911`. Unit tests (counters, fallback, lockout, sighting, feed, machine) and installed tests (the heat rows equal the spec table, 166 of 181 smackables carry a cost) ([provenance](provenance/ai-pursuit-911.md)) | Not played against the original: the original's per-callback counting of a continuous contact, the thresholds as an experience, the report heard by ear; the description-matching cause beyond the speech machine's `911Reported` wording; the scripted 911 during the last 20 s of a lockout outside free roam. |
| Heat | Floating-point growth outside cooldown, heat-selected waves/formations/support, 0.95 escape retention by default, paid-ticket reset; the career bin caps the chase heat (bins 15/14: 2 ... bins 4-1: 5) and scales the level 1 to 5 climb by `ScaleEscalationPerBucket`; a Bartender `TimePerLevel` still wins | Paint cooling: the `fecooling` table is read and a part fitted in a career garage multiplies the car's record heat (it is not a chase-time effect; [career-cooling](specs/career-cooling.md)). Per-car heat persistence exists in `nfsmw-career` (record heat, unload cap 5, spawn floor 1) and is fed to the chase; the career car select shows each car's heat meter. The garage has no decals, vinyls, numbers, rim paint, window tint or backroom shop, no price, and no heat preview. Free roam without a career permits heat up to 10, which exceeds normal career access. |
| Detection | Speeding threshold 65 mph; recent hit; heat 4 recognition; patrol 150/50 m front/rear limits, pursuit-row ranges, world obstruction and per-cop hiding latch | Complete offence detection, description matching and individual unit knowledge. Hit/throttle and road-edge rules contain documented approximations. |
| Driving AI | Road paths, adaptive PID, player-performance matching, avoidance, reverse recovery, formations, collapse, pit/box-in/rolling-block aims and support vehicles | Original driving/action arbitration, pursuit competitiveness and recovery tuning have not been validated on matched routes. Several tactic gates remain inferred. |
| Search | Tracked grace, traffic-lane searches, replacement cap, physical-unit reacquisition, and surplus cops (over the cool-down's `NumPatrolCars`) sent away two at a time to drive off with lights out until recycled | Original search coverage, sector dispatch, individual observations and matched-route recovery tuning. The flee rule reads each cop's own last sight test. |
| Escape and bust | Signed pursuit meter, cooldown/hiding, near/slow bust, control lock, stopped cops, fallback ticket (payable from the real ticket screen) and payment cleanup | Authored arrest NIS. Career impound strikes, money, the car select and the release are implemented ([busted-aftermath](specs/busted-aftermath.md)); the world is not rebuilt on resume (the car is placed on the road nearest the respawn marker) and the car model is not the career car. The fallback is a three-second hold then ticket. On an evade the original locks the cops out, which removes every active cop on its next pass; the rewrite does the same (decomp), so there is no visible disengagement to add for an evade. Other ends (the console `pursuit off`) also remove ground units at once. |
| Backup | Heat-selected wave types, inter-spawn delays, a finite cops-to-evade budget (`FullEngagementCopCount`, engaged cops that leave count as evaded, never fewer than one to evade, never fewer than the cops engaged), the backup called at `NumCopsToTriggerBackup` and arriving after `BackupCallTimer` while the player is seen, `backup_eta` in the audio feed and the status line | A backup ETA element in the pursuit board (the stock board has none; only the speech reads it), and dispatch callouts. A roadblock car joins the chase through the cheat rule (one car); the spike-hit recruit (`SpikesHit`) has no caller in the original and is not built. Replacements during visible contact remain an approximation. |
| Roadblocks/support | Installed layout tables; the `RequestRoadBlock` lottery (8 to 12 s timer, armed flag, per-heat odds from the install) with the manager's latch that holds back new pursuit cops; heavy-support SUV roadblocks; whole-or-nothing layouts on a plain, long cop-road segment with occupancy and on-road checks; installed sawhorse bodies and spike-strip models; the 60 m "roadblock sees you" rule, the dodge, the 20 s cheat rule that recruits one car, follow formation while whole, release when no car is left and repeats; ram and leader requests; a lottery roadblock takes the roaming cops that were out of view for 5 s and are over 100 m away for its cars (farthest first) before it creates any; sawhorses break into their four installed pieces through the `genericsmackable` sequence when a car reaches them; roadblock cars are retired by the cop manager's out-of-play rule (they were already routed there) | Live-play parity (nothing was run in a game here). The cop pool: the original asks its pool of inactive cars first and steals only for the shortfall, the rewrite builds no pool of cars (it shares each model's rig, so one would change nothing; see the cop fleet provenance) so it steals first and creates the rest; the cops-needed-to-evade count, Roadblocks deployed/dodged, spike strips dodged, cops involved/damaged and helicopters reach the rap sheet history (`scenes/world/career/rap_tally.rs`); props deployed, traffic cars hit, patrol/support counts and property damage do not. Pieces are drawn only where the shared scenery holds `SAWHORSE_FRAG` meshes (checked by the installed test on the physics side, not seen on screen). |
| Spikes | Installed strip model and collision box (6.5 m wide), the original's tire box (0.15 m along the axle) swept along the step, grounded and deck test, puncture/blowout handling and repair, strips tied to their roadblock's life | Exact original box routine (not in the sources), strip trigger/simulated hand-over and animation. The original punctures only racer-class cars: cops and traffic are not spiked and there are no AI racers in free roam. The helicopter never drops spikes (statistic only). |
| Helicopter | Spawn by the wave's turn (timer, minimum support delay, `copheli` weight, search-mode roll), road spawn point 250 m behind and 20 m up, fuel, search around the last known place with the heat row's radius, pursuit-level sight switch, hidden zones and tunnels, height sheet, skid hit, exit, minimap blip, status for the speech (sight, fuel, mode, health), rotor fed with the body's real heading and velocity, breaker destruction | Rotor wash (the install's `heliwash` group, three emitters, on the ground under the helicopter, intensity `1 - ramp(altitude, 5, 30)` refreshed once a second), `AvoidCamera` (a 7 m sphere 4 m in front of the game camera; the cut's camera counts), and the mod options that take several numbers (`Leading`, `ClosingSpeed`, `Acceleration`, `Brake`, `Turning`) | The sight-cone minimap object (`HELICOPTER_LINE_OF_SIGHT`: a 64 x 64 tinted image sharing the icon group's place; its scale and heading rule are in code that could not be re-read, so nothing is drawn), `SearchRadius` (two numbers read and held, no constant of the controller matches), and a comparison in a running game. There is no spotlight, gun or spike drop in the original. |
| Radar | Authored layout/opacity/state transitions, arrow/LEDs/laser; native 300 m cone selection, range normalization, 0.1 floor and per-update ramp | Speed cameras, sweep click, original full-body roll-dependent bearing basis and live rendered calibration against the original. Radar remains available without the rear-view mirror. |
| Pursuit HUD | Heat multiplier, timer, signed five-segment pursuit meter/cooldown, police/damaged/wrecked counts, hiding mark (authored fade scripts) | Heat arc masks, backup ETA, bounty, cost to state and complete infraction announcements. The engaged count currently counts live pursuit/roadblock units rather than the stock engagement-radius rule. |
| Music | Installed interactive pursuit sets, intensity steering, crossfades and return to licensed songs; guarded EA Trax transport | Matched original transitions/mixing, ending cues and all original scoring contexts. Fade timings include existing guesses. No listening-parity claim. |
| Police radio | The original's speech machine runs against the chase ([spec](specs/ai-pursuit-speech.md), code `libs/blackbox-pursuit/src/speech` and `audio/chase/radio`): the opening exchange with its parameters (the unit's battalion and call sign, the offence from the last infraction, the car's type, stock paint colour and speed in steps of 20, the heading held for 3 s and the road's name, one or several suspects), a spotter opening when the player blew past a cop, a restart as "unit rammed" after hitting a cop; the strategy commentary (formation announced, a lone cop's own plan, calls to position, the outcome, "suspect outrun", backup requests and dispatch's answers with the arrival time band); lost contact (lost visual, lost suspect, dispatch's break away, the quadrant lines, regained contact as re-initiate or spotted, "time expired" once the pursuit is over); heat jumps and jurisdiction shifts; the arrest bullhorn (the helicopter's when close) and the busted level; cops leaving (bail-out wording by health, heavy traffic or damage, a denial from another primary, "unit disabled"); the roadblock call and dispatch's answer, the roadblock warning and outcome; dead air (dispatch asks for an update); the helicopter's own lines. Voices: six for the ordinary cops and one for the cross car, one voice per cop, shuffled, a closer cop taking over a farther cop's voice as the original does; call signs from the battalion's pool. Tested: 72 unit tests of the machine on scripted chases, 30 of the adapter (tables, parameters, snapshot) and the weather cue, 5 of the world feed; installed tests: all 112 lines have an event in the install, and a scripted chase timeline of 64 requests assembles to audio with the real recordings. Console `speech flow` | Not heard against the original. The world now tells the radio, through an event log ([spec](specs/ai-pursuit-speech-world.md)): crashes into walls, props, traffic and cops (the ram classified as rear-end, T-bone, head-on or side swipe, and whether it was deliberate), a cop hitting traffic or a wall, jumps (told on landing), a passing cop, tunnels, a gas station blast, a roadblock built, dodged, hit and its tires burst, the ram and leader support calls and refusals, the backup's arrival time, rolls, hard braking and U-turns (from the pedals and the road's direction); a cop wrecked by a breaker is called in as disabled. Tested: 21 world tests with synthetic worlds, the notice mapping, and an installed scripted chase whose world events all assemble to audio with the real recordings. The free-roam 911 call is supplied from the props the player smashes and the hard traffic hits (see the 911 row). Not raised: the off-road places (the track's "Speech" map is installed, its id-to-place table is not read), AI racers (none in this world); the original never raises the helicopter joining a roadblock, the guardrail, train and spin-out lines. Dispatch's "no" to a heavy or leader support request has no recording (as in the original's data). A cop's sight is the world's sight test applied to its own range (no second ray cast), and "in view" is that within 150 m. The helicopter's quadrant lines, `HoldOn`, `DeadHeliSilence`; a jump is told on landing, the road's direction is the nearest segment's centre line, the gas station blast is taken as the player's. The sirens' speech focus input remains missing. No radio click, low-pass, panning or busted muffling (they need the dynamic mixer's speech object and the static stitch bank) |
| Sirens | Police light bars and helicopter rotor are wired; ground-car sirens play through the original `SIREN` AEMS module: a per-car state machine (yelp/wail/scream/die from `pursuitlevels`), four voices within 135 m, the lagged voice follower, the cop-car mixer state for distance and Doppler, off when the light bar is dark and die when wrecked ([spec](specs/cop-siren-audio.md)). Tested with unit tests and an installed headless render | Not compared by ear with the original. No speech focus (the first 5 s and 5 s without contact stand in), no speech-line input to the pressing branch, and `UpdateSiren` itself was only read from the decompilation, not located in `speed.exe`. Patrol cars sit in the voice slots silently, as inferred for the original. |
| Damage | Installed visual zones, overlays, window swaps, sequenced detachable parts; spike-only handling impairment | Original paint scuffs/deformation and a matched damage-effects comparison. Normal body damage has no health bar or generic handling penalty. Damage/heat remain session-local. |

## Pursuit breakers and hiding spots

Specs: [pursuit breakers](specs/pursuit-breakers.md), [pursuit hiding](specs/pursuit-hiding.md). Evidence tags are in
the specs; nothing below is a live-play or rendered comparison with the original.

| Area | Now | Still missing or unverified |
| --- | --- | --- |
| Breaker data | 16 installed event engines open a cop zone (5 to 24 m, 5 s); the track places 116 of them in 48 sections (9 gas stations, 3 donut/tyre shops, 4 gazebos, 57 scaffold supports...). The census is an installed test | Per-structure trigger rules beyond the gas station and the sequences' own filters (each engine's responses are read from the data at run time) |
| Breaker chain | Streamed city: ramming a gas pump (or the player's Elise driving into one at 14 m/s) detaches it, the pump's sequence collapses the roof and opens its zone; pillars are permanent and do nothing | Other structures are covered by the earlier trace tests, not by streamed-world drives |
| Cop reaction | Every police car inside a zone, including later entrants, is destroyed (as `ApplyBreakerZones`), shown with full body damage and counted once as a wreck (rep, cost to state, destroyed counter, the career's cops-destroyed statistics). The helicopter above a zone is destroyed too. A wreck is "unit disabled" to the speech machine. Cars outside are untouched | No breaker statistic exists in the rap sheet or milestones. A breaker raises no music, speech or score of its own in the original either (its message has one listener, the cop manager; decomp, inferred) |
| Cinematic moment | The breaker's `Moment` scene plays its authored ICE camera track (3 to 3.3 s) when the player's car touched the structure within 2 s and a chasing cop is within 20 to 80 m; it is stood on the nearest marker of the scene's kind, hides the HUD, holds the busted meter and slows the world by the track's speed (60 % at the water tower). [Spec](specs/pursuit-breaker-moment.md). Installed: all 14 moment scenes have a marker kind, a camera track and markers; the gas station's cut looks at the roof (0.4 to 1.7 m) from 12 to 18 m; a streamed pump hit by the player's Elise plays it | The player's autopilot during the cut (`EBecomeAiCar`/`EBecomePlayerCar`), traffic stop, noise, depth of field and letterbox (all zero or off in the data); the options byte that gates it; the touch rule is an approximation. No NIS animation codec is needed |
| Map icons | `PURSUIT_BREAKER_0..7` icons mark the resident breakers on the minimap during a chase; a fired breaker removes the icons within 20 m (the original's rule) | The original's icon records (source table, kind 12 not identified); the full-screen map has none |
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
