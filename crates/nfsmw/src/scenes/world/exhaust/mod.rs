//! Flames at the tail pipes: the blow-off after a gear change and the backfire at a sputter pop
//! (spec: exhaust-flames.md, sections 2 and 8).
//!
//! The car's `ecar` record names a particle group; its emitters (read from the install) are run at every pipe
//! marker of the car model with `blackbox-particles` and drawn as textured billboards, additive or alpha-blended
//! as the particle textures say. Nothing here is stored in the repository: definitions and textures come from
//! the install, and only while the `exhaust_flames` setting is on.

mod command;
mod enhanced;
mod tint;
use tint::{apply_flame_profile, tint_blue};
mod load;
mod scale;
mod trigger;
mod variant;

use blackbox_gfx::RenderBackend;
use blackbox_modding::content::FlameProfile;
use blackbox_particles::{Emitter, Frame};
use blackbox_render::{BlendMode, EffectLayer, EffectVertex, TextureHandle, TexturedEffect};
use glam::{Mat4, Vec3};

use trigger::{Backfire, ShiftEvent, ShiftTiming, blowoff_allowed, flame_intensity};

/// Most tail pipes that flame, and most live particles of one pipe's emitter (spec 8.4).
pub const MAX_PIPES: usize = 4;
pub const EMITTER_LIMIT: usize = 96;

/// Factor on the alpha of additive particles (the fire). The emitter data keeps their alpha at 40 to 60 of 255,
/// which an additive blend of `src * alpha` turns into a flame too faint to see against a lit road; the blend
/// state of the original is not known (spec section 6), so the rewrite brightens them (spec 8.5).
pub const ADDITIVE_GAIN: f32 = 4.0;

/// What the effects need of the car for one physics step.
pub struct CarState {
    /// The car model's frame in the world.
    pub to_world: Mat4,
    pub velocity: Vec3,
    /// -1 reverse, 0 neutral, 1 and up forward.
    pub gear: i32,
    /// The throttle pedal, 0 to 1.
    pub throttle: f32,
    /// The nitrous burns: the pipes flame blue for as long as it does.
    pub nitrous: bool,
}

/// A particle texture on the GPU and how it blends.
struct SpriteTexture {
    handle: Option<TextureHandle>,
    blend: BlendMode,
}

/// All the pipes' emitters of one group emitter, drawn with one texture.
struct Lane {
    texture: usize,
    /// One emitter per tail pipe, in the pipe order.
    emitters: Vec<Emitter>,
    /// The vertices of the lane's batch; they travel to the layer and come back after the upload.
    vertices: Vec<EffectVertex>,
}

/// The flames of a loaded car.
struct Active {
    enhanced: enhanced::Enhanced,
    /// Each pipe's frame in the car model's space.
    pipes: Vec<Mat4>,
    lanes: Vec<Lane>,
    /// The same emitters again for the nitrous, drawn in blue (see [`tint_blue`]). Their particles keep their
    /// colour to the end of their short life when the nitrous stops.
    blue: Vec<Lane>,
    textures: Vec<SpriteTexture>,
    timing: ShiftTiming,
    engine_upgrades: i32,
    shift: ShiftEvent,
    backfire: Backfire,
    /// How strongly the pipes spawned in the latest step, 0 when quiet.
    intensity: f32,
    /// The nitrous burned in the latest step.
    nitrous: bool,
    /// Live particles after the latest step.
    live: usize,
    order: Vec<OrderedSprite>,
    /// The batches handed to the layer by the latest `geometry`: the lane, the texture and the layer's index.
    handed: Vec<(usize, TextureHandle, usize)>,
}

type OrderedSprite = (f32, [Vec3; 4], [u8; 4], [[f32; 2]; 2]);

enum State {
    /// Nothing is loaded (the setting is off, or the car is new and the next frame loads it).
    Unloaded,
    /// The car has no pipes, no group or no textures.
    Missing,
    Ready(Box<Active>),
}

pub struct ExhaustFlames {
    enhanced: bool,
    enabled: bool,
    state: State,
    /// GPU textures to free at the next chance (a renderer is needed).
    stale: Vec<TextureHandle>,
    /// The installed engine upgrade level, supplied by the garage or a diagnostic override.
    engine_level: i32,
    /// Sputter pops reported since the last physics step.
    pops: u32,
    /// The console asked for a backfire.
    forced: bool,
    /// The tint of the nitrous variant the player chose, if it is a mod's. `None` draws the game's own blue flame.
    profile: Option<FlameProfile>,
    /// The particle effects of the nitrous variant the player chose (`variant.rs`).
    variant: variant::Variant,
}

