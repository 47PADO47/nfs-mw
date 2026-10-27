//! One shared real-time cadence and per-tick simulation delta for every world body and rule.

use blackbox_gfx::RenderBackend;
use blackbox_pursuit::health_bar::{Frame, Gates};

use glam::Vec3;

use super::players::LEAD;
use super::{View, ViewSource, WorldScene, ai, drive::STEP, space};
use crate::input::ActionState;
use crate::viewer::camera::PursuitFeed;

/// A screenshot update advances the real-time slice of this many default-cadence ticks.
const BATCH_STEPS: u32 = 6;

/// What the pursuit camera reads from the traffic: the busted meter, the bust and the nearest cop, in render space.
/// The police measure in physics space, so the player's position is converted to it for the cop's search.
fn pursuit_feed(traffic: &ai::TrafficWorld, player: Vec3) -> PursuitFeed {
    let at = Vec3::from_array(space::to_physics(player));
    PursuitFeed {
        meter: traffic.busted_meter(),
        busted: traffic.is_busted(),
        cop: traffic.nearest_matched_cop_position(at).map(|cop| space::to_render(cop.to_array())),
        player,
    }
}

impl WorldScene {
    pub(super) fn update_simulation(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) -> f32 {
        let _trace = blackbox_trace::scope("World: simulation");
        let _span = bevy_log::info_span!("world::simulation").entered();
        let trace = blackbox_trace::scope("World: simulation / prepare traffic");
        self.prepare_traffic(renderer);
        drop(trace);
        let arrested = self.traffic.as_ref().is_some_and(|traffic| traffic.arrest_active());
        let mut speedbreaker = self.tuning.speedbreaker;
        if self.extra.testing.unlimited_breaker {
            speedbreaker.max_duration = None;
        }
        if let Some(drive) = self.players.lead_mut() {
            drive.refill_extra(self.extra.testing.unlimited_nos, self.extra.testing.unlimited_breaker);
        }
        self.speedbreaker_clock.set_dilation(speedbreaker.dilation);
        // A session forbids time scaling: the speedbreaker is off for every player and the tick runs at 1.0
        // (`session_rules.rs`, `docs/specs/multiplayer/determinism.md` D2).
        let time_scale = self.rules.time_scale_allowed;
        self.physics_clock.set_rate_hz(match time_scale {
            true => self.tuning.physics_rate_hz,
            false => 60,
        });
        if !time_scale {
            self.speedbreaker_clock.reset();
        }
        for drive in self.players.drives_mut() {
            drive.bar_speedbreaker(!time_scale);
        }
        if let Some(drive) = self.players.lead_mut() {
            drive.hold_for_arrest(arrested);
            drive.set_speedbreaker_tuning(speedbreaker);
        }
        let driving = self.view == View::Chase;
        if let Some(drive) = self.players.lead_mut()
            && driving
        {
            drive.prepare_frame(input, dt);
        }
        self.prepare_guests(dt, driving, speedbreaker);
        let batch = dt == 0.0
            && ((driving && self.players.lead().is_some_and(|d| d.needs_batch()))
                || self.traffic.as_ref().is_some_and(|t| !t.settled()));
        // Only missing local world data pauses simulation, not loading distant visible scenery. Every player's
        // surroundings must be in (`docs/specs/coop-career.md` section 10).
        let loaded = self
            .foci_with_speed()
            .all(|(focus, speed)| self.residency.simulation_ready(focus, 20.0 + speed * dt.max(STEP)));
        let real_dt = match batch {
            true => BATCH_STEPS as f32 * STEP,
            false => dt,
        };
        let real_dt = match real_dt.is_finite() && real_dt > 0.0 {
            true => real_dt,
            false => 0.0,
        };
        self.seed_original_camera();
        let camera_ramp = self.speedbreaker_clock;
        let lead_breaker =
            loaded && self.players.lead_mut().filter(|_| driving).is_some_and(|drive| drive.prepare_time(real_dt));
        let active = lead_breaker | (loaded && driving && self.guests_prepare_time(real_dt));
        let simulation_scale = match time_scale {
            true => self.tuning.game_speed * self.moment.sim_speed(),
            false => 1.0,
        };
        let sim_dt = match loaded {
            true => {
                (self.speedbreaker_clock.advance_f64(active, f64::from(real_dt)) * f64::from(simulation_scale)) as f32
            }
            false => 0.0,
        };
        let deltas: Vec<_> = match loaded {
            false => Vec::new(),
            true => self
                .physics_clock
                .advance_real(real_dt)
                .into_iter()
                .map(|period| self.speedbreaker_clock.advance_tick(active, period) * simulation_scale)
                .filter(|delta| delta.is_finite() && *delta > 0.0)
                .collect(),
        };
        let tick_sim_dt = deltas.iter().sum();
        if loaded && let Some(drive) = self.players.lead_mut().filter(|_| driving) {
            drive.finish_visual_time(input, sim_dt, real_dt);
        }
        if loaded && driving {
            self.guests_finish_visual_time(sim_dt, real_dt);
        }
        if let Some(drive) = self.players.lead_mut()
            && (!driving || drive.waiting_for_road().is_some())
        {
            drive.age_effects(sim_dt);
        }
        self.age_guest_effects(driving, sim_dt);
        let steps = deltas.len() as u32;
        let look = self.players.lead_mut().map_or((0.0, 0.0), |drive| drive.take_look(steps));
        let guest_looks = self.guests_take_look(steps);
        // The police, the sight test, the roadblocks, the cop health bars and a running race tick with the physics, so
        // their state and the random numbers they draw do not depend on how ticks group into frames
        // (`docs/specs/multiplayer/determinism.md`, D2 and D4).
        for tick_delta in deltas {
            let trace = blackbox_trace::scope("World: simulation / physics step");
            self.physics_step(tick_delta, driving, look, &guest_looks);
            drop(trace);
            let trace = blackbox_trace::scope("World: simulation / police step");
            self.police_step_with_delta(renderer, tick_delta);
            drop(trace);
            let _trace = blackbox_trace::scope("World: simulation / race tick");
            self.race_tick_with_delta(tick_delta);
        }
        self.step_moment(sim_dt);
        // A loading pause holds the last displayed phase instead of jumping ahead and rewinding.
        let alpha = match batch || dt == 0.0 {
            true => 1.0,
            false => self.physics_clock.alpha(),
        };
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.set_alpha(alpha);
        }
        self.residency.world_parts().1.set_alpha(alpha);
        if let Some(drive) = self.players.lead_mut() {
            drive.set_alpha(match driving {
                true => alpha,
                false => 1.0,
            });
            drive.trace_frame(dt, steps, tick_sim_dt, loaded, self.residency.complete());
        }
        self.set_guest_alpha(match driving {
            true => alpha,
            false => 1.0,
        });
        if loaded {
            self.update_original_camera(real_dt, sim_dt, camera_ramp, active, simulation_scale);
        }
        let speedbreaker_active = driving && self.players.lead().is_some_and(|drive| drive.speedbreaker_active());
        if speedbreaker_active != self.sound_speedbreaker_active {
            self.common_sounds.push(match speedbreaker_active {
                true => crate::audio::CommonEvent::SpeedbreakerBegin,
                false => crate::audio::CommonEvent::SpeedbreakerEnd,
            });
        }
        self.sound_speedbreaker_active = speedbreaker_active;
        self.common_sounds.extend(self.residency.world_parts().1.take_common_sounds());
        sim_dt
    }

    /// One tick of the police and what reads it, after the physics tick: the sight test, the speeding trigger, the
    /// helicopter's line of sight and the busted timer, the pursuit camera, the roadblocks and the cop health bars.
    /// The speeding trigger reads the sighting the collision step just made, checked against the walls.
    fn police_step_with_delta(&mut self, renderer: &mut dyn RenderBackend, dt: f32) {
        if self.players.is_empty() {
            return;
        }
        if self.extra.testing.police_off {
            return;
        }
        self.coop_chase_tick_with_delta(dt);
        let primary = self.chase_primary();
        let (collision, _) = self.residency.world_parts();
        if let Some(traffic) = self.traffic.as_mut() {
            let cast = super::drive::sight::sight_cast(collision);
            traffic.update_sight(dt, &cast);
        }
        ai::speeding_pursuit(self, renderer);
        ai::scripted_cops(self, renderer);
        // The helicopter's sight asks the world whether the line to the player is clear.
        let (collision, _) = self.residency.world_parts();
        let mut roadblock_view = None;
        if let (Some(drive), Some(traffic)) = (self.players.get_mut(primary), self.traffic.as_mut()) {
            let view = drive.player_view();
            roadblock_view = Some(view);
            // The reset key starts the post-reset blink, which the busted timer reads this tick.
            if drive.take_manual_resets() > 0 {
                traffic.note_manual_reset();
            }
            let cast = super::drive::sight::sight_cast(collision);
            let clear = |from: glam::Vec3, to: glam::Vec3| cast(from, to).is_none();
            traffic.note_player_height(drive.half_height());
            if self.extra.testing.lock_heat {
                traffic.set_extra_heat(Some(self.extra.testing.heat));
            }
            traffic.update_police(dt, &view, &clear);
            // The pursuit camera reads the bust after the police's tick, on the same clock.
            drive.update_pursuit(dt, pursuit_feed(traffic, view.position));
        }
        // The cop manager's roadblock work on the same clock: the request lottery and the latch, the heavy support
        // request, and for each standing roadblock the dodge, the cheat rule and the release.
        if let (Some(view), Some(traffic)) = (roadblock_view, self.traffic.as_mut()) {
            let (collision, props) = self.residency.world_parts();
            traffic.update_roadblocks(dt, view, collision, Some(props), &self.physics.surfaces);
        }
        // The cop health bars step on the same clock as the police, after its tick (`effects/cop_health.rs`). The
        // traffic reads the chase gate itself, so `pursuing` here is not used.
        let camera = self.simulation_lens(self.physics_aspect);
        let frame = Frame {
            camera_pos: camera.position,
            camera_forward: camera.forward,
            gates: Gates {
                pursuing: false,
                shot_running: self.players.get(primary).is_some_and(|drive| drive.pursuit_shot_running()),
                hud_visible: self.hud_shown(),
            },
        };
        self.cop_health.step(self.traffic.as_ref(), frame, dt);
    }

    /// Every moving car integrates before pair contacts, and every pose is captured after those contacts.
    fn physics_step(&mut self, dt: f32, driving: bool, look: (f32, f32), guest_looks: &[(f32, f32)]) {
        let _span = bevy_log::info_span!("world::physics_step").entered();
        if let Some(drive) = self.players.lead_mut() {
            drive.refill_extra(self.extra.testing.unlimited_nos, self.extra.testing.unlimited_breaker);
        }
        self.update_moment_control_with_delta(dt);
        let primary = self.chase_primary();
        let focus = self.traffic_focus();
        // What the gameplay rules see (the crash gate, the cops' spawn window, the helicopter, what is out of view) is
        // the players' view (`session_rules.rs`); the impact effects are drawn from this machine's camera.
        let local = self.simulation_lens(self.physics_aspect);
        let (camera, shown) = match self.rules.views {
            ViewSource::LocalCamera => (local, self.gameplay_lens(self.physics_aspect)),
            ViewSource::PlayerCars => {
                let lens = self.rules_lens(self.physics_aspect);
                (lens, lens)
            }
        };
        let event_view = camera.position;
        let (collision, props) = self.residency.world_parts();
        let surfaces = &self.physics.surfaces;
        props.begin_native(dt);
        self.explosions.request(props.take_blasts().into_iter().map(|point| space::to_render(point.to_array())));
        self.explosions.announce_authored(
            props.take_authored_blasts().into_iter().map(|point| space::to_render(point.to_array())),
        );
        self.explosions.step(dt);
        let trace = blackbox_trace::scope("World: physics / player cars step");
        let player_step = match (driving, self.players.lead_mut()) {
            (true, Some(drive)) => drive.begin_physics_step_with_delta(dt, collision, props, surfaces),
            _ => None,
        };
        let player_advanced = player_step.is_some();
        // The co-op guests tick right after the lead, in id order.
        let guest_steps: Vec<_> = match driving {
            true => self
                .players
                .guests_mut()
                .map(|guest| guest.drive.begin_physics_step_with_delta(dt, collision, props, surfaces))
                .collect(),
            false => Vec::new(),
        };
        drop(trace);
        let mut world_sounds = Vec::new();
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.set_damage_view(glam::Vec3::from_array(space::to_physics(event_view)));
            traffic.note_primary_target(primary.0);
            traffic.begin_step_with_delta(dt, focus, collision, props, surfaces);
        }
        props.resolve_native_pairs();
        // The car the police follow takes the traffic's player contacts (the lead outside co-op); the others collide
        // with the traffic as guests.
        let (lead, mut guests) = self.players.lead_and_guests_mut();
        let lead = lead.filter(|_| player_step.is_some()).and_then(|d| d.sim_mut()).map(|sim| (LEAD, sim));
        let stepped = guests.iter_mut().zip(&guest_steps).filter(|(_, step)| step.is_some());
        let mut sims: Vec<_> =
            lead.into_iter().chain(stepped.filter_map(|(guest, _)| Some((guest.id, guest.drive.sim_mut()?)))).collect();
        let mut lead = sims.iter().position(|(id, _)| *id == primary).map(|at| sims.remove(at).1);
        let lead_id = lead.is_some().then_some(primary);
        let guest_ids: Vec<_> = sims.iter().map(|(id, _)| *id).collect();
        let mut guest_sims: Vec<_> = sims.into_iter().map(|(_, sim)| sim).collect();
        let mut contacts = Vec::new();
        if let Some(traffic) = self.traffic.as_mut() {
            let _span = bevy_log::info_span!("world::traffic_step").entered();
            let trace = blackbox_trace::scope("World: physics / traffic collisions");
            traffic.collide_pairs(lead.as_deref_mut());
            traffic.collide_guests(&mut guest_sims);
            drop(trace);
            let touches = traffic.touches().iter();
            contacts.extend(touches.filter_map(|touch| super::players::touch_contact(touch, lead_id, &guest_ids)));
        }
        super::players::collide_players(lead_id.zip(lead), &mut guest_sims, &guest_ids, &mut contacts);
        self.players.contacts = contacts;
        // Every shared pair observes integrated bodies. Complete all body contacts and drag before any engine or
        // suspension response produces the next tick's forces.
        if let (Some(drive), Some(step)) = (self.players.lead_mut(), player_step.as_ref()) {
            drive.finish_physics_body(step, collision, props, surfaces);
        }
        for (guest, step) in self.players.guests_mut().zip(&guest_steps) {
            if let Some(step) = step {
                guest.drive.finish_physics_body(step, collision, props, surfaces);
            }
        }
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.finish_bodies(collision, props, surfaces);
        }
        props.finish_native_world(&super::drive::WorldGround { collision, surfaces }, dt);
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.resolve_articulations();
        }
        let player_step = match (self.players.lead_mut(), player_step) {
            (Some(drive), Some(step)) => Some(drive.complete_physics_step(step, collision, props, surfaces)),
            _ => None,
        };
        let guest_steps: Vec<_> = self
            .players
            .guests_mut()
            .zip(guest_steps)
            .map(|(guest, step)| step.map(|step| guest.drive.complete_physics_step(step, collision, props, surfaces)))
            .collect();
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.complete_step(collision, props, surfaces);
            traffic.update_events(props, &self.physics, event_view);
            // The followed player can be a co-op guest. Only a car admitted to this tick supplies observations.
            let followed = lead_id.and_then(|id| self.players.get_mut(id)).and_then(|drive| drive.sim_mut());
            traffic.observe_completed_step(followed);
            // Recycle may remove a car or loose trailer; keep its final completed callback first.
            world_sounds.extend(traffic.take_collision_sounds());
            traffic.set_heli_camera(Some((camera.position, camera.forward)));
            traffic.finish_step(focus, &camera, &shown, props, collision, surfaces);
        }
        if let (Some(drive), Some(step)) = (self.players.lead_mut(), player_step) {
            drive.update_events(props, &self.physics, event_view);
            drive.finish_step(collision, surfaces, step);
        }
        let guests = self.players.guests_mut().zip(guest_steps);
        for (guest, step) in guests.filter_map(|(guest, step)| Some((guest, step?))) {
            guest.drive.update_events(props, &self.physics, event_view);
            guest.drive.finish_step(collision, surfaces, step);
        }
        let trace = blackbox_trace::scope("World: physics / props");
        let mut visual_contacts = props.take_impact_effects();
        for drive in self.players.drives_mut() {
            visual_contacts.extend(drive.take_impact_effects());
        }
        if let Some(traffic) = self.traffic.as_mut() {
            visual_contacts.extend(traffic.take_impact_effects());
        }
        self.impact_fx.set_enabled(self.vehicle_effects[0]);
        self.impact_fx.step(self.physics.database(), visual_contacts, local.position, dt);
        world_sounds.extend(props.take_sound_contacts());
        if let Some(drive) = self.players.lead_mut() {
            drive.append_world_sound_contacts(world_sounds, player_advanced, dt);
        }
        let stops = props.update_scenery_with_delta(dt, &self.physics);
        drop(trace);
        let moments = props.take_moments();
        let fired: Vec<Vec3> = stops.iter().map(|stop| stop.position).collect();
        self.object_fx.request(props.take_particle_commands());
        self.object_fx.step(props, dt);
        self.car_damage_fx.step(self.traffic.iter().flat_map(|traffic| traffic.damage_effect_sources()), dt);
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.breaker_stops(stops);
        }
        if let Some(drive) = self.players.lead_mut().filter(|_| driving) {
            drive.follow(collision, look, dt);
        }
        for (guest, &look) in self.players.guests_mut().zip(guest_looks).filter(|_| driving) {
            guest.drive.follow(collision, look, dt);
        }
        let helicopter = self.traffic.as_ref().and_then(|t| t.helicopter_motion());
        let helicopter = helicopter.map(|(at, _, _)| Vec3::from_array(space::to_physics(at)));
        self.heli_wash.step(dt, helicopter, self.residency.collision());
        self.finish_flares.step(dt);
        self.ambient_fx.step(dt);
        self.record_original_camera_with_delta(dt);
        let _trace = blackbox_trace::scope("World: physics / prop lifetimes, moments and break icons");
        let view = self.rules_lens(self.physics_aspect).view_proj();
        let others = self.guest_view_projs();
        let (collision, props) = self.residency.world_parts();
        let ground = super::drive::WorldGround { collision, surfaces: &self.physics.surfaces };
        props.update_lifetimes_seen_by(&ground, view, &others, dt);
        self.play_moments(moments);
        let (_, props) = self.residency.world_parts();
        let restored = props.take_restored_sections();
        self.breaker_icons.step(dt, props, &self.physics, &fired, &restored);
    }
}
