use super::*;
use blackbox_attrib::{Database, vlt_hash};
use std::rc::Rc;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_prop_particle_wrappers_and_atlas_frames_reach_draw_batches_and_release_textures() {
    let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(path).unwrap();
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let mut fx = ObjectFx::default();
    let mut renderer =
        crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native()).with_texture_tracking();
    let owner = Rc::new(());
    let mut animated = 0;
    let mut static_cells = 0;
    let mut nonidentity = 0;
    let mut cases = 0;
    for name in [
        "wooddust1",
        "sprkdir_lg1",
        "sprktrail1",
        "glasstrail1",
        "exp_lg1",
        "smoketrail1",
        "smoketrail2",
        "firetrail1",
        "carglass",
        "waterbarrel_sm1",
        "dustfall_md1",
        "hitcoin1",
    ] {
        let key = vlt_hash(name);
        let parameters = nfsmw_data::world::object_fx::parameters(&db, key).unwrap();
        nonidentity += usize::from(parameters.quadratic != [0.0, 1.0, 0.0, 0.0]);
        let emitters = nfsmw_data::world::object_fx::emitters(&db, key);
        assert!(!emitters.is_empty(), "{name}");
        for emitter in emitters {
            let animation = emitter.spec.animation;
            animated += usize::from(animation.grid > 0 && animation.fps > 0);
            static_cells += usize::from(animation.grid > 0 && animation.fps == 0 && animation.random_start);
            assert!(emitter.spec.intensity_range[0] < emitter.spec.intensity_range[1]);
        }
        fx.request([Command::Play(Source {
            id: cases,
            owner: Rc::downgrade(&owner),
            effect: ObjectEffect {
                name_tag: 0,
                effect: key,
                position: 0,
                intensity: 1.0,
                instance: 0,
                tracking: false,
                allow_hidden_spawn: false,
            },
            frame: Frame { to_world: glam::Mat4::from_translation(Vec3::Y * 10.0), ..Default::default() },
        })]);
        cases += 1;
    }
    fx.maintain(&mut renderer, &dir, &db);
    assert_eq!(fx.started, cases as u64);
    assert!(animated > 0 && static_cells > 0 && nonidentity > 0);
    let mut cropped = false;
    let mut vertices = 0;
    for _ in 0..30 {
        fx.step(&super::super::props::PropWorld::default(), 1.0 / 60.0);
        let mut batches = Vec::new();
        fx.textured(Vec3::ZERO, Vec3::Y, &mut batches);
        for batch in batches {
            vertices += batch.vertices.len();
            for quad in batch.vertices.as_chunks::<6>().0 {
                let width = quad[1].uv[0] - quad[0].uv[0];
                cropped |= width > 0.0 && width < 0.99;
                assert!(quad.iter().all(|v| v.uv.iter().all(|u| u.is_finite() && (0.0..=1.0).contains(u))));
            }
        }
    }
    assert!(vertices > 0 && cropped, "real atlas emitters must crop their texture in the draw batch");
    assert!(renderer.resources.lock().unwrap().textures > 0);
    fx.release(&mut renderer);
    assert_eq!(renderer.resources.lock().unwrap().textures, 0);
    eprintln!(
        "{cases} groups: {animated} animated emitters, {static_cells} random static atlases, {nonidentity} wrapper curves; {vertices} vertices"
    );
}
