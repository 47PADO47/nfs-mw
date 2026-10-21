//! Bounded installed behavior inventory before choosing a traffic suspension adapter.

use super::*;

mod controlled;

#[test]
#[ignore = "needs installed traffic attributes selected by NFSMW_GAME_DIR"]
fn installed_traffic_suspension_behavior_inventory() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    for (name, expected) in [
        ("traf4dseda", "SuspensionTraffic"),
        ("trafvanb", "SuspensionTraffic"),
        ("trafamb", "SuspensionTraffic"),
        ("semi", "SuspensionTraffic"),
        ("semib", "SuspensionTraffic"),
        ("semilog", "SuspensionTraffic"),
        ("trailerb", "SuspensionTrailer"),
        ("trailerlog", "SuspensionTrailer"),
        ("copmidsize", "SuspensionSimple"),
    ] {
        let vehicle = data.database().collection("pvehicle", name).unwrap();
        let mechanic = vehicle.get_string_key("BEHAVIOR_MECHANIC_SUSPENSION").unwrap();
        eprintln!("installed suspension {name}: {mechanic:?}");
        assert_eq!(
            mechanic.hash32,
            vlt_hash(expected),
            "{name} must identify its authored suspension before an adapter is chosen"
        );
    }
}
