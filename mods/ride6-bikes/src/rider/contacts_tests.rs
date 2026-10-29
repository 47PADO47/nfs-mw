use super::*;

/// A `parts.json` reduced to what the contacts read. Bones are glTF world places (x forward, y up, z right, metres) and
/// glTF turns; sockets are Unreal's (centimetres, a rotator). `grip` is the left grip bone's place; the right one mirrors it.
fn parts(vehicle: &str, grip: [f64; 3], seat_cm: [f64; 3], peg_cm: [f64; 3]) -> Value {
    let identity = [0.0, 0.0, 0.0, -1.0];
    let bone = |position: [f64; 3]| json!({ "position": position, "rotation": identity });
    let socket = |bone: &str, at: [f64; 3], rotator: [f64; 3]| json!({ "bone": bone, "location_cm": at, "rotation_deg": rotator });
    json!({
        "schema": 1,
        "vehicle": vehicle,
        "bones": {
            "BONE_CHASSIS": bone([0.0, 0.0, 0.0]),
            "BONE_F_VIS_WHEEL": bone([0.73, 0.22, 0.0]),
            "BONE_B_VIS_WHEEL": bone([-0.74, 0.23, 0.0]),
            "BONE_L_GRIP": bone([grip[0], grip[1], -grip[2]]),
            "BONE_R_GRIP": bone(grip),
        },
        "sockets": {
            "SOCKET_RIDER": socket("BONE_CHASSIS", seat_cm, [0.0, -90.0, 0.0]),
            "SOCKET_L_HANDLE_POSE": socket("BONE_L_GRIP", [0.0; 3], [0.0; 3]),
            "SOCKET_R_HANDLE_POSE": socket("BONE_R_GRIP", [0.0; 3], [0.0; 3]),
            "SOCKET_LEFT_FOOT": socket("BONE_CHASSIS", [peg_cm[0], -peg_cm[1], peg_cm[2]], [0.0; 3]),
            "SOCKET_RIGHT_FOOT": socket("BONE_CHASSIS", peg_cm, [0.0; 3]),
        },
    })
}

fn position(json: &Value, role: &str) -> Vec3 {
    let targets = json["targets"].as_array().unwrap();
    let target = targets.iter().find(|t| t["name"] == role).unwrap();
    let p = target["position"].as_array().unwrap();
    Vec3::new(p[0].as_f64().unwrap() as f32, p[1].as_f64().unwrap() as f32, p[2].as_f64().unwrap() as f32)
}

fn sport() -> Value {
    parts("Sport_Bike", [0.37, 0.91, 0.21], [-27.0, 0.0, 95.0], [-33.0, 22.0, 49.0])
}

fn cruiser() -> Value {
    parts("Cruiser_Bike", [0.20, 1.10, 0.45], [-10.0, 0.0, 70.0], [30.0, 25.0, 30.0])
}

#[test]
fn a_grip_bone_of_the_gltf_skeleton_lands_on_the_left_in_the_car_frame() {
    let json = contacts_json(&sport(), None).unwrap();
    // glTF (x, y up, z right) -> car (x, -z, y): the left grip bone is at z = -0.21 (left), 0.91 up.
    assert!(position(&json, "left_grip").distance(Vec3::new(0.37, 0.21, 0.91)) < 1e-5);
    assert!(position(&json, "right_grip").distance(Vec3::new(0.37, -0.21, 0.91)) < 1e-5);
    // The pegs are sockets of the chassis bone, in Unreal centimetres: (x, y right, z) -> car (x, -y, z) / 100.
    assert!(position(&json, "right_foot").distance(Vec3::new(-0.33, -0.22, 0.49)) < 1e-5);
    assert!(position(&json, "left_foot").distance(Vec3::new(-0.33, 0.22, 0.49)) < 1e-5);
    assert!((json["hubs"]["front"][0].as_f64().unwrap() - 0.73).abs() < 1e-5);
    assert!((json["hubs"]["rear"][1].as_f64().unwrap()).abs() < 1e-6, "the hubs are on the centre line");
    assert_eq!(json["vehicle"], "Sport_Bike");
    assert_eq!(json["version"], VERSION);
}

