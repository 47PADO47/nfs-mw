//! Fixed-tick scenery and detached-part cleanup. Spec: `docs/specs/prop-lifetime.md`.

use nfsmw_data::world::PropDynamics;

const REST_DELAY: f32 = 0.125;

#[derive(Debug, Clone)]
pub(super) struct Lifetime {
    pub active: bool,
    pub sinking: bool,
    offscreen: f32,
    part_offscreen: f32,
    rest: f32,
    sink: f32,
    off_world: f32,
}

impl Default for Lifetime {
    fn default() -> Self {
        Self {
            active: false,
            sinking: false,
            offscreen: 0.0,
            part_offscreen: 0.0,
            rest: REST_DELAY,
            sink: 0.0,
            off_world: 0.0,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Observation {
    pub fragment: bool,
    pub vehicle_part: bool,
    pub visible: bool,
    pub in_world: bool,
    pub resting: bool,
}

pub(super) enum Change {
    Keep,
    Sink(f32),
    Release,
}

impl Lifetime {
    pub fn activate(&mut self, spec: &PropDynamics) {
        if self.active || self.sinking {
            return;
        }
        self.active = true;
        self.offscreen = spec.kill_off_screen;
        self.part_offscreen = 0.0;
        self.rest = REST_DELAY;
        self.off_world = 0.0;
    }

    pub fn tick(&mut self, spec: &PropDynamics, view: Observation, dt: f32) -> Change {
        if !self.active || !dt.is_finite() || dt <= 0.0 {
            return Change::Keep;
        }
        if self.sinking {
            self.sink -= dt;
            if self.sink <= 0.0 {
                return Change::Release;
            }
            return Change::Sink(-dt * spec.dropout[1]);
        }
        if view.vehicle_part && spec.kill_off_screen > 0.0 {
            self.part_offscreen = match view.visible {
                true => 0.0,
                false => self.part_offscreen + dt,
            };
            if self.part_offscreen > spec.kill_off_screen {
                return Change::Release;
            }
        }
        if !view.vehicle_part && self.offscreen > 0.0 {
            self.offscreen = match view.visible {
                true => spec.kill_off_screen,
                false => self.offscreen - dt,
            };
            if self.offscreen < 0.0 {
                return Change::Release;
            }
        }
        self.off_world = match view.in_world || spec.allow_off_world {
            true => 0.0,
            false => self.off_world + dt,
        };
        if self.off_world >= 1.0 {
            return Change::Release;
        }
        if !view.resting {
            self.rest = REST_DELAY;
            return Change::Keep;
        }
        if self.rest > 0.0 {
            self.rest -= dt;
            return Change::Keep;
        }
        // A rewrite choice: a visible prop stays where it came to rest, so the sink and placement restore wait for it to leave the view.
        if spec.dropout[0] > 0.0 {
            if view.visible {
                return Change::Keep;
            }
            self.sinking = true;
            self.sink = spec.dropout[0];
            return Change::Keep;
        }
        if !view.vehicle_part {
            self.active = false;
        }
        Change::Keep
    }
}

/// Records body `id` for the cleanup scan. Every activation goes through here, so the scan misses no live lifetime.
pub(super) fn track_lifetime(live: &mut Vec<u32>, id: u32) {
    let Err(at) = live.binary_search(&id) else {
        return;
    };
    live.insert(at, id);
}

impl super::PropWorld {
    /// Cleanup observes the current physics camera, not interpolated render poses.
    #[cfg(test)]
    pub fn update_lifetimes(&mut self, ground: &dyn blackbox_vehicle::Ground, view: glam::Mat4, dt: f32) {
        self.update_lifetimes_seen_by(ground, view, &[], dt);
    }

    /// Cleanup observes the current physics camera of every player, not interpolated render poses: a body any of them
    /// sees counts as visible. `view` is the lead's, `others` the other players' (none outside co-op and sessions).
    pub fn update_lifetimes_seen_by(
        &mut self,
        ground: &dyn blackbox_vehicle::Ground,
        view: glam::Mat4,
        others: &[glam::Mat4],
        dt: f32,
    ) {
        let _span = bevy_log::info_span!("world::props_lifetimes").entered();
        let frustum = blackbox_scene::Frustum::from_view_proj(&view);
        let others: Vec<_> = others.iter().map(blackbox_scene::Frustum::from_view_proj).collect();
        // Only tracked bodies are visited, in ascending id order. Bodies that stay active are tracked again;
        // the rest, including removed bodies, drop out. Both lists are reused across ticks.
        let mut ids = std::mem::take(&mut self.lifetime_scratch);
        ids.clear();
        ids.extend_from_slice(&self.live_lifetimes);
        self.live_lifetimes.clear();
        for &id in &ids {
            let Some(c) = self.colliders.get_mut(&id) else {
                continue;
            };
            if !c.body.as_ref().is_some_and(|b| b.lifetime.active) {
                continue;
            }
            let bounds = c.pose(1.0).1;
            let visible = frustum.intersects(&bounds) || others.iter().any(|other| other.intersects(&bounds));
            let (Some(body), Some(spec)) = (&mut c.body, &c.shape.dynamics) else { continue };
            let radius = body.rigid.radius();
            let top = body.rigid.position + glam::Vec3::Y * radius;
            let in_world = spec.allow_off_world || ground.hit(top, glam::Vec3::NEG_Y, 10000.0).is_some();
            let observation = Observation {
                fragment: self.fragments.contains_key(&id),
                vehicle_part: self.fragments.get(&id).is_some_and(|fragment| !fragment.scenery),
                visible,
                in_world,
                resting: body.rigid.state == blackbox_vehicle::rigid_body::BodyState::Asleep
                    && !body.attached
                    && !body.object_contact,
            };
            match body.lifetime.tick(spec, observation, dt) {
                Change::Keep => {}
                Change::Sink(delta) => {
                    body.rigid.position.y += delta;
                }
                Change::Release if observation.fragment => {
                    self.remove_fragment(id);
                    continue;
                }
                Change::Release => {
                    body.restore(spec);
                    self.reset_model(id);
                }
            }
            self.regrid(id);
            let active = self.colliders.get(&id).and_then(|c| c.body.as_ref()).is_some_and(|b| b.lifetime.active);
            if active {
                self.live_lifetimes.push(id);
            }
        }
        self.lifetime_scratch = ids;
    }
}