impl Default for ExhaustFlames {
    /// Off, and nothing allocated.
    fn default() -> Self {
        Self {
            enhanced: false,
            enabled: false,
            state: State::Unloaded,
            stale: Vec::new(),
            engine_level: 0,
            pops: 0,
            forced: false,
            profile: None,
            variant: variant::Variant::default(),
        }
    }
}

/// Lane `index` of the orange lanes followed by the blue ones. It takes the two lists, not the `Active`, so that
/// the other fields stay free to use while the lane is borrowed.
fn lane_of<'a>(orange: &'a mut [Lane], blue: &'a mut [Lane], index: usize) -> &'a mut Lane {
    match index < orange.len() {
        true => &mut orange[index],
        false => &mut blue[index - orange.len()],
    }
}

impl Active {
    fn live(&self) -> usize {
        self.lanes.iter().chain(&self.blue).flat_map(|l| &l.emitters).map(Emitter::len).sum()
    }

    /// The orange lanes, then the blue ones: the order the batches are numbered in.
    fn lane_count(&self) -> usize {
        self.lanes.len() + self.blue.len()
    }

    fn lane(&self, index: usize) -> &Lane {
        match index < self.lanes.len() {
            true => &self.lanes[index],
            false => &self.blue[index - self.lanes.len()],
        }
    }

    /// One physics step: the shift event and the backfire, then every pipe's emitters (spawning while the pipes
    /// flame). A step with nothing flaming and nothing alive does no more than that.
    fn step(&mut self, dt: f32, car: &CarState, pops: u32, forced: bool, engine_level: i32, replaced: (bool, bool)) {
        let shifted = self.shift.note_gear(car.gear);
        self.shift.advance(dt, car.gear, car.velocity.length(), self.timing);
        self.backfire.pops(pops, car.throttle, car.gear);
        if forced {
            self.backfire.start();
        }
        let blowoff = blowoff_allowed(engine_level, self.engine_upgrades);
        self.intensity = flame_intensity(blowoff, &self.shift, &self.backfire);
        self.backfire.advance(dt);
        self.nitrous = car.nitrous;
        let event = match forced || (pops > 0 && car.throttle < trigger::LIFT_OFF_THROTTLE && car.gear >= 1) {
            true => Some(crate::garage::BackfireEvent::Pop),
            false => match shifted && blowoff && self.shift.active() {
                true => Some(crate::garage::BackfireEvent::Shift),
                false => None,
            },
        };
        self.enhanced.step(dt, car, event);
        if self.intensity == 0.0 && !self.nitrous && self.live == 0 {
            return;
        }
        // The burning nitrous flames blue in place of the orange blow-off and backfire.
        // A variant's effects can stand in for the game's flames (`variant.rs`): then they spawn nothing.
        let orange = if self.nitrous || replaced.1 { 0.0 } else { self.intensity };
        let blue = if self.nitrous && !replaced.0 { 1.0 } else { 0.0 };
        for (lanes, intensity) in [(&mut self.lanes, orange), (&mut self.blue, blue)] {
            for lane in lanes {
                for (emitter, pipe) in lane.emitters.iter_mut().zip(&self.pipes) {
                    emitter.set_enabled(intensity > 0.0);
                    let frame = Frame { to_world: car.to_world * *pipe, inherit_velocity: car.velocity, intensity };
                    emitter.step(dt, &frame);
                }
            }
        }
        self.live = self.live();
    }

    fn age(&mut self, dt: f32) {
        self.enhanced.age(dt);
        self.intensity = 0.0;
        self.nitrous = false;
        if self.live == 0 {
            return;
        }
        for emitter in self.lanes.iter_mut().chain(&mut self.blue).flat_map(|l| l.emitters.iter_mut()) {
            emitter.set_enabled(false);
            emitter.step(dt, &Frame::default());
        }
        self.live = self.live();
    }

    fn disconnect(&mut self) {
        self.enhanced.disconnect();
        self.shift.reset();
        self.backfire.reset();
        self.intensity = 0.0;
        self.nitrous = false;
    }

    #[cfg(test)]
    fn clear(&mut self) {
        self.disconnect();
        for emitter in self.lanes.iter_mut().chain(&mut self.blue).flat_map(|l| l.emitters.iter_mut()) {
            emitter.clear();
        }
        self.live = 0;
    }

