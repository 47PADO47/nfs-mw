use serde_json::json;

use super::*;

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ride6-bikes-camera-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn object(name: &str, properties: Value) -> Value {
    json!({ "Name": format!("{name}_GEN_VARIABLE"), "Properties": properties })
}

fn xyz(x: f64, y: f64, z: f64) -> Value {
    json!({ "X": x, "Y": y, "Z": z })
}

/// A physics dump with a root base container, a class base and one bike, as the dump lays them out.
fn dump(name: &str) -> PathBuf {
    let dir = temp(name);
    let containers = CONTAINERS.iter().fold(dir.clone(), |dir, part| dir.join(part));
    let class = containers.join("Sport");
    fs::create_dir_all(&class).unwrap();
    let root = json!([
        object(
            "ChaseNearExternalCameraBehavior",
            json!({"RelativeLocation": xyz(-330.0, 0.0, 100.0), "TargetLocalOffset": xyz(30.0, 0.0, 70.0)})
        ),
        object(
            "ChaseFarExternalCameraBehavior",
            json!({"RelativeLocation": xyz(-400.0, 0.0, 145.0), "TargetLocalOffset": xyz(0.0, 0.0, 70.0)})
        ),
        object("ThirdPersonNear", json!({"FieldOfView": 85.5})),
        object("ThirdPersonFar", json!({"FieldOfView": 86.0})),
        object(
            "FrontCamInternalCameraBehavior",
            json!({"RelativeLocation": xyz(88.0, 0.0, 110.0), "RollFactor": 0.8, "PitchFactor": 0.3})
        ),
        object("FrontCamReplay", json!({"FieldOfView": 104.5})),
        object(
            "HelmetInternalCameraBehavior",
            json!({"RollFactor": 0.0, "PitchFactor": 0.25, "RelativeLocation": xyz(10.0, 0.0, 120.0)})
        ),
        object("Helmet", json!({"FieldOfView": 91.5})),
        object("HelmetIgnitionNearPlaneCameraBehavior", json!({"OverrideNearPlane": 10.0})),
        object("GameHelmetCameraBehavior", json!({"FollowCameraRoll": true, "MaxAngle": 5.0})),
    ]);
    fs::write(containers.join("GameBaseCAmeraContainer.json"), root.to_string()).unwrap();
    fs::write(
        class.join("GameBaseCAmeraContainer_Sport.json"),
        json!([object("ThirdPersonFar", json!({"FieldOfView": 88.0}))]).to_string(),
    )
    .unwrap();
    let bike = json!([
        object("ChaseNearExternalCameraBehavior", json!({"RelativeLocation": xyz(-300.0, 0.0, 90.0)})),
        object(
            "HelmetInternalCameraBehavior",
            json!({"RelativeLocation": xyz(5.0, 0.0, 130.0), "RelativeRotation": {"Pitch": -10.0, "Yaw": 0.0, "Roll": 0.0}})
        ),
    ]);
    fs::write(class.join("Ducati_Test-Bike-2020.json"), bike.to_string()).unwrap();
    dir
}

const FRAMES: Frames = Frames { offset: [0.0016, 0.08] };

fn parsed(text: &str) -> blackbox_modding::content::BikeCamera {
    let table: toml::Table = text.parse().expect("the tables are TOML");
    table["bike"]["camera"].clone().try_into().expect("the tables are the content schema's")
}

#[test]
fn a_number_comes_from_the_bike_then_its_class_then_the_root_container() {
    let physics = dump("layers");
    let container = Container::read(&physics, "Ducati_Test_Bike-2020").expect("found by its letters and digits");
    assert_eq!(
        container.location("ChaseNearExternalCameraBehavior", "RelativeLocation"),
        Some([-300.0, 0.0, 90.0]),
        "the bike's own"
    );
    assert_eq!(
        container.location("ChaseNearExternalCameraBehavior", "TargetLocalOffset"),
        Some([30.0, 0.0, 70.0]),
        "the root's"
    );
    assert_eq!(container.number("ThirdPersonFar", "FieldOfView"), Some(88.0), "the class's over the root's");
    assert_eq!(container.number("ThirdPersonNear", "FieldOfView"), Some(85.5));
    assert!(Container::read(&physics, "Nobody_Rides_This").is_err());
    fs::remove_dir_all(&physics).unwrap();
}

