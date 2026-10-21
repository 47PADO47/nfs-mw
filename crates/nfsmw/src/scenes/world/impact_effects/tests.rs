use super::*;
use blackbox_attrib::RefSpec;
use std::rc::Rc;

pub(super) fn installed() -> (GameDir, Database) {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    (dir, db)
}
pub(super) fn sample(db: &Database, class: &str, name: &str, action: Action, expected: &str, life: &Rc<()>) -> Contact {
    let source = db.collection(class, name).unwrap();
    let profile = SourceEffects::read(db, source);
    let (surface, link) = db
        .collections_of("simsurface")
        .filter_map(|surface| profile.pick(action, surface.key()).map(|link| (surface.key(), link)))
        .find(|(_, link)| link.effect == vlt_hash(expected))
        .unwrap_or_else(|| panic!("{name} {action:?} must select {expected}"));
    let particles = link.particles.expect("accepted installed group must have supported native PC profiles");
    assert!(particles.emitters.iter().flatten().next().is_some());
    let kind = match action {
        Action::HitObject => Kind::Object,
        Action::HitGround => Kind::Ground,
        _ => Kind::World,
    };
    let mut raw =
        RawContact::new(kind, surface, Vec3::ZERO, Vec3::Y, -Vec3::Y * link.max.max(40.0), link.max, Vec3::ZERO);
    raw.detached = action == Action::Detached;
    Contact {
        source: RefSpec { class: vlt_hash(class), collection: source.key() },
        owner: next_owner(),
        lifetime: Rc::downgrade(life),
        raw,
    }
}

#[test]
fn raw_callbacks_keep_normal_closing_tangential_sliding_and_owner_velocity_separate() {
    let raw = RawContact::new(Kind::Object, 7, Vec3::ZERO, Vec3::X, Vec3::new(-2.0, 30.0, 0.0), 5.0, Vec3::Z * 12.0);
    assert_eq!(raw.closing, -Vec3::X * 2.0);
    assert_eq!(raw.sliding, Vec3::Y * 30.0);
    assert_eq!(raw.owner_velocity, Vec3::Z * 12.0);
    assert!(next_owner() != next_owner());
}

#[test]
fn source_contact_timeout_and_retired_owners_use_production_step_cleanup() {
    let mut fx = ImpactEffects::default();
    let db = Database::default();
    let owner = Rc::new(());
    let id = next_owner();
    fx.banks.insert(id, Bank { particles: PcSparks::default(), owner: Rc::downgrade(&owner), timeout: 0.2 });
    for _ in 0..11 {
        fx.step(&db, [], Vec3::ZERO, 1.0 / 60.0);
    }
    assert!(fx.banks.contains_key(&id), "the original .2 second service window is still open");
    for _ in 0..2 {
        fx.step(&db, [], Vec3::ZERO, 1.0 / 60.0);
    }
    assert!(fx.banks.is_empty(), "production step expires the scrape service window");
    fx.banks.insert(id, Bank { particles: PcSparks::default(), owner: Rc::downgrade(&owner), timeout: 0.2 });
    drop(owner);
    fx.step(&db, [], Vec3::ZERO, 1.0 / 60.0);
    assert!(fx.banks.is_empty(), "retired owners stop their persistent clocks immediately");
    assert_ne!(id, next_owner());
}

#[test]
#[ignore = "requires an installed game"]
fn installed_source_contact_groups_select_native_profiles_and_reach_shared_batches() {
    let (dir, db) = installed();
    let owner = Rc::new(());
    let mut backend =
        crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native()).with_texture_tracking();
    for (class, name, action, effect) in [
        ("smackable", "largemetalpost", Action::HitWorld, "lamp_lrg_hit_world"),
        ("smackable", "largemetalpost", Action::HitObject, "lamp_lrg_hit_car"),
        ("pvehicle", "traftaxi", Action::HitWorld, "carhitwall"),
        ("pvehicle", "copmidsize", Action::HitObject, "carhitcar"),
        ("smackable", "genericwood_frag", Action::HitWorld, "wood_frag_med_hit_world"),
        ("smackable", "glassshatter1", Action::HitGround, "glass_hit_world"),
        ("smackable", "trafficcablelight", Action::HitWorld, "mtl_box_hit_world"),
    ] {
        let contact = sample(&db, class, name, action, effect, &owner);
        let mut fx = ImpactEffects::default();
        fx.step(&db, [contact], Vec3::new(3.0, 3.0, 2.0), 1.0 / 60.0);
        fx.maintain(&mut backend, &dir);
        for _ in 0..5 {
            fx.step(&db, [], Vec3::ZERO, 1.0 / 60.0);
        }
        let mut batches = Vec::new();
        fx.textured(Vec3::Y, &mut batches);
        assert!(
            batches.iter().any(|batch| !batch.vertices.is_empty()),
            "{name} {effect} must reach shared textured batches"
        );
        assert!(fx.accepted > 0);
        eprintln!("{name} {effect}: {} native batches", batches.len());
        for _ in 0..3600 {
            fx.step(&db, [], Vec3::ZERO, 1.0 / 60.0);
        }
        assert!(fx.banks.is_empty(), "native clocks and particles must drain");
        fx.release(&mut backend);
    }
}