    /// Fill lane `lane`'s vertices with its quads, far to near; false when it has no particles.
    fn quads(&mut self, lane: usize, camera: Vec3, forward: Vec3, flame: Option<FlameProfile>) -> bool {
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
        let up = right.cross(forward).normalize_or(Vec3::Z);
        let blue = lane >= self.lanes.len();
        let gain = match self.textures[self.lane(lane).texture].blend {
            BlendMode::Additive => ADDITIVE_GAIN,
            _ => 1.0,
        };
        let lane = lane_of(&mut self.lanes, &mut self.blue, lane);
        self.order.clear();
        lane.vertices.clear();
        for sprite in lane.emitters.iter().flat_map(Emitter::sprites) {
            let corners = sprite.corners(right, up, forward);
            let mut color = sprite.color;
            if blue {
                color = tint_blue(color);
                color = apply_flame_profile(color, flame);
            }
            color[3] = (f32::from(color[3]) * gain).min(255.0) as u8;
            self.order.push(((sprite.position - camera).dot(forward), corners, color, sprite.uv));
        }
        if self.order.is_empty() {
            return false;
        }
        self.order.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
        for &(_, c, color, uv) in &self.order {
            // The texture's top edge is v = 0: start from the upper-left corner.
            EffectLayer::particle_quad_uv(&mut lane.vertices, [c[3], c[2], c[1], c[0]], color, [0.0; 2], uv);
        }
        true
    }
}

impl ExhaustFlames {
    /// The nitrous variant the player chose, if it is a mod's: its particle effects are loaded at the next frame
    /// ([`Self::load_variant`]). `None` is the game's own flames only.
    pub fn set_effects(&mut self, profile: Option<std::sync::Arc<blackbox_modding::content::NitrousProfile>>) {
        self.variant.set_wanted(profile);
    }

    /// Load the particle effects of the variant in effect, once the car's pipes are known, or drop them when the variant
    /// changed. Does nothing while the setting is off or nothing changed.
    pub fn load_variant(&mut self, renderer: &mut dyn RenderBackend, dir: &game_install::GameDir) {
        if !self.enabled || !self.variant.needs_load() {
            return;
        }
        let State::Ready(active) = &self.state else { return };
        let pipes = active.pipes.len();
        self.variant.load(renderer, dir, pipes);
    }

    /// The tint of a mod's nitrous variant for the burning flames, or `None` for the game's own.
    pub fn set_profile(&mut self, profile: Option<FlameProfile>) {
        self.profile = profile;
    }

    /// Follow the installed engine level without changing loaded emitters or textures.
    pub fn set_performance(&mut self, performance: nfsmw_data::car::physics::PerformancePackage) {
        self.engine_level = i32::from(performance.get(nfsmw_data::car::physics::PerformanceCategory::Engine));
    }

    #[cfg(test)]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Select the enhanced appearance while retaining the original particle fallback.
    pub fn set_enhanced(&mut self, on: bool) {
        self.enhanced = on;
        if let State::Ready(active) = &mut self.state {
            active.enhanced.enabled = on;
        }
    }

    pub fn set_appearance(&mut self, appearance: &crate::garage::ExhaustSelection) {
        if let State::Ready(active) = &mut self.state {
            active.enhanced.appearance = appearance.clone();
        }
    }

    pub fn set_preview(&mut self, on: bool) {
        if let State::Ready(active) = &mut self.state {
            active.enhanced.set_preview(on);
        }
    }

    pub fn set_backfire(
        &mut self,
        style: crate::settings::BackfireStyle,
        preview: Option<crate::garage::BackfireEvent>,
        garage_open: bool,
    ) {
        if let State::Ready(active) = &mut self.state {
            active.enhanced.backfire_style = style;
            if !garage_open {
                active.enhanced.clear_backfire_preview();
                return;
            }
            if let Some(event) = preview {
                active.enhanced.preview_backfire(event);
            }
        }
    }

    /// Match the rendered car between fixed physics steps, keeping the flame root on the pipe.
    pub fn set_render_pose(&mut self, pose: Mat4) {
        if let State::Ready(active) = &mut self.state {
            active.enhanced.set_pose(pose);
        }
    }

    pub fn advance_visual(&mut self, dt: f32, pose: Mat4) {
        if let State::Ready(active) = &mut self.state {
            active.enhanced.advance_visual(dt, pose);
        }
    }