#[test]
fn the_tables_are_in_metres_and_the_models_frame_and_read_back_by_the_content_schema() {
    let physics = dump("tables");
    let container = Container::read(&physics, "Ducati_Test_Bike-2020").unwrap();
    let text = tables(&container, FRAMES, 0.73).expect("every camera is in the data");
    let cameras = parsed(&text);
    let near = cameras.chase_near.expect("the near chase");
    assert!((near.distance_m - (3.0 - 0.0016)).abs() < 1e-4, "{near:?}");
    assert!((near.height_m - (0.9 + 0.08)).abs() < 1e-4);
    assert!((near.target_ahead_m - (0.3 + 0.0016)).abs() < 1e-4 && (near.target_height_m - 0.78).abs() < 1e-4);
    assert_eq!(near.fov_deg, 85.5);
    assert_eq!(cameras.chase_far.expect("the far chase").fov_deg, 88.0);
    let front = cameras.front.expect("the front camera");
    assert!(front.position_m[0] > 0.73 + 1.5, "ahead of the front hub: {:?}", front.position_m);
    assert!((front.position_m[2] - 1.18).abs() < 1e-4, "the mount's height");
    assert!(front.look_back);
    assert_eq!((front.fov_deg, front.pitch_factor, front.roll_factor), (104.5, 0.3, 0.8));
    let helmet = cameras.helmet.expect("the helmet camera");
    assert!((helmet.fixed_position_m[0] - 0.0516).abs() < 1e-4 && (helmet.fixed_position_m[2] - 1.38).abs() < 1e-4);
    assert_eq!((helmet.pitch_deg, helmet.fov_deg, helmet.near_m), (-10.0, 91.5, 0.1));
    assert_eq!((helmet.pitch_factor, helmet.roll_factor, helmet.follow_roll_max_deg), (0.25, 0.0, 5.0));
    assert_eq!(helmet.eye_bones, ["eye_l", "eye_r"]);
    assert_eq!(helmet.hide_meshes, ["helmetRacing_default", "visorClosedRacing_default"]);
    fs::remove_dir_all(&physics).unwrap();
}

#[test]
fn a_helmet_camera_tilted_down_further_than_any_other_class_does_is_capped() {
    let physics = dump("cap");
    let class = CONTAINERS.iter().fold(physics.clone(), |dir, part| dir.join(part)).join("Sport");
    let steep = json!([object(
        "HelmetInternalCameraBehavior",
        json!({"RelativeRotation": {"Pitch": -60.0, "Yaw": 0.0, "Roll": 0.0}})
    )]);
    fs::write(class.join("Ducati_Test-Bike-2020.json"), steep.to_string()).unwrap();
    let container = Container::read(&physics, "Ducati_Test_Bike-2020").unwrap();
    let helmet = parsed(&tables(&container, FRAMES, 0.73).unwrap()).helmet.unwrap();
    assert_eq!(helmet.pitch_deg, -10.0);
    fs::remove_dir_all(&physics).unwrap();
}

#[test]
fn the_unreal_y_axis_points_right_and_the_models_left() {
    assert_eq!(Frames { offset: [0.0, 0.0] }.point([100.0, 20.0, 50.0]), [1.0, -0.2, 0.5]);
}

#[test]
fn the_frames_differ_by_the_mean_of_the_two_hubs_differences() {
    let frames = Frames::from_hubs(([0.7356, 0.2248], [-0.7389, 0.2323]), ([0.7372, 0.3046], [-0.7372, 0.3121]));
    assert!((frames.offset[0] - 0.0017).abs() < 1e-3 && (frames.offset[1] - 0.0798).abs() < 1e-3, "{frames:?}");
}

