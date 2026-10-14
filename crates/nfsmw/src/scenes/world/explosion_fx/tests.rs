use super::*;

fn spec() -> EmitterSpec {
    EmitterSpec { rate: 100.0, life: 0.5, speed: 2.0, size: [0.5; 4], ..Default::default() }
}

#[test]
fn a_blast_spawns_while_it_runs_and_is_spent_after_its_particles_die() {
    let templates = [(0, spec())];
    let mut blast = Blast::new(&templates, Vec3::ZERO, 1);
    blast.step(0.1);
    assert!(blast.emitters[0].len() > 0, "a running blast spawns particles");
    assert!(blast.alive());
    while blast.age < BLAST_SECONDS {
        blast.step(0.1);
    }
    // Spawning has stopped: after the particles' life the blast is spent.
    for _ in 0..20 {
        blast.step(0.1);
    }
    assert!(!blast.alive());
}

#[test]
fn requests_are_dropped_when_the_effect_has_no_art() {
    let mut fx = ExplosionFx { load: Load::Missing, ..Default::default() };
    fx.request([Vec3::ZERO, Vec3::X]);
    assert!(fx.queued.is_empty());
    fx.step(1.0 / 60.0);
    assert!(fx.blasts.is_empty());
}

#[test]
fn requests_wait_for_the_textures_to_load() {
    let mut fx = ExplosionFx::default();
    fx.request([Vec3::Z]);
    assert_eq!(fx.queued, vec![Vec3::Z], "no load yet: the request is kept");
}
