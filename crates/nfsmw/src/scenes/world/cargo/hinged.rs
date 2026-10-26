//! Hinged parts of a mod car leaving it (`docs/modding/car-damage-parts.md`).
//!
//! A part leaves when its zones' damage reaches its release level, or when the car is destroyed or busted (every part that
//! can detach). It becomes a fragment of the prop solver, the way the installed vehicle fragments do (`super::update`):
//! a rigid body with the car's pose, velocity and a kick, that falls, collides, comes to rest and is released after a
//! while. The car stops drawing it (its slot joins `hidden_slots`). The part keeps the sound of the installed detached
//! parts: the prop contact sound of [`DEBRIS_SOUND`].

use std::rc::Rc;
use std::sync::Arc;

use blackbox_attrib::vlt_hash;
use glam::Vec3;
use nfsmw_data::car::mods::hinged::{
    self, DEBRIS_SOUND, Parent, debris_bounds, debris_dynamics, debris_motion, part_axis, part_frame,
};

use crate::scenes::world::drive::{CarRig, CarSim, STEP};
use crate::scenes::world::props::{FragmentSpawn, PropWorld};

/// Move the parts that follow the car's speed (an active spoiler) one step: the car's speed decides whether each is going
/// up or down.
fn advance(sim: &mut CarSim, hinges: &[hinged::Hinge]) {
    if hinges.iter().all(|hinge| hinge.active.is_none()) {
        return;
    }
    let mph = sim.velocity().length() * hinged::MPS_TO_MPH;
    let Some(events) = sim.events.as_mut() else { return };
    events.deployment.resize(hinges.len(), hinged::Deployment::default());
    for (hinge, deployment) in hinges.iter().zip(&mut events.deployment) {
        if let Some(active) = &hinge.active {
            deployment.step(active, mph, STEP);
        }
    }
}

/// Move the active parts of the car in `sim`, and release the hinged parts that are due, as bodies of `props`.
pub(super) fn release(sim: &mut CarSim, rig: &CarRig, props: &mut PropWorld) {
    let hinges = rig.hinges();
    if hinges.is_empty() {
        return;
    }
    advance(sim, hinges);
    let Some(events) = sim.events.as_ref() else { return };
    let wrecked = events.is_destroyed();
    let due: Vec<(usize, f32, f32)> = hinges
        .iter()
        .enumerate()
        .filter(|(n, _)| !events.hidden_slots.contains(&hinged::slot_of(*n)))
        .filter_map(|(n, hinge)| {
            let fraction = hinge.fraction(|zone| events.damage.zone(zone).panel);
            let deployed = events.deployment.get(n).map_or(0.0, |deployment| deployment.amount);
            hinge.releases(fraction, wrecked).then_some((n, hinge.swing(fraction), deployed))
        })
        .collect();
    if due.is_empty() {
        return;
    }
    let body = sim.body();
    let parent =
        Parent { position: body.position, velocity: body.linear_velocity, angular_velocity: body.angular_velocity };
    let model = sim.pose().transform();
    let Some(events) = sim.events.as_mut() else { return };
    let owner = Rc::downgrade(&events.owner);
    for (n, turned, deployed) in due {
        let hinge = &hinges[n];
        let slot = hinged::slot_of(n);
        // Whether or not the body could be made, the car does not draw the part again.
        events.hidden_slots.insert(slot);
        let Some(spawn) = spawn_of(rig, n, (turned, deployed), &parent, model) else {
            log::warn!("hinged part {} has no body to leave the car with", hinge.name);
            continue;
        };
        let spawn = FragmentSpawn { owner: owner.clone(), ..spawn };
        let released = props.spawn_fragment(spawn);
        log::debug!("hinged part {} left the car (wrecked {wrecked}): {released:?}", hinge.name);
    }
}

/// What the prop solver needs to start the part `n` of the car drawn by `rig`, turned by `stance.0` degrees and deployed
/// by `stance.1` (0 to 1).
fn spawn_of(rig: &CarRig, n: usize, stance: (f32, f32), parent: &Parent, model: glam::Mat4) -> Option<FragmentSpawn> {
    let (turned, deployed) = stance;
    let hinge = rig.hinges().get(n)?;
    let slot = hinged::slot_of(n);
    let placement = rig.model().placements.iter().find(|p| p.slot == slot)?;
    let solid = rig.model().solids.get(&placement.solid)?;
    let (low, high) = (Vec3::from_array(solid.bounds_min), Vec3::from_array(solid.bounds_max));
    let stand = hinge.transform(turned, deployed);
    let turn = stand * placement.transform;
    let mut instances = rig.fragment_instances(slot, None);
    if instances.is_empty() {
        return None;
    }
    for instance in &mut instances {
        instance.transform = stand * instance.transform;
    }
    let centre = turn.transform_point3((low + high) * 0.5);
    let (position, orientation) = part_frame(model, centre);
    let motion = debris_motion(parent, position, part_axis(model, Vec3::from_array(hinge.axis)), turned);
    Some(FragmentSpawn {
        bounds: Arc::new(debris_bounds((high - low) * 0.5)),
        root: 0,
        spec: Arc::new(debris_dynamics(hinge.mass_kg, vlt_hash(DEBRIS_SOUND))),
        pose: (position, orientation),
        model,
        velocity: (motion.velocity, motion.spin),
        instances,
        owner: std::rc::Weak::new(),
    })
}
