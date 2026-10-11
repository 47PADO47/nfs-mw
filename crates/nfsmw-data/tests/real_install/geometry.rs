//! Check vehicle hierarchy and tractor/trailer anchors against the user's install.

use blackbox_attrib::vlt_hash;
use blackbox_collision::bounds_flags;
use glam::Vec3;
use nfsmw_data::car::physics::PhysicsData;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn vehicle_geometry_retains_the_original_hierarchy_and_joint_anchors() {
    let Some(dir) = crate::install() else { return };
    let data = PhysicsData::load(&dir).unwrap();
    let tractor = data.car("semilog").unwrap().geometry.unwrap();
    let trailer = data.car("trailerlog").unwrap().geometry.unwrap();
    let hitch = vlt_hash("5THWHEEL");
    assert_eq!(tractor.bounds.name_hash, vlt_hash("SEMI"), "variant inherits the complete model bounds");
    assert_eq!(tractor.joint(hitch, false), Some(Vec3::new(0.0, -0.403, -2.928)));
    assert_eq!(trailer.joint(hitch, true), Some(Vec3::new(0.015, -1.168, 4.483)));
    assert!(tractor.joint(hitch, true).is_none());
    assert_eq!(tractor.body.primitives.len(), 4);
    assert_eq!(trailer.body.primitives.len(), 8);
    assert!(!tractor.body.ground_meshes.is_empty());
    assert!(!trailer.body.ground_meshes.is_empty());
    for n in 1..=7 {
        let attachment = vlt_hash(&format!("ATTACHMENT{n}"));
        assert!(trailer.bounds.nodes.iter().any(|b| b.name_hash == attachment));
        assert!(!trailer.body.primitives.iter().any(|b| b.name == attachment), "cargo subtree is a separate body");
    }
    assert!(trailer.body.primitives.iter().all(|p| p.collides(bounds_flags::PRIM_VS_OBJECTS)));
    for model in ["semia", "semib", "semicmt", "semicon", "semicrate", "semilog"] {
        let a = data.car(model).unwrap().geometry.unwrap();
        let b = data.car(&data.trailer_of(model).unwrap()).unwrap().geometry.unwrap();
        assert!(a.joint(hitch, false).is_some() && b.joint(hitch, true).is_some(), "{model}");
    }
}
