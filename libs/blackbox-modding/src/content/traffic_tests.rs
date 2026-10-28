use super::*;

fn car(text: &str) -> Result<CarManifest, ContentError> {
    parse_car("hatchback", text)
}

#[test]
fn a_car_that_is_not_traffic_has_no_traffic_entry() {
    assert_eq!(car("").unwrap().traffic, None);
    assert_eq!(car("role = \"both\"\n").unwrap().traffic, None);
}

#[test]
fn the_traffic_role_is_traffic_only_and_takes_the_defaults_of_a_sedan() {
    let manifest = car("role = \"traffic\"\n").unwrap();
    assert_eq!(manifest.role, Role::Traffic);
    assert!(manifest.role.is_traffic() && !manifest.role.is_player() && !manifest.role.is_cop());
    assert_eq!(manifest.traffic, Some(Traffic::of_class(TrafficClass::Sedan)));
    let traffic = manifest.traffic.unwrap();
    assert_eq!((traffic.weight, traffic.max_instances, traffic.speed_factor), (20, 0, 1.0));
    assert!(traffic.patterns.is_empty(), "no pattern named: every pattern");
}

#[test]
fn the_class_sets_the_defaults_and_the_table_overrides_them() {
    let manifest = car("role = \"traffic\"\n[traffic]\nclass = \"truck\"\n").unwrap();
    let truck = manifest.traffic.unwrap();
    assert_eq!(truck.class, TrafficClass::Truck);
    assert_eq!((truck.weight, truck.max_instances), (6, 1), "rare and one at a time");
    assert!(truck.speed_factor < 1.0);
    let text = "role = \"traffic\"\n[traffic]\nclass = \"van\"\nweight = 40\nmax_instances = 3\nspeed_factor = 1.2\n";
    let van = car(text).unwrap().traffic.unwrap();
    assert_eq!((van.class, van.weight, van.max_instances, van.speed_factor), (TrafficClass::Van, 40, 3, 1.2));
}

#[test]
fn every_class_is_known_by_its_name() {
    for name in ["compact", "sedan", "coupe", "suv", "pickup", "van", "truck", "bus", "taxi"] {
        let manifest = car(&format!("role = \"traffic\"\n[traffic]\nclass = \"{name}\"\n")).unwrap();
        assert_eq!(manifest.traffic.unwrap().class.name(), name);
    }
    assert!(matches!(car("role = \"traffic\"\n[traffic]\nclass = \"tank\"\n"), Err(ContentError::Parse(_))));
}

#[test]
fn a_player_car_may_be_traffic_too() {
    let manifest = car("role = \"player\"\n[traffic]\nclass = \"coupe\"\n").unwrap();
    assert!(manifest.role.is_player() && !manifest.role.is_traffic());
    assert_eq!(manifest.traffic.unwrap().class, TrafficClass::Coupe);
}

#[test]
fn the_patterns_are_lower_case_and_listed_once() {
    let manifest = car("role = \"traffic\"\n[traffic]\npatterns = [\"Downtown\", \"collegeSouth\"]\n").unwrap();
    assert_eq!(manifest.traffic.unwrap().patterns, ["downtown", "collegesouth"]);
    let again = "role = \"traffic\"\n[traffic]\npatterns = [\"downtown\", \"DOWNTOWN\"]\n";
    assert!(matches!(car(again), Err(ContentError::Field { field: "traffic patterns", .. })));
    let empty = "role = \"traffic\"\n[traffic]\npatterns = [\"\"]\n";
    assert!(matches!(car(empty), Err(ContentError::Field { field: "traffic patterns", .. })));
    let spaced = "role = \"traffic\"\n[traffic]\npatterns = [\"down town\"]\n";
    assert!(matches!(car(spaced), Err(ContentError::Field { field: "traffic patterns", .. })));
}

#[test]
fn numbers_outside_their_range_are_refused() {
    let table = |line: &str| car(&format!("role = \"traffic\"\n[traffic]\n{line}\n"));
    assert!(matches!(table("weight = 0"), Err(ContentError::Field { field: "traffic weight", .. })));
    assert!(matches!(table("weight = 101"), Err(ContentError::Field { field: "traffic weight", .. })));
    assert!(matches!(table("max_instances = 17"), Err(ContentError::Field { field: "traffic max_instances", .. })));
    assert!(matches!(table("speed_factor = 0.4"), Err(ContentError::Field { field: "traffic speed_factor", .. })));
    assert!(matches!(table("speed_factor = 1.6"), Err(ContentError::Field { field: "traffic speed_factor", .. })));
    assert!(table("weight = 1\nmax_instances = 16\nspeed_factor = 0.5").is_ok());
    assert!(table("weight = 100\nmax_instances = 0\nspeed_factor = 1.5").is_ok());
}

#[test]
fn an_unknown_key_of_the_table_is_refused() {
    let error = car("role = \"traffic\"\n[traffic]\nnight = true\n").unwrap_err();
    assert!(matches!(&error, ContentError::Parse(message) if message.contains("night")), "{error}");
}

#[test]
fn a_traffic_car_has_no_cop_keys() {
    assert_eq!(car("role = \"traffic\"\nheat = [1]\n"), Err(ContentError::HeatWithoutCop));
    assert_eq!(car("role = \"traffic\"\ncop_class = \"heavy\"\n"), Err(ContentError::HeatWithoutCop));
}
