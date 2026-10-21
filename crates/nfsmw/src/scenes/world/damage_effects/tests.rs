use super::*;
use glam::Mat4;
use nfsmw_data::car::damage_effects::Group;
use std::rc::Rc;

fn config() -> DamageFx {
    DamageFx {
        engine: Some(Mat4::from_translation(Vec3::Z)),
        damage: Group { key: 1, ..Default::default() },
        death: Group { key: 2, ..Default::default() },
    }
}

fn source<'a>(owner: &Rc<()>, fx: &'a DamageFx, health: f32) -> Source<'a> {
    Source {
        id: 7,
        owner: Rc::downgrade(owner),
        fx,
        health,
        frame: Frame { to_world: Mat4::from_translation(Vec3::Y * 10.0), ..Default::default() },
    }
}

#[test]
fn persistent_health_effects_change_groups_drain_and_isolate_reused_actor_ids() {
    let spec = EmitterSpec { rate: 60.0, life: 0.5, size: [0.3; 4], colors: [[255; 4]; 4], ..Default::default() };
    let mut bank = DamageEffects::default();
    bank.groups.insert(1, vec![(1, spec.clone())]);
    bank.groups.insert(2, vec![(2, spec)]);
    let fx = config();
    let owner = Rc::new(());
    bank.step([source(&owner, &fx, 0.5001)], 1.0 / 60.0);
    assert!(bank.runs.is_empty());
    for _ in 0..20 {
        bank.step([source(&owner, &fx, 0.5)], 1.0 / 60.0);
    }
    assert_eq!(bank.runs.len(), 1);
    assert!(bank.runs[0].emitters[0].1.len() >= 19, "continuous damage emission, not one collision pulse");
    assert_eq!(bank.runs[0].frame.to_world.w_axis.z, 1.0);
    bank.step([source(&owner, &fx, 0.0)], 1.0 / 60.0);
    assert!(bank.runs.iter().any(|run| run.group == 2 && run.active));
    assert!(bank.runs.iter().any(|run| run.group == 1 && !run.active));
    let replacement = Rc::new(());
    bank.step([source(&replacement, &fx, 0.0)], 1.0 / 60.0);
    assert_eq!(
        bank.runs.iter().filter(|run| run.group == 2).count(),
        2,
        "reusing an actor id cannot inherit the retired owner's emitter"
    );
    for _ in 0..60 {
        bank.step([source(&replacement, &fx, 1.0)], 1.0 / 60.0);
    }
    assert!(bank.runs.is_empty(), "repair stops emission and drains existing particles");
}

#[test]
#[ignore = "requires an installed game"]
fn installed_heat_one_health_groups_reach_shared_textured_batches_and_release() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let model = nfsmw_data::car::load_driving(
        &dir,
        "COPMIDSIZE",
        &nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None },
    )
    .unwrap();
    let fx = &model.driving_lods.damage_fx;
    assert!(fx.engine.is_some());
    assert!(fx.damage.key != 0 && fx.death.key != 0 && fx.damage.key != fx.death.key);
    assert!(!fx.damage.emitters.is_empty() && !fx.death.emitters.is_empty());
    let mut backend =
        crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native()).with_texture_tracking();
    let mut bank = DamageEffects::default();
    let owner = Rc::new(());
    for health in [0.5, 0.0] {
        bank.step([source(&owner, fx, health)], 1.0 / 60.0);
        bank.maintain(&mut backend, &dir);
        for _ in 0..120 {
            bank.step([source(&owner, fx, health)], 1.0 / 60.0);
        }
        let group = fx.select(health).unwrap().key;
        let active = bank.runs.iter().find(|run| run.group == group && run.active).unwrap();
        let count: usize = active.emitters.iter().map(|(_, e)| e.len()).sum();
        assert!(count > 0, "health {health} must spawn installed smoke/death particles");
        let mut batches = Vec::new();
        bank.textured(Vec3::new(-10.0, -10.0, 5.0), Vec3::Y, &mut batches);
        assert!(!batches.is_empty() && batches.iter().any(|batch| !batch.vertices.is_empty()));
        eprintln!("COPMIDSIZE health {health}: group {group:08X}, {count} particles, {} draw batches", batches.len());
    }
    for _ in 0..3600 {
        bank.step([], 1.0 / 60.0);
    }
    assert!(bank.runs.is_empty(), "retired car emission drains");
    bank.release(&mut backend);
    assert_eq!(backend.resources.lock().unwrap().textures, 0);
}
