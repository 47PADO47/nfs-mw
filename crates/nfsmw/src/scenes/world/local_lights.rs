//! Local-light test slice. These are enhanced lights, not recovered original physical lamp parameters.
use super::{WorldScene, car_lights::Kind};
#[cfg(test)]
mod inventory;
#[cfg(test)]
mod lantern_review;
#[cfg(test)]
mod night_review;
#[cfg(test)]
mod tests;
use blackbox_gfx::{InstanceKey, LocalLight, RenderBackend};
use glam::Vec3;
use std::collections::HashMap;

mod catalog;
mod fixtures;
pub(super) mod lantern;
mod profile;
mod selection;
mod taillights;
pub(super) use fixtures::{Fixture, FixturePlace};
use selection::{Candidate, Switches};

/// Glass emission power by owning placement.
pub(super) type Emission = HashMap<InstanceKey, f32, foldhash::fast::FixedState>;

/// The light scan's per-frame lists, kept between frames so a warmed-up frame allocates nothing.
#[derive(Default)]
pub(super) struct Scratch {
    lights: Vec<LocalLight>,
    tail: Vec<LocalLight>,
    traffic: Vec<LocalLight>,
    candidates: Vec<Candidate>,
    owners: Vec<InstanceKey>,
    drawn: Vec<(InstanceKey, blackbox_gfx::MeshHandle)>,
}

impl WorldScene {
    pub(super) fn update_local_lights(&mut self, renderer: &mut dyn RenderBackend) {
        let options = renderer.render_experiments();
        self.street_glows.clear();
        self.fixture_emission.clear();
        let mut scratch = std::mem::take(&mut self.local_scratch);
        let Scratch { lights, tail, traffic, candidates, owners, drawn } = &mut scratch;
        lights.clear();
        tail.clear();
        traffic.clear();
        candidates.clear();
        if options.local_headlights
            && let Some(drive) = self.players.lead()
        {
            let (lamps, _, transform, state) = drive.lamp_source();
            if state.headlights && !state.dark {
                for lamp in lamps.lamps().iter().filter(|l| l.kind == Kind::Head && l.flags & state.broken == 0).take(2)
                {
                    let direction = transform.transform_vector3(lamp.direction).normalize_or_zero();
                    lights.push(LocalLight {
                        position: (transform.transform_point3(lamp.at) + direction * 0.16).to_array(),
                        direction: direction.to_array(),
                        casts_shadow: true,
                        shadow_exclude: None,
                        upper_plane: Some(transform.transform_vector3(Vec3::Z).normalize_or_zero().to_array()),
                        range: 45.0,
                        outer_cos: 0.80,
                        inner_cos: 0.94,
                        color: (Vec3::new(1.0, 0.96, 0.86) * options.local_headlight_strength * 1.3).to_array(),
                    });
                }
            }
        }
        if options.local_tail_lights
            && let Some(drive) = self.players.lead()
        {
            let (lamps, _, transform, state) = drive.lamp_source();
            taillights::emit(&lamps, transform, state, tail);
        }
        let traffic_strength = match options.full_day_night {
            true => self.sun.cycle().map_or_else(|| super::full_cycle::at(self.day.hour()).night, |c| c.night),
            false => 1.0,
        };
        if options.local_traffic_headlights
            && traffic_strength > 0.0
            && let Some(cars) = &self.traffic
        {
            let camera = self.gameplay_lens(renderer.aspect_ratio()).position;
            let available_cars = selection::remaining_slots(options.local_light_limit, lights.len() + tail.len()) / 2;
            cars.local_headlights(
                camera,
                options.local_headlight_strength * traffic_strength,
                options.local_traffic_limit.min(available_cars as u32),
                traffic,
            );
            let priority = traffic.len().min(4);
            lights.extend(traffic.drain(..priority));
        }
        let street_night = match options.full_day_night {
            true => self.sun.cycle().map_or_else(|| super::full_cycle::at(self.day.hour()).night, |c| c.night),
            false => f32::from(u8::from(options.experimental_night)),
        };
        let switches = Switches {
            lamps: options.local_lamps,
            street: options.local_street_lights && street_night > 0.0 && options.local_street_strength > 0.0,
            street_night,
            street_strength: options.local_street_strength,
            street_range: options.local_street_range,
            lamp_strength: options.local_light_strength,
            street_glow: options.local_street_glow,
        };
        if switches.any() {
            let camera = self.gameplay_lens(renderer.aspect_ratio()).position;
            self.scan_fixtures(camera, &switches, selection::glow_distance(options.local_light_distance), candidates);
            selection::sort(candidates);
            // The main view of the last frame says which LOD each lantern shows.
            selection::resolve_drawn_glass(candidates, &self.visible, owners, drawn);
            selection::push_glows(candidates, &switches, &mut self.street_glows);
            let used = lights.len() + traffic.len() + tail.len();
            let remaining = selection::remaining_slots(options.local_light_limit, used);
            selection::push_lights(candidates, options.local_light_distance, remaining, lights);
        }
        // Additional traffic beams still illuminate but do not take nearby tunnel shadow slots.
        lights.append(traffic);
        let remaining = selection::remaining_slots(options.local_light_limit, lights.len());
        lights.extend(tail.drain(..).take(remaining));
        renderer.set_local_lights(lights.as_slice());
        self.local_scratch = scratch;
    }