#[test]
#[ignore = "requires an installed game"]
fn installed_source_contact_scrape_times_out_and_retires_through_production_step() {
    let (_, db) = installed();
    let owner = Rc::new(());
    let source = db.collection("pvehicle", "copmidsize").unwrap();
    let profile = SourceEffects::read(&db, source);
    let (surface, link) = db
        .collections_of("simsurface")
        .filter_map(|surface| {
            profile
                .pick(Action::ScrapeWorld, surface.key())
                .filter(|link| link.particles.is_some())
                .map(|link| (surface.key(), link))
        })
        .next()
        .expect("cop world scrape native group");
    let mut raw = RawContact::new(Kind::World, surface, Vec3::ZERO, Vec3::Y, Vec3::X * link.max, 0.0, Vec3::ZERO);
    raw.closing = Vec3::ZERO;
    let contact = Contact {
        source: RefSpec { class: vlt_hash("pvehicle"), collection: source.key() },
        owner: next_owner(),
        lifetime: Rc::downgrade(&owner),
        raw,
    };
    let id = contact.owner;
    let mut fx = ImpactEffects::default();
    fx.step(&db, [contact.clone()], Vec3::ZERO, 1.0 / 60.0);
    assert!(fx.banks[&id].particles.pending());
    for _ in 0..11 {
        fx.step(&db, [], Vec3::ZERO, 1.0 / 60.0);
    }
    assert!(fx.banks[&id].particles.pending());
    for _ in 0..2 {
        fx.step(&db, [], Vec3::ZERO, 1.0 / 60.0);
    }
    assert!(!fx.banks[&id].particles.pending(), "timeout stops continuous sources while existing particles drain");
    fx.step(&db, [contact], Vec3::ZERO, 1.0 / 60.0);
    drop(owner);
    fx.step(&db, [], Vec3::ZERO, 1.0 / 60.0);
    assert!(!fx.banks[&id].particles.pending());
    for _ in 0..3600 {
        fx.step(&db, [], Vec3::ZERO, 1.0 / 60.0);
    }
    assert!(fx.banks.is_empty());
}

#[test]
fn source_contact_disabled_toggle_drops_old_banks_and_queued_emission() {
    let owner = Rc::new(());
    let mut fx = ImpactEffects::default();
    fx.banks.insert(next_owner(), Bank { particles: PcSparks::default(), owner: Rc::downgrade(&owner), timeout: 0.2 });
    fx.pending.insert(77);
    fx.set_enabled(false);
    fx.step(&Database::default(), [], Vec3::ZERO, 1.0 / 60.0);
    assert!(fx.banks.is_empty() && fx.pending.is_empty());
    fx.set_enabled(true);
    assert!(!fx.disabled);
}

#[test]
#[ignore = "requires an installed game"]
fn installed_source_contact_detachment_precedes_ordinary_hit_and_uses_shared_pair_normal() {
    let (_, db) = installed();
    let owner = Rc::new(());
    let mut contact = sample(&db, "smackable", "trafficlight", Action::Detached, "lamp_lrg_hit_car", &owner);
    contact.raw.detached_normal = Vec3::X;
    contact.raw.normal = -Vec3::X;
    let mut fx = ImpactEffects::default();
    fx.step(&db, [contact], Vec3::ZERO, 1.0 / 60.0);
    assert_eq!(fx.accepted, 1, "an accepted detach returns before the ordinary hit producer");
    assert_eq!(fx.banks.len(), 1);
}

#[test]
fn source_contact_ground_uses_solver_closing_with_retained_arm_and_keeps_sliding() {
    let contact = blackbox_vehicle::rigid_body::GroundContact {
        normal: Vec3::Y,
        velocity: Vec3::new(12.0, -30.0, 4.0),
        closing_speed: 2.5,
        impulse_delta_v: 3.0,
        local_arm: Vec3::new(0.7, -0.4, 1.2),
        ..Default::default()
    };
    let raw = RawContact::ground(&contact, Vec3::Z * 7.0);
    assert_eq!(raw.closing, Vec3::NEG_Y * 2.5, "use successful solver input rather than stale point projection");
    assert_eq!(raw.sliding, Vec3::new(12.0, 0.0, 4.0));
    assert_eq!(raw.delta_v, 3.0);
    assert_eq!(raw.owner_velocity, Vec3::Z * 7.0);
}
