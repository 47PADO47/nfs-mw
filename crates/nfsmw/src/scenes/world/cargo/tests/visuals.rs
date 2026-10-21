//! Actual uploaded cargo retains its mesh, material and world pose at detachment.

use super::*;
use crate::app::test_backend::NullBackend;
use crate::scenes::car::materials::CarMaterials;
use crate::scenes::world::sun;
use crate::settings::CarShading;
use blackbox_gfx::{Capabilities, GraphicsApi, Instance};
use blackbox_scene::MaterialLookup;

fn close(a: glam::Mat4, b: glam::Mat4) -> bool {
    a.to_cols_array().into_iter().zip(b.to_cols_array()).all(|(a, b)| (a - b).abs() < 0.0001)
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_cargo_release_preserves_uploaded_parts_and_world_pose() {
    let (dir, data) = installed().expect("set NFSMW_GAME_DIR to run installed tests");
    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    for (name, expected) in [("TRAILERLOG", 7), ("TRAILERB", 3)] {
        let model = nfsmw_data::car::load(&dir, name, &options).unwrap();
        let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan))
            .with_mesh_tracking()
            .with_glossy_tracking();
        let materials = CarMaterials::upload(&mut renderer, &model, CarShading::default(), sun::to_sun());
        for placement in
            model.placements.iter().filter(|p| nfsmw_data::car::LAYOUT.slots[p.slot].name.starts_with("ATTACHMENT"))
        {
            let solid = &model.solids[&placement.solid];
            let lookup = materials.for_placement(&model.swaps, placement.left_brake);
            for group in &solid.groups {
                let hash = group.diffuse_texture(solid).expect("stock cargo has a diffuse reference");
                assert!(lookup.material(hash).is_some(), "{name}: released {} must retain its texture", solid.name);
            }
        }
        materials.destroy(&mut renderer);
        let rig = CarRig::upload(&mut renderer, 123, model, CarShading::default(), sun::to_sun());
        let mut sim = CarSim::new(data.car(name).unwrap(), rig.rest_heights());
        sim.configure_trailer();
        sim.body_mut().place(Vec3::Y * 4.0, Quat::IDENTITY);
        rear_hit(&mut sim);
        let mut props = PropWorld::default();
        let parent = sim.pose();
        let mut before = Vec::new();
        rig.instances(&parent, None, &Default::default(), &mut before);
        // Hold the parent pose at the instant of release. This checks visual handover before integration.
        for _ in 0..240 {
            update(&mut sim, &rig, &mut props, &data, parent.position);
            let hidden = &sim.events.as_ref().unwrap().hidden_slots;
            let expected_parts: Vec<Instance> = hidden
                .iter()
                .flat_map(|&slot| rig.fragment_instances(slot, None))
                .map(|i| Instance::new(i.mesh, parent.transform() * i.transform))
                .collect();
            let mut detached = Vec::new();
            props.fragment_instances(&mut detached);
            assert_eq!(detached.len(), expected_parts.len(), "{name}: released slots must all draw");
            let mut unmatched = expected_parts;
            for part in detached {
                let index = unmatched
                    .iter()
                    .position(|expected| expected.mesh == part.mesh && close(expected.transform, part.transform))
                    .expect("detached mesh must start at the same world pose as its parent part");
                unmatched.swap_remove(index);
            }
            let mut attached = Vec::new();
            rig.instances(&parent, Some(hidden), &Default::default(), &mut attached);
            assert_eq!(attached.len() + hidden.len(), before.len(), "released parts are hidden exactly once");
        }
        assert_eq!(props.fragment_count(), expected, "{name}: actual cargo sequence must release the load");
        let mut detached = Vec::new();
        props.fragment_instances(&mut detached);
        assert_eq!(detached.len(), expected);
        let before_reshade = detached.clone();
        let replacements = rig.reshade(&mut renderer, CarShading::Original, sun::to_sun());
        props.remap_fragment_meshes(&replacements);
        detached.clear();
        props.fragment_instances(&mut detached);
        assert_eq!(detached.len(), before_reshade.len());
        for old in &before_reshade {
            let new_mesh =
                replacements.iter().find(|(mesh, _)| *mesh == old.mesh).expect("every released mesh is remapped").1;
            assert!(
                detached.iter().any(|part| part.mesh == new_mesh && close(part.transform, old.transform)),
                "reshading preserves released cargo pose"
            );
        }
        eprintln!("{name}: {expected} textured cargo meshes preserve their world pose at release");
        drop(sim);
        detached.clear();
        props.fragment_instances(&mut detached);
        assert!(detached.is_empty(), "owner destruction must remove cargo visuals");
        rig.release(&mut renderer);
    }
}
