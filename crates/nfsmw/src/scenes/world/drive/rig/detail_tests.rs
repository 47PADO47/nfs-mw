use super::*;
use crate::scenes::world::drive::{DetailView, DrawOptions};
use crate::{
    app::test_backend::NullBackend,
    settings::{CarGeometry, DetailSettings},
};
use blackbox_scenery::LodView;

#[test]
#[ignore = "requires an installed game"]
fn installed_car_detail_switches_cached_meshes_without_uploads_or_actor_mutation() {
    let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut changed = 0;
    for name in ["ELISE", "BMWM3GTR", "COPMIDSIZE", "COPGTO", "SEMI", "TRAILERB"] {
        let model = nfsmw_data::car::load_driving(
            &dir,
            name,
            &nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None },
        )
        .unwrap();
        assert!(!model.placements.is_empty());
        let floor = model.floor_height;
        let canonical = model.placements.clone();
        let mut backend = NullBackend::new(crate::settings::test_caps::native())
            .with_mesh_tracking()
            .with_texture_tracking()
            .with_glossy_tracking();
        let rig = CarRig::upload(&mut backend, keys::next_car(), model, CarShading::Original, Vec3::Z);
        let before = {
            let r = backend.resources.lock().unwrap();
            (r.meshes, r.textures)
        };
        let pose = CarPose { position: Vec3::ZERO, rotation: Quat::IDENTITY, wheels: [WheelPose::default(); 4] };
        let near = DetailView {
            view: LodView { position: [0.0, -5.0, 0.0], forward: [0.0, 1.0, 0.0], pixel_scale: 343.0 },
            settings: DetailSettings { car: CarGeometry::High, ..Default::default() },
        };
        let far = DetailView { view: LodView { position: [0.0, -100.0, 0.0], ..near.view }, ..near };
        let mut full = Vec::new();
        rig.instances(&pose, None, &Default::default(), &mut full);
        let mut distant = Vec::new();
        rig.instances_detail(
            &pose,
            None,
            &Default::default(),
            DrawOptions { car: rig.car, detail: Some(&far), ..Default::default() },
            &mut distant,
        );
        assert!(!distant.is_empty(), "{name} retains usable body geometry");
        changed += usize::from(
            distant.iter().map(|i| i.mesh).collect::<Vec<_>>() != full.iter().map(|i| i.mesh).collect::<Vec<_>>(),
        );
        for view in [near, far, near, far] {
            let mut out = Vec::new();
            rig.instances_detail(
                &pose,
                None,
                &Default::default(),
                DrawOptions { car: rig.car, detail: Some(&view), ..Default::default() },
                &mut out,
            );
            let mut keys: Vec<_> = out.iter().map(|i| i.key).collect();
            keys.sort();
            keys.dedup();
            assert_eq!(keys.len(), out.len(), "{name} needs one stable identity per part");
            let hidden: std::collections::BTreeSet<_> = rig.model.placements.iter().map(|p| p.slot).collect();
            let mut removed = Vec::new();
            rig.instances_detail(
                &pose,
                Some(&hidden),
                &Default::default(),
                DrawOptions { car: rig.car, detail: Some(&view), ..Default::default() },
                &mut removed,
            );
            assert!(removed.is_empty(), "hidden canonical slots stay hidden at every level");
        }
        let mut independent = Vec::new();
        rig.instances(&pose, None, &Default::default(), &mut independent);
        let geometry = |list: &[Instance]| list.iter().map(|i| (i.mesh, i.key, i.transform)).collect::<Vec<_>>();
        assert_eq!(
            geometry(&full),
            geometry(&independent),
            "main-view detail must not leak into shadow/reflection submissions"
        );
        assert_eq!(rig.model.floor_height, floor);
        assert_eq!(rig.model.placements, canonical);
        assert_eq!(
            {
                let r = backend.resources.lock().unwrap();
                (r.meshes, r.textures)
            },
            before
        );
        rig.release(&mut backend);
        let counts = backend.resources.lock().unwrap();
        assert_eq!((counts.meshes, counts.textures), (0, 0));
    }
    assert!(changed >= 3, "fixtures must exercise actual lower meshes");
}
