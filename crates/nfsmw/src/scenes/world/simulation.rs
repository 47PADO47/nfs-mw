//! A single 60 Hz timeline for the player, traffic, object contacts and world timers.

use blackbox_gfx::RenderBackend;
use blackbox_pursuit::health_bar::{Frame, Gates};

use glam::Vec3;

use super::players::LEAD;
use super::{View, ViewSource, WorldScene, ai, drive::STEP, space};
use crate::input::ActionState;
use crate::viewer::camera::PursuitFeed;

/// A screenshot update has no elapsed frame time, so it runs this many shared ticks.
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
        self.prepare_traffic(renderer);
        let arrested = self.traffic.as_ref().is_some_and(|traffic| traffic.arrest_active());
        let speedbreaker = self.tuning.speedbreaker;
        self.speedbreaker_clock.set_dilation(speedbreaker.dilation);
        // A session forbids time scaling: the speedbreaker is off for every player and the tick runs at 1.0
        // (`session_rules.rs`, `docs/specs/multiplayer/determinism.md` D2).
        let time_scale = self.rules.time_scale_allowed;
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
        self.seed_original_camera();
        let camera_ramp = self.speedbreaker_clock;
        let lead_breaker =
            loaded && self.players.lead_mut().filter(|_| driving).is_some_and(|drive| drive.prepare_time());
        let active = lead_breaker | (loaded && driving && self.guests_prepare_time());
        let simulation_scale = match time_scale {
            true => self.tuning.game_speed * self.moment.sim_speed(),
            false => 1.0,
        };
        let sim_dt = match loaded {
            false => 0.0,
            true => self.speedbreaker_clock.advance(active, real_dt) * simulation_scale,
        };
        if loaded && let Some(drive) = self.players.lead_mut().filter(|_| driving) {
            drive.finish_time(input, sim_dt, real_dt);
        }
        if loaded && driving {
            self.guests_finish_time(sim_dt, real_dt);
        }
        if let Some(drive) = self.players.lead_mut()
            && (!driving || drive.waiting_for_road().is_some())
        {
            drive.age_effects(sim_dt);
        }
        self.age_guest_effects(driving, sim_dt);
        let steps = self.physics_clock.advance(sim_dt);
        let look = self.players.lead_mut().map_or((0.0, 0.0), |drive| drive.take_look(steps));
        let guest_looks = self.guests_take_look(steps);
        // The police, the sight test, the roadblocks, the cop health bars and a running race tick with the physics, so
        // their state and the random numbers they draw do not depend on how ticks group into frames
        // (`docs/specs/multiplayer/determinism.md`, D2 and D4).
        for _ in 0..steps {
            self.physics_step(driving, look, &guest_looks);
            self.police_step(renderer);
            self.race_tick();
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
            drive.trace_frame(dt, steps, loaded, self.residency.complete());
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
    fn police_step(&mut self, renderer: &mut dyn RenderBackend) {
        if self.players.is_empty() {
            return;
        }
        self.coop_chase_tick();
        let primary = self.chase_primary();
        let (collision, _) = self.residency.world_parts();
        if let Some(traffic) = self.traffic.as_mut() {
            let cast = super::drive::sight::sight_cast(collision);
            traffic.update_sight(STEP, &cast);
        }
        ai::speeding_pursuit(self, renderer);
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
            traffic.update_police(STEP, &view, &clear);
            // The pursuit camera reads the bust after the police's tick, on the same clock.
            drive.update_pursuit(STEP, pursuit_feed(traffic, view.position));
        }
        // The cop manager's roadblock work on the same clock: the request lottery and the latch, the heavy support
        // request, and for each standing roadblock the dodge, the cheat rule and the release.
        if let (Some(view), Some(traffic)) = (roadblock_view, self.traffic.as_mut()) {
            let (collision, props) = self.residency.world_parts();
            traffic.update_roadblocks(STEP, view, collision, Some(props), &self.physics.surfaces);
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
        self.cop_health.step(self.traffic.as_ref(), frame, STEP);
    }

    /// Every moving car integrates before pair contacts, and every pose is captured after those contacts.
    fn physics_step(&mut self, driving: bool, look: (f32, f32), guest_looks: &[(f32, f32)]) {
        let _span = bevy_log::info_span!("world::physics_step").entered();
        self.update_moment_control();
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
        props.begin_native(STEP);
        self.explosions.request(props.take_blasts().into_iter().map(|point| space::to_render(point.to_array())));
        self.explosions.announce_authored(
            props.take_authored_blasts().into_iter().map(|point| space::to_render(point.to_array())),
        );
        self.explosions.step(STEP);
        let player_step = match (driving, self.players.lead_mut()) {
            (true, Some(drive)) => drive.step(collision, props, surfaces),
            _ => None,
        };
        let player_advanced = player_step.is_some();
        // The co-op guests tick right after the lead, in id order.
        let guest_steps: Vec<_> = match driving {
            true => self.players.guests_mut().map(|guest| guest.drive.step(collision, props, surfaces)).collect(),
            false => Vec::new(),
        };
        let mut world_sounds = Vec::new();
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
            traffic.set_damage_view(glam::Vec3::from_array(space::to_physics(event_view)));
            traffic.note_primary_target(primary.0);
            traffic.step(focus, collision, props, surfaces);
            traffic.collide(lead.as_deref_mut());
            traffic.collide_guests(&mut guest_sims);
            let touches = traffic.touches().iter();
            contacts.extend(touches.filter_map(|touch| super::players::touch_contact(touch, lead_id, &guest_ids)));
            traffic.update_events(props, &self.physics, event_view);
            // Recycle may remove a car or loose trailer; keep its final completed callback first.
            world_sounds.extend(traffic.take_collision_sounds());
            traffic.set_heli_camera(Some((camera.position, camera.forward)));
            traffic.finish_step(focus, &camera, &shown, props, collision, surfaces);
        }
        super::players::collide_players(lead_id.zip(lead), &mut guest_sims, &guest_ids, &mut contacts);
        self.players.contacts = contacts;
        if let (Some(drive), Some(step)) = (self.players.lead_mut(), player_step) {
            drive.update_events(props, &self.physics, event_view);
            drive.finish_step(collision, surfaces, step);
        }
        let guests = self.players.guests_mut().zip(guest_steps);
        for (guest, step) in guests.filter_map(|(guest, step)| Some((guest, step?))) {
            guest.drive.update_events(props, &self.physics, event_view);
            guest.drive.finish_step(collision, surfaces, step);
        }
        props.finish_native_world(&super::drive::WorldGround { collision, surfaces }, STEP);
        let mut visual_contacts = props.take_impact_effects();
        for drive in self.players.drives_mut() {
            visual_contacts.extend(drive.take_impact_effects());
        }
        if let Some(traffic) = self.traffic.as_mut() {
            visual_contacts.extend(traffic.take_impact_effects());
        }
        self.impact_fx.set_enabled(self.vehicle_effects[0]);
        self.impact_fx.step(self.physics.database(), visual_contacts, local.position, STEP);
        world_sounds.extend(props.take_sound_contacts());
        if let Some(drive) = self.players.lead_mut() {
            drive.append_world_sound_contacts(world_sounds, player_advanced);
        }
        let stops = props.update_scenery(&self.physics);
        let moments = props.take_moments();
        let fired: Vec<Vec3> = stops.iter().map(|stop| stop.position).collect();
        self.object_fx.request(props.take_particle_commands());
        self.object_fx.step(props, STEP);
        self.car_damage_fx.step(self.traffic.iter().flat_map(|traffic| traffic.damage_effect_sources()), STEP);
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.breaker_stops(stops);
        }
        if let Some(drive) = self.players.lead_mut().filter(|_| driving) {
            drive.follow(collision, look, STEP);
        }
        for (guest, &look) in self.players.guests_mut().zip(guest_looks).filter(|_| driving) {
            guest.drive.follow(collision, look, STEP);
        }
        let helicopter = self.traffic.as_ref().and_then(|t| t.helicopter_motion());
        let helicopter = helicopter.map(|(at, _, _)| Vec3::from_array(space::to_physics(at)));
        self.heli_wash.step(STEP, helicopter, self.residency.collision());
        self.finish_flares.step(STEP);
        self.record_original_camera();
        let view = self.rules_lens(self.physics_aspect).view_proj();
        let others = self.guest_view_projs();
        let (collision, props) = self.residency.world_parts();
        let ground = super::drive::WorldGround { collision, surfaces: &self.physics.surfaces };
        props.update_lifetimes_seen_by(&ground, view, &others, STEP);
        self.play_moments(moments);
        let (_, props) = self.residency.world_parts();
        let restored = props.take_restored_sections();
        self.breaker_icons.step(STEP, props, &self.physics, &fired, &restored);
    }
}