    /// Turning off unloads textures; turning on loads the car's flames at the next frame.
    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
        if !on {
            self.unload();
        }
    }

    /// Forget the loaded car (it is another car now, or the setting went off): its textures are freed by
    /// [`Self::release`].
    pub fn unload(&mut self) {
        self.pops = 0;
        self.forced = false;
        self.variant.unload();
        let State::Ready(active) = std::mem::replace(&mut self.state, State::Unloaded) else { return };
        self.stale.extend(active.textures.iter().filter_map(|t| t.handle));
        self.stale.extend(active.enhanced.texture);
    }

    /// Whether the next frame should load the car's flames.
    pub fn wants_load(&self) -> bool {
        self.enabled && matches!(self.state, State::Unloaded)
    }

    /// Free the GPU textures of flames that were unloaded.
    pub fn release(&mut self, renderer: &mut dyn RenderBackend) {
        self.variant.release(renderer);
        for handle in self.stale.drain(..) {
            renderer.destroy_texture(handle);
        }
    }

    #[cfg(test)]
    fn active(&self) -> Option<&Active> {
        match &self.state {
            State::Ready(active) => Some(active),
            _ => None,
        }
    }

    /// Sputter pops the sound reported; they act at the next physics step.
    pub fn note_pops(&mut self, pops: u32) {
        if self.enabled && matches!(self.state, State::Ready(_)) {
            self.pops = self.pops.saturating_add(pops);
        }
    }

    /// One physics step.
    pub fn step(&mut self, dt: f32, car: &CarState) {
        let (pops, forced) = (std::mem::take(&mut self.pops), std::mem::take(&mut self.forced));
        let State::Ready(active) = &mut self.state else { return };
        let replaced = (self.variant.replaces_burn(), self.variant.replaces_pop());
        active.step(dt, car, pops, forced, self.engine_level, replaced);
        let pop = active.intensity;
        self.variant.step(dt, car, &active.pipes, pop);
    }

    /// Let the particles alive finish their life without spawning any (the physics is parked).
    pub fn age(&mut self, dt: f32) {
        if let State::Ready(active) = &mut self.state {
            active.age(dt);
        }
        self.variant.age(dt);
    }

    /// The car was put somewhere else: forget the shift event and the backfire; the particles alive finish their
    /// short life.
    pub fn disconnect(&mut self) {
        self.pops = 0;
        if let State::Ready(active) = &mut self.state {
            active.disconnect();
        }
    }

    /// Remove every particle and forget the events.
    #[cfg(test)]
    pub fn clear(&mut self) {
        self.pops = 0;
        if let State::Ready(active) = &mut self.state {
            active.clear();
        }
    }

    /// Live particles.
    #[cfg(test)]
    pub fn live(&self) -> usize {
        self.active().map_or(0, |a| a.live)
    }

    /// Append one textured batch per particle texture to `layer`, each sorted far to near. Call
    /// [`Self::reclaim`] once the layer has been uploaded.
    pub fn geometry(&mut self, camera: Vec3, forward: Vec3, layer: &mut EffectLayer) {
        self.own_geometry(camera, forward, layer);
        self.variant.geometry(camera, forward, layer);
    }

    /// The game's own flames' batches.
    fn own_geometry(&mut self, camera: Vec3, forward: Vec3, layer: &mut EffectLayer) {
        let flame = self.profile;
        let State::Ready(active) = &mut self.state else { return };
        active.handed.clear();
        if active.enhanced.geometry(&active.pipes, &mut layer.flames) {
            return;
        }
        if active.live == 0 {
            return;
        }
        for lane in 0..active.lane_count() {
            let texture = &active.textures[active.lane(lane).texture];
            let (blend, Some(handle)) = (texture.blend, texture.handle) else { continue };
            if !active.quads(lane, camera, forward, flame) {
                continue;
            }
            let vertices = std::mem::take(&mut lane_of(&mut active.lanes, &mut active.blue, lane).vertices);
            active.handed.push((lane, handle, layer.textured.len()));
            layer.textured.push(TexturedEffect { texture: handle, blend, vertices });
        }
    }

    /// Append flames and constrain only these new volumes against resident world collision.
    pub fn geometry_in_world(
        &mut self,
        camera: Vec3,
        forward: Vec3,
        world: &blackbox_collision::CollisionWorld,
        layer: &mut EffectLayer,
    ) {
        let start = layer.flames.len();
        self.geometry(camera, forward, layer);
        if let State::Ready(active) = &self.state {
            active.enhanced.contact(world, layer, start);
        }
    }

    /// Take the vertex buffers back from `layer` so that the next frame fills them again without allocating.
    pub fn reclaim(&mut self, layer: &mut EffectLayer) {
        self.variant.reclaim(layer);
        let State::Ready(active) = &mut self.state else { return };
        for (lane, handle, index) in active.handed.drain(..) {
            let Some(batch) = layer.textured.get_mut(index).filter(|b| b.texture == handle) else { continue };
            lane_of(&mut active.lanes, &mut active.blue, lane).vertices = std::mem::take(&mut batch.vertices);
        }
    }
}

#[cfg(test)]
mod tests;
