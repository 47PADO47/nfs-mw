//! A car's physics data reads into the vehicle library's parameter structs with sane values.

use blackbox_attrib::Database;
use nfsmw_data::car::physics::{self, Fields, SurfaceTable};

use crate::install;

fn attributes() -> Option<Database> {
    let dir = install()?;
    Some(Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).expect("attributes.bin"))
}

fn link<'a>(db: &'a Database, car: &str, field: &str) -> Fields<'a> {
    let pvehicle = db.collection("pvehicle", car).unwrap_or_else(|| panic!("no pvehicle {car}"));
    Fields(pvehicle.follow(field).unwrap_or_else(|| panic!("{car}: no {field} link")))
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_m3_gtr_powertrain_matches_the_data() {
    let Some(db) = attributes() else { return };
    let engine = physics::engine(link(&db, "bmwm3gtr", "engine"));
    assert_eq!(engine.torque.len(), 9);
    assert_eq!((engine.idle, engine.red_line, engine.max_rpm), (800.0, 8500.0, 9500.0));
    assert_eq!(engine.torque[4], 283.0);
    assert_eq!(engine.speed_limiter, [0.0, 0.0], "no governor");
    let t = physics::transmission(link(&db, "bmwm3gtr", "transmission"));
    assert_eq!(t.gear_ratio.len(), 8, "reverse, neutral and six forward gears");
    assert_eq!(t.gear_ratio[1], 0.0);
    assert_eq!(t.final_gear, 3.85);
    assert!((0.0..=1.0).contains(&t.torque_split));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_car_has_a_sane_engine_and_gearbox() {
    let Some(db) = attributes() else { return };
    let mut checked = 0;
    for car in db.collections_of("pvehicle") {
        let Some(name) = car.name() else { continue };
        let (Some(engine), Some(trans)) = (car.follow("engine"), car.follow("transmission")) else { continue };
        let (engine, trans) = (physics::engine(Fields(engine)), physics::transmission(Fields(trans)));
        if engine.torque.len() < 2 {
            continue; // not a real car (a template or a prop)
        }
        assert!(engine.idle > 300.0 && engine.idle < engine.red_line, "{name}: idle {}", engine.idle);
        assert!((4000.0..=14000.0).contains(&engine.red_line), "{name}: redline {}", engine.red_line);
        assert!(engine.torque.iter().all(|&t| t > 0.0 && t < 2000.0), "{name}: torque {:?}", engine.torque);
        let forward = trans.gear_ratio.iter().skip(2).filter(|&&r| r > 0.0).count();
        assert!((3..=7).contains(&forward), "{name}: {forward} forward gears");
        assert!(trans.final_gear > 1.0 && trans.final_gear < 8.0, "{name}: final {}", trans.final_gear);
        checked += 1;
    }
    assert!(checked >= 50, "only {checked} cars checked");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn road_surfaces_have_grip() {
    let Some(db) = attributes() else { return };
    let table = SurfaceTable::from_database(&db);
    assert_eq!(table.len(), 47);
    let asphalt = table.grip(SurfaceTable::hash_of("asphalt"));
    assert!(asphalt.lateral > 0.5 && asphalt.drive > 0.5, "{asphalt:?}");
    let grass = table.grip(SurfaceTable::hash_of("grass"));
    assert!(
        grass.lateral < asphalt.lateral && grass.rolling > asphalt.rolling,
        "grass {grass:?} vs asphalt {asphalt:?}"
    );
    assert!(!table.knows(0), "no surface is hash 0: it reads as `unknown`");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn car_bounds_are_a_car_sized_box() {
    let Some(dir) = install() else { return };
    let sets = physics::read_car_bounds(&dir).unwrap();
    // Edition and replacement assets vary in count; require complete, distinct bounds trees.
    assert!(sets.len() >= 50, "only {} car bounds sets", sets.len());
    let names: std::collections::HashSet<_> = sets.iter().map(|s| s.name_hash).collect();
    assert_eq!(names.len(), sets.len(), "duplicate car names");
    assert!(sets.iter().all(|s| !s.nodes.is_empty() && s.is_tree()), "incomplete bounds tree");
    // docs/formats/collision.md: the M3 GTR's root box.
    let m3 = physics::car_bounds(&sets, "BMWM3GTR").unwrap();
    assert!((m3.half_dimensions - glam::Vec3::new(0.938, 0.621, 2.271)).abs().max_element() < 1e-3, "{m3:?}");
    for car in ["PORSCHE911", "CORVETTE", "CAMARO", "LANDROVER"] {
        let Ok(b) = physics::car_bounds(&sets, car) else { continue };
        let h = b.half_dimensions;
        assert!((0.7..1.3).contains(&h.x) && (0.4..1.2).contains(&h.y) && (1.9..3.4).contains(&h.z), "{car}: {h}");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_m3_gtr_brakes_and_tires_match_the_data() {
    let Some(db) = attributes() else { return };
    let tires = physics::tires(link(&db, "bmwm3gtr", "tires"));
    assert_eq!(tires.rim_size, [19.0, 19.0], "docs/formats/attributes.md");
    // 19 inch rims with a low sidewall: a 0.32 to 0.36 m rolling radius.
    for axle in 0..2 {
        let r = tires.radius(axle);
        assert!((0.30..0.38).contains(&r), "axle {axle}: radius {r}");
        assert!(tires.static_grip[axle] > 0.5 && tires.static_grip[axle] < 2.5, "{:?}", tires.static_grip);
        assert!(tires.dynamic_grip[axle] <= tires.static_grip[axle] + 1e-6);
    }
    let brakes = physics::brakes(link(&db, "bmwm3gtr", "brakes"));
    assert!(brakes.brakes.iter().all(|&b| b > 100.0), "{brakes:?}");
    assert!(brakes.ebrake > 0.0);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_m3_gtr_body_has_its_mass_and_a_normal_gravity() {
    let Some(db) = attributes() else { return };
    let pvehicle = Fields(db.collection("pvehicle", "bmwm3gtr").unwrap());
    let body = physics::body(pvehicle);
    assert_eq!(body.mass, 1350.0, "docs/formats/attributes.md");
    assert_eq!(body.tensor_scale, glam::Vec3::new(1.0, 2.0, 1.0));
    assert!((-10.0..-9.5).contains(&body.spec.gravity), "gravity {}", body.spec.gravity);
    assert!(
        body.spec.ground_friction[0] > 0.0 && body.spec.ground_friction[0] <= 1.5,
        "{:?}",
        body.spec.ground_friction
    );
    assert!(body.spec.sleep_velocity > 0.0);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_car_has_a_plausible_mass() {
    let Some(db) = attributes() else { return };
    let mut checked = 0;
    for car in db.collections_of("pvehicle") {
        let Some(name) = car.name() else { continue };
        if car.follow("engine").is_none() || physics::engine(Fields(car.follow("engine").unwrap())).torque.len() < 2 {
            continue;
        }
        let body = physics::body(Fields(car));
        assert!((700.0..12000.0).contains(&body.mass), "{name}: {} kg", body.mass);
        checked += 1;
    }
    assert!(checked >= 50);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_m3_gtr_chassis_is_a_sports_car_chassis() {
    let Some(db) = attributes() else { return };
    let c = physics::chassis(link(&db, "bmwm3gtr", "chassis"));
    // docs/formats/attributes.md: wheel base 2.725 m, 54 % of the weight on the front axle.
    assert_eq!((c.wheel_base, c.front_weight_bias), (2.725, 54.0));
    for axle in 0..2 {
        assert!((100.0..2000.0).contains(&c.spring_stiffness[axle]), "spring {:?}", c.spring_stiffness);
        assert!(c.shock_stiffness[axle] > 0.0 && c.shock_ext_stiffness[axle] > 0.0, "{c:?}");
        assert!((1.2..2.2).contains(&c.track_width[axle]), "track {:?}", c.track_width);
        assert!((1.0..12.0).contains(&c.ride_height[axle]), "ride {:?}", c.ride_height);
        assert!((1.0..12.0).contains(&c.travel[axle]), "travel {:?}", c.travel);
    }
    assert!((0.0..2.0).contains(&c.front_axle) && c.front_axle < c.wheel_base);
    let aero = physics::aero(link(&db, "bmwm3gtr", "chassis"));
    assert!(aero.drag_coefficient > 0.0, "{aero:?}");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn cars_assemble_into_vehicle_specs() {
    let Some(dir) = install() else { return };
    let data = physics::PhysicsData::load(&dir).unwrap();
    for car in ["BMWM3GTR", "PORSCHE911", "CORVETTE", "CAMARO", "MUSTANGGT"] {
        let Ok(p) = data.car(car) else { continue };
        let s = &p.spec;
        assert!((900.0..2500.0).contains(&s.mass), "{car}: {} kg", s.mass);
        assert!(s.engine.red_line > 5000.0 && s.engine.torque.len() >= 2, "{car}");
        assert!(s.transmission.top_gear() >= 5, "{car}: {} gear ids", s.transmission.gear_ratio.len());
        assert!(s.nos.present(), "{car}: a nitrous level is picked");
        assert!(s.dimension.z > 1.9, "{car}: {:?}", s.dimension);
        // The wheels hang inside the body: the axles are within its length.
        let rear = s.chassis.front_axle - s.chassis.wheel_base;
        assert!(s.chassis.front_axle < s.dimension.z + 0.3 && rear > -s.dimension.z - 0.3, "{car}: {:?}", s.chassis);
    }
}

/// The tractors of the traffic patterns and the trailer each one pulls (`semi` has none).
const TRACTORS: [(&str, Option<&str>); 7] = [
    ("semi", None),
    ("semia", Some("trailera")),
    ("semib", Some("trailerb")),
    ("semicmt", Some("trailercmt")),
    ("semicon", Some("trailercon")),
    ("semicrate", Some("trailercrate")),
    ("semilog", Some("trailerlog")),
];

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_semi_tractors_and_trailers_have_physics() {
    let Some(dir) = install() else { return };
    let data = physics::PhysicsData::load(&dir).unwrap();
    let tractor = data.car("semi").unwrap();
    for (name, trailer) in TRACTORS {
        let p = data.car(name).unwrap_or_else(|e| panic!("{name}: {e:#}"));
        // The variants have no bounds of their own: they use the box of `semi`.
        assert_eq!(p.bounds, tractor.bounds, "{name}");
        assert_eq!(p.spec.mass, 10_000.0, "{name}");
        assert!(p.spec.engine.torque.len() >= 2, "{name} has an engine");
        assert_eq!(data.trailer_of(name).as_deref(), trailer, "{name}");
        let Some(trailer) = trailer else { continue };
        let t = data.car(trailer).unwrap_or_else(|e| panic!("{trailer}: {e:#}"));
        assert!((10_000.0..=10_001.0).contains(&t.spec.mass), "{trailer}: {} kg", t.spec.mass);
        assert!(t.spec.engine.torque.is_empty() && t.spec.transmission.gear_ratio.iter().all(|&r| r == 0.0));
        assert!(t.spec.dimension.z > p.spec.dimension.z, "{trailer} is longer than its tractor");
        // The axles sit at the back of the trailer's box.
        let front = t.spec.chassis.front_axle;
        let rear = front - t.spec.chassis.wheel_base;
        assert!(front < 0.0 && rear > -t.spec.dimension.z, "{trailer}: {:?}", t.spec.chassis);
    }
    assert_eq!(data.lineage("semib"), ["semib", "semi", "tractors", "cars", "default"]);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn walls_have_friction_and_bounce() {
    let Some(dir) = install() else { return };
    let data = physics::PhysicsData::load(&dir).unwrap();
    let w = data.car("BMWM3GTR").unwrap().walls;
    assert!(w.friction[0] > 0.0 && w.friction[0] <= 1.5 && w.friction[1] <= w.friction[0] + 1e-6, "{w:?}");
    assert!(w.elasticity.max_element() <= 1.0 && w.elasticity.min_element() >= 0.0, "{w:?}");
    assert!(w.moment_scale.min_element() >= 1.0, "{w:?}");
}
