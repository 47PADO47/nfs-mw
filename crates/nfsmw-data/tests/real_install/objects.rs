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