#[test]
fn the_seat_is_the_riders_socket_turned_to_face_forward() {
    let json = contacts_json(&sport(), None).unwrap();
    let seat = &json["rider_root_in_bike"];
    let t = seat["translation"].as_array().unwrap();
    assert!((t[0].as_f64().unwrap() + 0.27).abs() < 1e-5 && (t[2].as_f64().unwrap() - 0.95).abs() < 1e-5);
    // Unreal yaw -90 degrees is a +90 degree turn about z in the car frame (the spec's seat).
    let q = seat["rotation"].as_array().unwrap();
    let z = q[2].as_f64().unwrap().abs();
    assert!((z - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-5, "{q:?}");
}

#[test]
fn two_bikes_with_different_controls_get_different_contact_points() {
    let (sport, cruiser) = (contacts_json(&sport(), None).unwrap(), contacts_json(&cruiser(), None).unwrap());
    for role in ["left_grip", "right_grip", "left_foot", "right_foot"] {
        assert!(position(&sport, role).distance(position(&cruiser, role)) > 0.1, "{role}");
    }
    assert_ne!(sport["rider_root_in_bike"], cruiser["rider_root_in_bike"]);
    // The cruiser's high bars are above its seat; its forward pegs are ahead of the seat.
    assert!(
        position(&cruiser, "left_foot").x > cruiser["rider_root_in_bike"]["translation"][0].as_f64().unwrap() as f32
    );
}

#[test]
fn a_socket_on_a_turned_bone_is_placed_by_the_bones_turn() {
    // The grip bone pitched 90 degrees nose up (a turn about glTF z, the right axis): a socket 10 cm ahead of the bone
    // in Unreal's x is then 10 cm above it.
    let mut doc = sport();
    let half = std::f64::consts::FRAC_1_SQRT_2;
    doc["bones"]["BONE_R_GRIP"]["rotation"] = json!([0.0, 0.0, half, half]);
    doc["sockets"]["SOCKET_R_HANDLE_POSE"]["location_cm"] = json!([10.0, 0.0, 0.0]);
    let json = contacts_json(&doc, None).unwrap();
    let bone = Vec3::new(0.37, -0.21, 0.91);
    assert!(position(&json, "right_grip").distance(bone + Vec3::new(0.0, 0.0, 0.10)) < 1e-4);
}

#[test]
fn a_skeleton_missing_a_socket_or_a_bone_is_refused_by_name() {
    let mut doc = sport();
    doc["sockets"].as_object_mut().unwrap().remove("SOCKET_LEFT_FOOT");
    let error = contacts_json(&doc, None).unwrap_err();
    assert!(format!("{error:#}").contains("SOCKET_LEFT_FOOT"), "{error:#}");
    let mut doc = sport();
    doc["bones"].as_object_mut().unwrap().remove("BONE_F_VIS_WHEEL");
    assert!(format!("{:#}", contacts_json(&doc, None).unwrap_err()).contains("BONE_F_VIS_WHEEL"));
    assert!(contacts_json(&json!({ "vehicle": "x" }), None).is_err());
}

/// A dump table with one row, as the physics dump writes it.
fn table() -> Value {
    json!([{ "Rows": { "Sport_Bike": {
        "RiderMaxLean": 1.0, "AeroTankCorrection": 0.5, "LeanKneeCorrection": 0.65, "LeanElbowCorrection": 0.26,
        "KneeCorrection": 0.1, "ElbowCorrection": 0.0, "MufflerCorrection": 1.0, "SpineCompensation1": -5.0,
        "SpineCompensation2": 2.0, "ArmsIKSpineCompensationAlpha": 0.0,
        "TransformPVElbowCorrection": { "Translation": { "X": -7.0, "Y": 0.0, "Z": -10.0 } },
        "QuickShiftType": "EQuickShiftType::SEMIAUTOMATIC", "BikeType": "EBikeTypeOverride::STREET"
    } } }])
}

#[test]
fn the_vehicles_correction_row_is_carried_with_the_contacts() {
    let json = contacts_json(&sport(), Some(&table())).unwrap();
    let row = &json["correction"];
    assert_eq!(row["lean_knee"], 0.65);
    assert_eq!(row["spine_1_deg"], -5.0);
    assert_eq!(row["pole_elbow_cm"], json!([-7.0, 0.0, -10.0]));
    assert_eq!(row["bike_type"], "STREET");
    assert_eq!(row["quick_shift"], "SEMIAUTOMATIC");
    // A vehicle with no row has the contacts and no correction.
    assert!(contacts_json(&cruiser(), Some(&table())).unwrap().get("correction").is_none());
    assert!(contacts_json(&cruiser(), None).unwrap().get("correction").is_none());
}

#[test]
fn the_file_is_written_into_the_cars_rider_folder_from_the_bikes_part_table() {
    let dir = std::env::temp_dir().join(format!("ride6-contacts-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let (bike, car, dump) = (dir.join("bike"), dir.join("files/CARS/SPORT"), dir.join("dump"));
    fs::create_dir_all(&bike).unwrap();
    fs::write(bike.join("parts.json"), sport().to_string()).unwrap();
    let table_path = CORRECTION_TABLE.iter().fold(dump.clone(), |path, part| path.join(part));
    fs::create_dir_all(table_path.parent().unwrap()).unwrap();
    fs::write(&table_path, table().to_string()).unwrap();
    assert!(write(&bike, &car, &dump).unwrap(), "the row was found in the dump");
    let written: Value = serde_json::from_slice(&fs::read(car.join("RIDER").join(FILE)).unwrap()).unwrap();
    assert!(position(&written, "left_grip").distance(Vec3::new(0.37, 0.21, 0.91)) < 1e-5);
    // Without a dump the contacts are written and the answer says there is no row.
    assert!(!write(&bike, &car, &dir.join("nowhere")).unwrap());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_shared_attach_file_and_the_contacts_place_the_same_bike_alike() {
    let skeleton = bike_skeleton(&sport()).unwrap();
    let shared = points(&skeleton).unwrap().placement();
    let json = contacts_json(&sport(), None).unwrap();
    for key in ["rider_root_in_bike", "hubs", "targets"] {
        assert_eq!(shared[key], json[key], "{key}");
    }
}