#[test]
fn replacing_the_tables_keeps_the_rest_of_the_manifest_and_a_second_run_changes_nothing() {
    let manifest =
        "kind = \"bike\"\n\n[bike]\ndrag = 0.15\n\n[bike.camera.chase_near]\ndistance_m = 1.0\n\n[bike.front]\nx = 1\n";
    let once = replace(manifest, "[bike.camera.helmet]\nfov_deg = 90.0\n");
    assert!(once.contains("[bike]\ndrag = 0.15") && once.contains("[bike.front]\nx = 1"), "{once}");
    assert!(!once.contains("chase_near") && once.ends_with("[bike.camera.helmet]\nfov_deg = 90.0\n"), "{once}");
    assert_eq!(replace(&once, "[bike.camera.helmet]\nfov_deg = 90.0\n"), once);
}

#[test]
fn apply_writes_the_tables_into_an_imported_bikes_manifest_using_its_hubs() {
    let physics = dump("apply");
    let mod_dir = temp("apply-mod");
    fs::create_dir_all(mod_dir.join("cars")).unwrap();
    let rider = mod_dir.join("files/CARS/TESTBIKE/RIDER");
    fs::create_dir_all(&rider).unwrap();
    fs::write(
        rider.join("contacts.json"),
        json!({"hubs": {"front": [0.7356, 0.0, 0.2248], "rear": [-0.7389, 0.0, 0.2323]}}).to_string(),
    )
    .unwrap();
    let manifest =
        "kind = \"bike\"\n\n[wheels.front]\nx_m = 0.7372\nz_m = 0.3046\n\n[wheels.rear]\nx_m = -0.7372\nz_m = 0.3121\n";
    fs::write(mod_dir.join("cars/testbike.toml"), manifest).unwrap();
    apply(&mod_dir, "TESTBIKE", &physics, "Ducati_Test_Bike-2020").unwrap();
    let text = fs::read_to_string(mod_dir.join("cars/testbike.toml")).unwrap();
    assert!(text.starts_with("kind = \"bike\"") && text.contains("[wheels.rear]"));
    let near = parsed(&text).chase_near.unwrap();
    assert!((near.height_m - (0.9 + 0.0798)).abs() < 2e-3, "the model's frame is the skeleton's moved up: {near:?}");
    // Again: the tables are replaced, not added.
    apply(&mod_dir, "TESTBIKE", &physics, "Ducati_Test_Bike-2020").unwrap();
    assert_eq!(fs::read_to_string(mod_dir.join("cars/testbike.toml")).unwrap(), text);
    // A bike without contact points has no tables.
    fs::remove_file(rider.join("contacts.json")).unwrap();
    assert!(apply(&mod_dir, "TESTBIKE", &physics, "Ducati_Test_Bike-2020").is_err());
    fs::remove_dir_all(&physics).unwrap();
    fs::remove_dir_all(&mod_dir).unwrap();
}

#[test]
fn a_bike_a_model_year_apart_takes_the_container_of_the_same_name_nearest_in_time() {
    let dir = temp("years");
    let class = dir.join("Sport");
    fs::create_dir_all(&class).unwrap();
    for name in ["Suzuki_GSX-R-1000-2007", "Suzuki_GSX-R-1000-2019", "Suzuki_GSX-R-1000-RM-2019", "Yamaha_MT07-2020"] {
        fs::write(class.join(format!("{name}.json")), "[]").unwrap();
    }
    let stem =
        |vehicle: &str| locate(&dir, vehicle).map(|path| path.file_stem().unwrap().to_string_lossy().into_owned());
    assert_eq!(stem("Suzuki_GSX-R1000_2020").unwrap(), "Suzuki_GSX-R-1000-2019", "the nearest year, not the first");
    assert_eq!(stem("Suzuki_GSX-R-1000-2007").unwrap(), "Suzuki_GSX-R-1000-2007", "an exact name wins");
    assert_eq!(stem("Yamaha_MT-07_2021").unwrap(), "Yamaha_MT07-2020");
    assert_eq!(stem("Suzuki_GSX-R1000_RM_2020").unwrap(), "Suzuki_GSX-R-1000-RM-2019", "the race version has its own");
    assert!(stem("Suzuki_GSX-R1000_EM_2020").is_err(), "a name that differs in more than the year is another bike");
    assert!(stem("Yamaha_MT-09_2021").is_err());
    fs::remove_dir_all(&dir).unwrap();
}
