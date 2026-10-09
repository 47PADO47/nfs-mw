use super::*;
use nfsmw_data::vehicle_effects::EmitterStyle;

const STEP: f32 = 1.0 / 60.0;

fn link() -> SparkLink {
    SparkLink {
        min: 1.0,
        max: 10.0,
        styles: [Some(EmitterStyle { color: [1.0; 4], life: 0.4, life_variance: 0.1 }), None],
    }
}

fn contact(velocity: Vec3, impulse: f32) -> VisualContact {
    VisualContact {
        point: Vec3::new(2.0, 3.0, 4.0),
        normal: Vec3::NEG_Z,
        velocity,
        impulse_delta_v: impulse,
        surface: Some(7),
        kind: super::super::super::drive::ContactKind::Face,
        info: None,
        prop: None,
    }
}

#[test]
fn impact_strength_creates_one_burst_from_the_reacting_contact() {
    let c = contact(Vec3::Z * 10.0, 10.0);
    let mut particles = Sparks::default();
    particles.emit_selected(&[c, c, c], |_, hit| hit.then(link), STEP);
    assert_eq!(particles.len(), 48, "duplicate probes must not multiply a burst");
    assert!(particles.particles.iter().all(|p| p.point.abs_diff_eq(Vec3::new(3.97, -2.0, 3.0), 1e-5)));
    particles.emit_selected(&[contact(Vec3::ZERO, 10.0)], |_, _| Some(link()), STEP);
    assert_eq!(particles.emitted, 48, "stationary correction is not an impact");
}

#[test]
fn scrape_needs_contact_motion_and_never_uses_the_hit_impulse_as_speed() {
    let mut particles = Sparks::default();
    for _ in 0..60 {
        particles.emit_selected(&[contact(Vec3::X * 10.0, 0.0)], |_, hit| (!hit).then(link), STEP);
    }
    assert_eq!(particles.emitted, 160);
    for quiet in [Vec3::ZERO, Vec3::X * 0.5, Vec3::NEG_Z * 20.0] {
        particles.emit_selected(&[contact(quiet, 100.0)], |_, _| Some(link()), STEP);
    }
    assert_eq!(particles.emitted, 160, "static, slow and separating contacts are quiet");
}

#[test]
fn missing_material_unsupported_effect_and_invalid_inputs_emit_nothing() {
    let c = contact(Vec3::Z * 10.0, 10.0);
    for invalid in [
        VisualContact { surface: None, ..c },
        VisualContact { point: Vec3::NAN, ..c },
        VisualContact { velocity: Vec3::NAN, ..c },
        VisualContact { normal: Vec3::ZERO, ..c },
        VisualContact { impulse_delta_v: f32::NAN, ..c },
    ] {
        let mut particles = Sparks::default();
        particles.emit_selected(&[invalid], |_, _| Some(link()), STEP);
        assert_eq!(particles.len(), 0);
    }
    let mut particles = Sparks::default();
    particles.emit_selected(&[c], |_, _| None, STEP);
    particles.emit_selected(&[c], |_, _| Some(SparkLink { max: f32::NAN, ..link() }), STEP);
    assert_eq!(particles.len(), 0);
}

#[test]
fn long_scrapes_are_bounded_age_out_and_have_no_lost_contact_backlog() {
    let c = contact(Vec3::X * 10.0, 0.0);
    let mut particles = Sparks::default();
    for _ in 0..1000 {
        particles.emit_selected(&[c], |_, _| Some(link()), 10.0);
    }
    assert_eq!(particles.len(), MAX_SPARKS);
    particles.age(0.6);
    assert_eq!(particles.len(), 0);
    particles.emit_selected(&[], |_, _| Some(link()), STEP);
    particles.emit_selected(&[c], |_, _| Some(link()), STEP);
    assert!(particles.len() < 3);
    particles.clear();
    assert_eq!(particles.len(), 0);
}

#[test]
fn fixed_input_produces_identical_finite_geometry_and_fades() {
    let run = || {
        let mut particles = Sparks::default();
        particles.emit_selected(&[contact(Vec3::Z * 10.0, 10.0)], |_, _| Some(link()), STEP);
        particles.age(0.1);
        let mut geometry = Vec::new();
        particles.geometry(Vec3::new(-5.0, 0.0, 8.0), Vec3::X, &mut geometry);
        geometry
    };
    let vertices = run();
    assert_eq!(vertices, run());
    assert_eq!(vertices.len(), 48 * 6);
    assert!(vertices.iter().all(|v| Vec3::from(v.position).is_finite() && v.color[3] < 255));
}
