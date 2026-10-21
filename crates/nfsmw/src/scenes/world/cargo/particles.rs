//! Installed vehicle one-shot effects, including cop glass, use collision markers and incoming velocity.
use crate::scenes::world::{
    prop_effects::{Command, Source},
    props::PropWorld,
    space,
};
use blackbox_particles::Frame;
use glam::{Mat3, Mat4, Quat, Vec3};
use nfsmw_data::{car::physics::CarGeometry, world::sequences::ObjectEffect};
use std::rc::Weak;

pub(super) struct Context {
    pub parent: (Vec3, Quat),
    pub velocity: Vec3,
    pub model: Mat4,
    pub owner: Weak<()>,
}

pub(super) fn play(props: &mut PropWorld, geometry: &CarGeometry, context: Context, effect: ObjectEffect) {
    let Context { parent, velocity, model, owner } = context;
    if effect.effect == 0 {
        return;
    }
    if effect.instance != 0 {
        log::debug!("tracked vehicle particle effect {:08X} is not supported", effect.effect);
        return;
    }
    let Some(to_world) = frame(&geometry.bounds, parent, model, effect.position) else { return };
    let frame =
        Frame { to_world, inherit_velocity: space::to_render(velocity.to_array()), intensity: effect.intensity };
    props.queue_vehicle_particles(Command::Play(Source { id: 0, owner, effect, frame }));
}

fn frame(bounds: &blackbox_collision::BoundsSet, parent: (Vec3, Quat), model: Mat4, marker: u32) -> Option<Mat4> {
    if marker == 0 {
        return Some(model);
    }
    let Some(node) = bounds.children(bounds.root()).iter().find(|n| n.name_hash == marker) else {
        log::debug!("vehicle effect missing marker {marker:08X}");
        return None;
    };
    let local = Quat::from_array(node.orientation);
    if !local.is_finite() || local.length_squared() < 1e-6 {
        return None;
    }
    let axes = Mat3::from_quat(parent.1 * local.normalize());
    let rotation = Quat::from_mat3(&Mat3::from_cols(
        space::to_render(axes.z_axis.to_array()),
        space::to_render((-axes.x_axis).to_array()),
        space::to_render(axes.y_axis.to_array()),
    ));
    let position = space::to_render((parent.0 + parent.1 * Vec3::from_array(node.position)).to_array());
    Some(Mat4::from_rotation_translation(rotation, position))
}
