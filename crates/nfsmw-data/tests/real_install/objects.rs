//! Contact materials read from the user's installation.

use blackbox_attrib::Database;
use glam::Vec3;
use nfsmw_data::car::physics::{self, Fields};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn vehicle_contact_materials_come_from_their_own_records() {
    let Some(dir) = crate::install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let samples = [
        ("bmwm3gtr", [0.0, 0.0, 0.05], [4.0, 15.0, 3.0], 1.0),
        ("trafha", [0.0, 0.0, 0.1], [2.0, 1.0, 4.0], 0.5),
        ("semi", [0.0, 0.0, 0.05], [2.0, 3.0, 2.0], 1.0),
        ("copmidsize", [0.15, 0.0, 0.15], [1.2, 1.2, 1.0], 0.75),
    ];
    for (name, elasticity, inertia, mass) in samples {
        let car = db.collection("pvehicle", name).unwrap();
        let body = physics::body(Fields(car));
        let material = body.spec.objects;
        assert_eq!(material.friction, [0.55, 0.35], "{name}");
        assert_eq!(material.elasticity, Vec3::from(elasticity), "{name}");
        assert_eq!(material.inertia_scale, Vec3::from(inertia), "{name}");
        assert_eq!(material.mass_scale, mass, "{name}");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn prop_attributes_preserve_mass_inertia_and_breakaway_values() {
    let Some(dir) = crate::install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let samples = [
        ("cone", 100.0, 0.0, true, Vec3::splat(3.0)),
        ("trafficlight", 200.0, 7500.0, true, Vec3::new(1.0, 2.0, 1.0)),
        ("tree", 100.0, 20000.0, true, Vec3::splat(2.0)),
        ("log", 1000.0, 0.0, false, Vec3::splat(3.0)),
    ];
    for (name, mass, detach, no_effect, tensor) in samples {
        let c = db.collection("smackable", name).unwrap();
        let p = nfsmw_data::world::PropDynamics::read(c).unwrap();
        assert_eq!(p.mass, mass, "{name}");
        assert_eq!(p.detach_impulse, detach, "{name}");
        assert_eq!(p.no_car_effect, no_effect, "{name}");
        assert_eq!(p.tensor_scale, tensor, "three-component MOMENT in {name}");
        assert_eq!(p.respawn_time, 120.0, "{name}");
        assert_eq!(p.body.objects.friction, [0.35, 0.23], "{name}");
    }
}
