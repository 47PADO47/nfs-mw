//! Sequence-to-renderer commands retain source ownership and ordered stops.
use super::*;
use crate::scenes::world::{
    prop_effects::{Command, Source},
    space,
};
use blackbox_particles::Frame;
use nfsmw_data::world::sequences::ObjectEffect;

impl PropWorld {
    pub fn queue_vehicle_particles(&mut self, command: Command) {
        self.scenery.particles.push(command);
    }
    pub fn effect_frame(&self, id: u32, marker: u32) -> Option<Frame> {
        let model = self.scenery.models.get(&id)?;
        let body = &self.colliders.get(&id)?.body.as_ref()?.rigid;
        let visual = self.colliders.get(&id)?.body.as_ref()?.transform(1.0);
        let mut position = visual.w_axis.truncate();
        let mut rotation = glam::Quat::IDENTITY;
        if marker != 0 {
            let root = &model.bounds.nodes[model.bounds_root];
            let node = model.bounds.children(root).iter().find(|n| n.name_hash == marker)?;
            position =
                space::to_render((body.position + body.orientation * Vec3::from_array(node.position)).to_array());
            let local = glam::Quat::from_array(node.orientation);
            if !local.is_finite() || local.length_squared() < 1e-6 {
                return None;
            }
            let axes = glam::Mat3::from_quat(body.orientation * local.normalize());
            rotation = glam::Quat::from_mat3(&glam::Mat3::from_cols(
                space::to_render(axes.z_axis.to_array()),
                space::to_render((-axes.x_axis).to_array()),
                space::to_render(axes.y_axis.to_array()),
            ));
        }
        Some(Frame {
            to_world: Mat4::from_rotation_translation(rotation, position),
            inherit_velocity: space::to_render(body.linear_velocity.to_array()),
            intensity: 1.0,
        })
    }

    pub(super) fn object_effect(&mut self, id: u32, effect: ObjectEffect, data: &PhysicsData) {
        if effect.effect == 0 {
            return;
        }
        let Some(model) = self.scenery.models.get(&id) else { return };
        if !effect.allow_hidden_spawn && model.hidden {
            log::debug!("prop effect {:08X}: source {id} hidden", effect.effect);
            return;
        }
        let Some(mut frame) = self.effect_frame(id, effect.position) else {
            log::debug!("prop effect {:08X}: source {id} missing marker {:08X}", effect.effect, effect.position);
            return;
        };
        frame.intensity = effect.intensity;
        let owner = model.owner.clone();
        if !nfsmw_data::world::object_fx::emitters(data.database(), effect.effect).is_empty() {
            self.scenery.models.get_mut(&id).unwrap().authored_visual = true;
        }
        self.scenery.particles.push(Command::Play(Source { id, owner, effect, frame }));
    }

    pub(super) fn stop_object_effect(&mut self, id: u32, instance: Option<u32>) {
        let Some(model) = self.scenery.models.get(&id) else { return };
        self.scenery.particles.push(Command::Stop { source: id, owner: model.owner.clone(), instance });
    }

    pub fn take_particle_commands(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.scenery.particles)
    }
    pub fn take_authored_blasts(&mut self) -> Vec<Vec3> {
        std::mem::take(&mut self.scenery.authored_blasts)
    }
}
