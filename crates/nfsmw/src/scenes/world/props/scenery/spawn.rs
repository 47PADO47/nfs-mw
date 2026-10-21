//! Named fragment construction preserves both physics and mesh frames.

use super::super::body::PropBody;
use super::{PropWorld, SceneryModel};
use blackbox_vehicle::rigid_body::BodyState;
use glam::{Quat, Vec3};
use nfsmw_data::{car::physics::PhysicsData, world::sequences::Fragment};
use std::{collections::BTreeSet, rc::Weak, sync::Arc};

impl PropWorld {
    pub(super) fn spawn_scenery_fragment(&mut self, id: u32, fragment: Fragment, data: &PhysicsData) -> Option<u32> {
        if self.scenery.models.len() >= 2297 || fragment.joint != 0 {
            return None;
        }
        let model = self.scenery.models.get(&id)?;
        if (!fragment.allow_duplicates && model.spawned.contains(&fragment.part))
            || (!fragment.allow_hidden_spawn && (model.hidden || model.hidden_parts.contains(&fragment.part)))
        {
            return None;
        }
        let parts = model.parts.clone()?;
        let render_root = parts.hierarchy.find(fragment.part)?;
        let visible = parts.hierarchy.visible_nodes(render_root);
        if !visible.iter().any(|i| parts.available.contains(i)) {
            return None;
        }
        let bounds = model.bounds.clone();
        let parent_root = &bounds.nodes[model.bounds_root];
        let child = bounds.children(parent_root).iter().position(|n| n.name_hash == fragment.collision)?;
        let bounds_root = usize::try_from(parent_root.first_child).ok()? + child;
        let node = &bounds.nodes[bounds_root];
        let local = Quat::from_array(node.orientation);
        if !local.is_finite() || local.length_squared() < 1e-6 {
            return None;
        }
        let parent = self.colliders.get(&id)?.body.as_ref()?;
        let pose = (
            parent.rigid.position + parent.rigid.orientation * Vec3::from_array(node.position),
            parent.rigid.orientation * local.normalize(),
        );
        let velocity = (
            parent.rigid.linear_velocity * fragment.inherit_velocity,
            parent.rigid.angular_velocity * fragment.inherit_velocity,
        );
        let visual = parent.transform(1.0);
        let owner: Weak<()> = model.owner.clone();
        if owner.strong_count() == 0 {
            return None;
        }
        let spec = match (fragment.no_physics, data.fragment_dynamics(fragment.attributes)) {
            (_, Some(spec)) => spec,
            (false, None) => return None,
            (true, None) => {
                // A visual model has no independent body attributes. Retain a frozen
                // pose carrier, cleaned up by its parent rather than prop physics.
                let mut carrier = self.colliders.get(&id)?.shape.dynamics.as_ref()?.as_ref().clone();
                carrier.event_sequencer = 0;
                carrier.kill_off_screen = 0.0;
                carrier.allow_off_world = true;
                Arc::new(carrier)
            }
        };
        let mut body = PropBody::fragment(&bounds, bounds_root, &spec, pose, visual)?;
        body.rigid.linear_velocity = velocity.0;
        body.rigid.angular_velocity = velocity.1;
        if fragment.no_physics {
            body.rigid.state = BodyState::Frozen;
        }
        let child_model = SceneryModel {
            parts: Some(parts),
            bounds,
            bounds_root,
            render_root,
            key: spec.event_sequencer,
            sequence: None,
            player: None,
            failed: false,
            hidden: false,
            authored_visual: false,
            hidden_parts: BTreeSet::new(),
            spawned: BTreeSet::new(),
            triggered: false,
            fired: false,
            owner: owner.clone(),
            parent_owner: None,
            intact: false,
        };
        let child = self.insert_fragment_with(body, spec, Vec::new(), owner, true)?;
        if fragment.no_physics {
            self.colliders.get_mut(&child)?.collidable = false;
            self.regrid(child);
        }
        self.scenery.models.insert(child, child_model);
        let model = self.scenery.models.get_mut(&id)?;
        model.spawned.insert(fragment.part);
        if fragment.hide_part {
            model.hidden_parts.insert(fragment.part);
        }
        if fragment.hide_parent {
            model.hidden = true;
        }
        log::debug!("scenery {id}: released part {:08X}, body {child}", fragment.part);
        Some(child)
    }
}
