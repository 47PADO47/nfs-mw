//! Flames at the tail pipes: the nitrous, the blow-off after a gear change and the backfire at a sputter pop
//! (spec: exhaust-flames.md, sections 2 and 8).
//!
//! The car's `ecar` record names a particle group; its emitters (read from the install) are run at every pipe
//! marker of the car model with `blackbox-particles` and drawn as textured billboards, additive or alpha-blended
//! as the particle textures say. Nothing here is stored in the repository: definitions and textures come from
//! the install, and only while the `exhaust_flames` setting is on.

mod command;
mod load;
mod trigger;

use blackbox_particles::{Emitter, Frame};
use blackbox_render::{BlendMode, EffectLayer, EffectVertex, Renderer, TextureHandle, TexturedEffect};
use glam::{Mat4, Vec3};

use trigger::{Backfire, ShiftEvent, ShiftTiming, blowoff_allowed, flame_intensity};

/// Most tail pipes that flame, and most live particles of one pipe's emitter (spec 8.4).
pub const MAX_PIPES: usize = 4;
pub const EMITTER_LIMIT: usize = 96;

/// What the effects need of the car for one physics step.
pub struct CarState {
    /// The car model's frame in the world.
    pub to_world: Mat4,
    pub velocity: Vec3,
    /// -1 reverse, 0 neutral, 1 and up forward.
    pub gear: i32,
    /// The nitrous is burning.
    pub nitrous: bool,
    /// The throttle pedal, 0 to 1.
    pub throttle: f32,
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
    /// Each pipe's frame in the car model's space.
    pipes: Vec<Mat4>,
    lanes: Vec<Lane>,
    textures: Vec<SpriteTexture>,
    timing: ShiftTiming,
    engine_upgrades: i32,
    shift: ShiftEvent,
    backfire: Backfire,
    /// How strongly the pipes spawned in the latest step, 0 when quiet.
    intensity: f32,
    /// Live particles after the latest step.
    live: usize,
    order: Vec<(f32, [Vec3; 4], [u8; 4])>,
    /// The batches handed to the layer by the latest `geometry`: the lane, the texture and the layer's index.
    handed: Vec<(usize, TextureHandle, usize)>,
}

enum State {
    /// Nothing is loaded (the setting is off, or the car is new and the next frame loads it).
    Unloaded,
    /// The car has no pipes, no group or no textures.
    Missing,
    Ready(Box<Active>),
}

pub struct ExhaustFlames {
    enabled: bool,
    state: State,
    /// GPU textures to free at the next chance (a renderer is needed).
    stale: Vec<TextureHandle>,
    /// The installed engine upgrade level (the career will supply it; for now a console setting).
    engine_level: i32,
    /// Sputter pops reported since the last physics step.
    pops: u32,
    /// The console asked for a backfire.
    forced: bool,
}

impl Default for ExhaustFlames {
    /// Off, and nothing allocated.
    fn default() -> Self {
        Self { enabled: false, state: State::Unloaded, stale: Vec::new(), engine_level: 0, pops: 0, forced: false }
    }
}

impl Active {
    fn live(&self) -> usize {
        self.lanes.iter().flat_map(|l| &l.emitters).map(Emitter::len).sum()
    }

    /// One physics step: the shift event and the backfire, then every pipe's emitters (spawning while the pipes
    /// flame). A step with nothing flaming and nothing alive does no more than that.
    fn step(&mut self, dt: f32, car: &CarState, pops: u32, forced: bool, engine_level: i32) {
        self.shift.note_gear(car.gear);
        self.shift.advance(dt, car.gear, car.velocity.length(), self.timing);
        self.backfire.pops(pops, car.throttle, car.gear);
        if forced {
            self.backfire.start();
        }
        let blowoff = blowoff_allowed(engine_level, self.engine_upgrades);
        self.intensity = flame_intensity(car.nitrous, blowoff, &self.shift, &self.backfire);
        self.backfire.advance(dt);
        if self.intensity == 0.0 && self.live == 0 {
            return;
        }
        for lane in &mut self.lanes {
            for (emitter, pipe) in lane.emitters.iter_mut().zip(&self.pipes) {
                emitter.set_enabled(self.intensity > 0.0);
                let frame =
                    Frame { to_world: car.to_world * *pipe, inherit_velocity: car.velocity, intensity: self.intensity };
                emitter.step(dt, &frame);
            }
        }
        self.live = self.live();
    }