    /// The admitted fixtures of the drawn tiles within `distance` of `camera`, recording street glass emission.
    fn scan_fixtures(&mut self, camera: Vec3, switches: &Switches, distance: f32, out: &mut Vec<Candidate>) {
        let props = self.residency.props();
        for (placed, place) in self.residency.fixture_placements() {
            if !self.scenery_groups.admits(placed.group) {
                continue;
            }
            let Some((transform, _)) = super::visibility::current_pose(placed, props) else { continue };
            let up = transform.transform_vector3(Vec3::Z);
            for (index, fixture) in place.fixtures.iter().enumerate() {
                if !selection::admits(fixture, switches, || props.fixture_powered(placed.prop_id), up) {
                    continue;
                }
                if fixture.street {
                    self.fixture_emission.insert(placed.key, switches.emission());
                }
                let lods = placed.lods.iter().enumerate().filter(|(_, mesh)| mesh.is_some());
                let all = lods.fold(0u8, |mask, (slot, _)| mask | 1 << slot);
                let glass = place.glass_lods != 0 && place.glass_lods & all == all;
                let fixture = Fixture { glass, ..*fixture };
                let mut candidate = selection::candidate(&fixture, transform, placed.key, index, camera, switches);
                if !glass && place.glass_lods != 0 {
                    for (slot, mesh) in placed.lods.iter().enumerate() {
                        candidate.glass_meshes[slot] = mesh.filter(|_| place.glass_lods & (1 << slot) != 0);
                    }
                }
                // Visible bulbs remain useful beyond the surface-light and shadow budgets.
                if candidate.distance < distance {
                    out.push(candidate);
                }
            }
        }
    }
}

/// Apply current fixture power to the actual drawn mesh, including reflection views and selected LODs. Instances
/// of unpowered owners keep the emission their source gave them.
pub(super) fn set_instance_emission<S: std::hash::BuildHasher>(
    instances: &mut [blackbox_gfx::Instance],
    power: &HashMap<InstanceKey, f32, S>,
) {
    if power.is_empty() {
        return;
    }
    for instance in instances {
        if instance.owner_key == InstanceKey::TRANSIENT {
            continue;
        }
        if let Some(&power) = power.get(&instance.owner_key) {
            instance.emission = power;
        }
    }
}

#[cfg(test)]
mod emission_tests {
    use super::*;
    use blackbox_gfx::{Instance, MeshHandle};

    #[test]
    fn emission_only_touches_powered_owners() {
        let mesh = MeshHandle::from_raw(1);
        let lit = InstanceKey::new(1, 1);
        let mut other = Instance::keyed(mesh, glam::Mat4::IDENTITY, InstanceKey::new(1, 2));
        other.emission = 0.25;
        let mut instances =
            [Instance::keyed(mesh, glam::Mat4::IDENTITY, lit), other, Instance::new(mesh, glam::Mat4::IDENTITY)];
        set_instance_emission(&mut instances, &Emission::default());
        assert_eq!(instances.map(|i| i.emission), [0.0, 0.25, 0.0]);
        let mut power = Emission::default();
        power.insert(lit, 2.0);
        power.insert(InstanceKey::TRANSIENT, 9.0);
        set_instance_emission(&mut instances, &power);
        assert_eq!(instances.map(|i| i.emission), [2.0, 0.25, 0.0], "other sources and transient instances are kept");
    }
}
