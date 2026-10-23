//! Exercise the real world selection path, including the viewer without a player.

use super::super::{Options, WorldScene, resident::Placed};
use super::*;
use blackbox_gfx::{InstanceKey, MeshHandle};
use blackbox_scene::Aabb;
use glam::{Mat4, Vec3};

fn placed(mesh: MeshHandle) -> Placed {
    Placed {
        lods: [Some(mesh); 4],
        detailed: None,
        transform: Mat4::IDENTITY,
        bounds: Aabb::new([-2.0; 3], [2.0; 3]),
        radius: 2.0,
        flags: 0,
        group: 0,
        prop: None,
        prop_id: 0,
        key: InstanceKey::new(1, 1),
        original_shadow: false,
        casts_sun_shadow: false,
        spawner: None,
        parts: None,
        signal: None,
    }
}

#[test]
#[ignore = "requires installed game metadata to construct WorldScene"]
fn installed_world_viewer_selects_multiple_lamps_without_player_and_obeys_visibility() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let mut world = WorldScene::open(
        &dir,
        Options {
            at: Some([0.0, 0.0]),
            height: 0.0,
            heading: 180.0,
            pitch: 0.0,
            fog_distance: 1000.0,
            wait_for_load: false,
            drive: None,
        },
    )
    .unwrap();
    assert!(world.players.lead_mut().is_none());
    world.traffic = None;
    world.camera.position = Vec3::new(10.0, 0.0, 0.0);
    world.camera.yaw = std::f32::consts::PI;
    world.camera.pitch = 0.0;
    let (lights, textures, _) = super::tests::fixture();
    let mesh = MeshHandle::from_raw(1);
    world.environment_lights = lights;
    world.residency.shared.materials = textures;
    world.residency.shared.flare_markers.insert(
        mesh,
        vec![
            Marker { position: Vec3::X, kind: Kind::AmberBlink },
            Marker { position: Vec3::new(1.0, 0.4, 0.0), kind: Kind::AmberBlink },
        ]
        .into(),
    );
    world.residency.shared_placed = vec![placed(mesh)];
    let renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
    let count =
        |world: &mut WorldScene| world.environment_effects(&renderer).iter().map(|e| e.vertices.len()).sum::<usize>();
    assert_eq!(count(&mut world), 12, "both authored lamps reach the world viewer without a player");
    world.environment_lights.blink.advance(0.6);
    assert_eq!(count(&mut world), 0, "world blink state gates the output");
    world.environment_lights.blink.advance(0.6);
    assert_eq!(count(&mut world), 12);

    world.residency.shared_placed[0].group = 7;
    assert_eq!(count(&mut world), 0, "disabled scenery groups cannot leave ghost lights");
    world.residency.set_collision_group(7, true);
    world.scenery_groups.set(7, true);
    assert_eq!(count(&mut world), 12);
    world.residency.shared_placed[0].prop_id = 91;
    world.residency.world_parts().1.knock(91);
    assert_eq!(count(&mut world), 0, "the real prop visibility path suppresses hidden lamps");
    world.residency.shared_placed[0].prop_id = 0;
    world.residency.shared_placed[0].lods = [Some(MeshHandle::from_raw(2)); 4];
    assert_eq!(count(&mut world), 0, "an unmarked selected model inherits no lamp");
    world.residency.shared_placed[0].lods = [Some(mesh); 4];
    world.camera.yaw = 0.0;
    assert_eq!(count(&mut world), 0, "main-view frustum selection removes lamps behind the camera");
    world.camera.yaw = std::f32::consts::PI;
    world.residency.shared_placed.clear();
    assert_eq!(count(&mut world), 0, "removing the placement removes every lamp");
}