    fn age(&mut self, dt: f32) {
        self.intensity = 0.0;
        if self.live == 0 {
            return;
        }
        for emitter in self.lanes.iter_mut().flat_map(|l| l.emitters.iter_mut()) {
            emitter.set_enabled(false);
            emitter.step(dt, &Frame::default());
        }
        self.live = self.live();
    }

    fn disconnect(&mut self) {
        self.shift.reset();
        self.backfire.reset();
        self.intensity = 0.0;
    }

    fn clear(&mut self) {
        self.disconnect();
        for emitter in self.lanes.iter_mut().flat_map(|l| l.emitters.iter_mut()) {
            emitter.clear();
        }
        self.live = 0;
    }

    /// Fill lane `lane`'s vertices with its quads, far to near; false when it has no particles.
    fn quads(&mut self, lane: usize, camera: Vec3, forward: Vec3) -> bool {
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
        let up = right.cross(forward).normalize_or(Vec3::Z);
        let lane = &mut self.lanes[lane];
        self.order.clear();
        lane.vertices.clear();
        for sprite in lane.emitters.iter().flat_map(Emitter::sprites) {
            let corners = sprite.corners(right, up, forward);
            self.order.push(((sprite.position - camera).dot(forward), corners, sprite.color));
        }
        if self.order.is_empty() {
            return false;
        }
        self.order.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
        for &(_, c, color) in &self.order {
            // The texture's top edge is v = 0: start from the upper-left corner.
            EffectLayer::particle_quad(&mut lane.vertices, [c[3], c[2], c[1], c[0]], color, [0.0; 2]);
        }
        true
    }
}

impl ExhaustFlames {
    #[cfg(test)]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// The `exhaust_flames` setting. Turning it off drops the emitters and frees the textures at the next chance;
    /// turning it on loads the car's flames at the next frame.
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
        let State::Ready(active) = std::mem::replace(&mut self.state, State::Unloaded) else { return };
        self.stale.extend(active.textures.iter().filter_map(|t| t.handle));
    }

    /// Whether the next frame should load the car's flames.
    pub fn wants_load(&self) -> bool {
        self.enabled && matches!(self.state, State::Unloaded)
    }

    /// Free the GPU textures of flames that were unloaded.
    pub fn release(&mut self, renderer: &mut Renderer) {
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
        active.step(dt, car, pops, forced, self.engine_level);
    }

    /// Let the particles alive finish their life without spawning any (the physics is parked).
    pub fn age(&mut self, dt: f32) {
        if let State::Ready(active) = &mut self.state {
            active.age(dt);
        }
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
        let State::Ready(active) = &mut self.state else { return };
        active.handed.clear();
        if active.live == 0 {
            return;
        }
        for lane in 0..active.lanes.len() {
            let texture = &active.textures[active.lanes[lane].texture];
            let (blend, Some(handle)) = (texture.blend, texture.handle) else { continue };
            if !active.quads(lane, camera, forward) {
                continue;
            }
            let vertices = std::mem::take(&mut active.lanes[lane].vertices);
            active.handed.push((lane, handle, layer.textured.len()));
            layer.textured.push(TexturedEffect { texture: handle, blend, vertices });
        }
    }

    /// Take the vertex buffers back from `layer` so that the next frame fills them again without allocating.
    pub fn reclaim(&mut self, layer: &mut EffectLayer) {
        let State::Ready(active) = &mut self.state else { return };
        for (lane, handle, index) in active.handed.drain(..) {
            let Some(batch) = layer.textured.get_mut(index).filter(|b| b.texture == handle) else { continue };
            active.lanes[lane].vertices = std::mem::take(&mut batch.vertices);
        }
    }
}

#[cfg(test)]
mod tests;
