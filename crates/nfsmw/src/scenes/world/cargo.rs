//! Host dispatch of installed vehicle fragment events.

use std::rc::Rc;

use blackbox_attrib::vlt_hash;
use glam::{Quat, Vec3};
use nfsmw_data::car::{LAYOUT, physics::PhysicsData};
use nfsmw_data::world::sequences::WorldEvent;

use super::drive::{CarRig, CarSim};
use super::props::{FragmentSpawn, PropWorld};
mod particles;

pub fn update(sim: &mut CarSim, rig: &CarRig, props: &mut PropWorld, data: &PhysicsData, view: Vec3) {
    update_with(sim, props, data, view, |slot| {
        let instances = rig.fragment_instances(slot);
        (!instances.is_empty()).then_some(instances)
    });
}

fn update_with(
    sim: &mut CarSim,
    props: &mut PropWorld,
    data: &PhysicsData,
    view: Vec3,
    instances: impl Fn(usize) -> Option<Vec<blackbox_render::Instance>>,
) {
    let Some(geometry) = sim.geometry.clone() else { return };
    let body = sim.body();
    let parent = (body.position, body.orientation);
    let velocity = (body.linear_velocity, body.angular_velocity);
    let model = sim.pose().transform();
    let distance = sim.pose().position.distance(view);
    let Some(events) = sim.events.as_mut() else { return };
    for effect in events.tick(distance) {
        let fragment = match effect.event {
            WorldEvent::Fragment(fragment) => fragment,
            WorldEvent::ObjectEffect(particle) => {
                particles::play(
                    props,
                    &geometry,
                    particles::Context {
                        parent,
                        velocity: effect.velocity.unwrap_or(velocity.0),
                        model,
                        owner: Rc::downgrade(&events.owner),
                    },
                    particle,
                );
                continue;
            }
            other => {
                log::debug!("vehicle event effect not yet implemented: {other:?}");
                continue;
            }
        };
        if fragment.joint != 0 || fragment.no_physics || fragment.simple_physics || fragment.hide_parent {
            log::warn!("vehicle fragment {:08X}: unsupported joint/physics/parent flags", fragment.part);
            continue;
        }
        let Some(slot) = LAYOUT.slots.iter().position(|slot| vlt_hash(slot.name) == fragment.part) else { continue };
        if (events.spawned.contains(&fragment.part) && !fragment.allow_duplicates)
            || (events.hidden_slots.contains(&slot) && !fragment.allow_hidden_spawn)
        {
            continue;
        }
        let bounds = &geometry.bounds;
        let Some(child) = bounds.children(bounds.root()).iter().position(|n| n.name_hash == fragment.collision) else {
            continue;
        };
        let root = usize::try_from(bounds.root().first_child).unwrap_or(0) + child;
        let node = &bounds.nodes[root];
        let Some(spec) = data.fragment_dynamics(fragment.attributes) else { continue };
        let Some(instances) = instances(slot) else { continue };
        let local_q = Quat::from_array(node.orientation);
        if !local_q.is_finite() || local_q.length_squared() < 1e-6 {
            continue;
        }
        let spawned = props.spawn_fragment(FragmentSpawn {
            bounds: bounds.clone(),
            root,
            spec,
            pose: (parent.0 + parent.1 * Vec3::from_array(node.position), parent.1 * local_q.normalize()),
            model,
            velocity: (
                effect.velocity.unwrap_or(velocity.0) * fragment.inherit_velocity,
                velocity.1 * fragment.inherit_velocity,
            ),
            instances,
            owner: Rc::downgrade(&events.owner),
        });
        if spawned.is_some() {
            events.spawned.insert(fragment.part);
            if fragment.hide_part {
                events.hidden_slots.insert(slot);
            }
            log::debug!("released vehicle fragment {:08X} as {:?}", fragment.part, spawned);
        }
    }
}

#[cfg(test)]
mod tests;
