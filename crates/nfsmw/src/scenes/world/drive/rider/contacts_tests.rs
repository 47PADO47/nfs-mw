use super::super::attach::tests::SAMPLE;
use super::super::pose::{Frame, rider_targets};
use super::*;

/// A cruiser's contacts: its own seat, hubs, and all four targets (the rig's sample has two).
const CRUISER: &str = r#"{
    "version": 1, "vehicle": "Cruiser",
    "rider_root_in_bike": {"translation": [-0.10, 0.0, 0.70], "rotation": [0.0, 0.0, 0.7071068, 0.7071068]},
    "hubs": {"front": [0.80, 0.0, 0.30], "rear": [-0.78, 0.0, 0.31]},
    "targets": [
        {"name": "left_grip", "position": [0.20, 0.45, 1.10], "steers": true},
        {"name": "right_grip", "position": [0.20, -0.45, 1.10], "steers": true},
        {"name": "left_foot", "position": [0.30, 0.25, 0.30], "steers": false},
        {"name": "right_foot", "position": [0.30, -0.25, 0.30], "steers": false}
    ],
    "correction": {"lean_knee": 0.65, "pole_elbow_cm": [-7.0, 0.0, -10.0], "bike_type": "STREET"}
}"#;

fn rig() -> Attach {
    Attach::parse(SAMPLE.as_bytes()).unwrap()
}

#[test]
fn a_file_as_the_converter_writes_it_reads_back_with_its_correction() {
    let contacts = Contacts::parse(CRUISER.as_bytes()).unwrap();
    assert_eq!(contacts.targets.len(), 4);
    let correction = contacts.correction.unwrap();
    assert_eq!(correction.lean_knee, 0.65);
    assert_eq!(correction.pole_elbow_cm, [-7.0, 0.0, -10.0]);
    assert_eq!(correction.bike_type.as_deref(), Some("STREET"));
    assert_eq!(correction.knee, 0.0, "a key the file leaves out is 0");
}

#[test]
fn a_file_without_a_correction_is_still_a_file() {
    let text = r#"{
        "version": 1,
        "rider_root_in_bike": {"translation": [0.0, 0.0, 0.8], "rotation": [0.0, 0.0, 0.7071068, 0.7071068]},
        "hubs": {"front": [0.7, 0.0, 0.3], "rear": [-0.7, 0.0, 0.3]},
        "targets": [{"name": "left_grip", "position": [0.2, 0.3, 1.0], "steers": true}]
    }"#;
    let contacts = Contacts::parse(text.as_bytes()).unwrap();
    assert!(contacts.correction.is_none());
}

#[test]
fn the_bikes_seat_hubs_and_targets_replace_the_rigs_and_the_rest_stays() {
    let rig = rig();
    let merged = rig.clone().with_contacts(&Contacts::parse(CRUISER.as_bytes()).unwrap()).unwrap();
    assert_eq!(merged.hubs.front, [0.80, 0.0, 0.30]);
    assert_eq!(merged.rider_root_in_bike.translation, [-0.10, 0.0, 0.70]);
    // The rig placed left_grip and left_foot; both are the bike's now, and the right ones were added.
    assert_eq!(merged.targets.len(), 4);
    let grip = &merged.targets[merged.target_index("left_grip").unwrap()];
    assert_eq!(grip.position, [0.20, 0.45, 1.10]);
    assert!(grip.steers);
    assert!(!merged.targets[merged.target_index("left_foot").unwrap()].steers);
    // Limbs and inertia are the rig's.
    assert_eq!(merged.limbs.len(), rig.limbs.len());
    assert_eq!(merged.limbs[0].tip_offset, rig.limbs[0].tip_offset);
    assert_eq!(merged.inertia.bone, rig.inertia.bone);
}

#[test]
fn the_hands_and_feet_of_two_bikes_are_aimed_at_their_own_controls() {
    let rig = rig();
    let cruiser = rig.clone().with_contacts(&Contacts::parse(CRUISER.as_bytes()).unwrap()).unwrap();
    let frame = |attach: &Attach| {
        let offset = attach.frame_offset(Vec3::from_array(attach.hubs.front), Vec3::from_array(attach.hubs.rear));
        Frame::new(&attach.rider_root_in_bike.transform(), offset)
    };
    let (own, shared) = (rider_targets(&cruiser, &frame(&cruiser), None), rider_targets(&rig, &frame(&rig), None));
    let grip = cruiser.target_index("left_grip").unwrap();
    // In rider space the target is the bike's point seen from the rider's root: the cruiser's grip is not the sample's.
    assert!(own[grip].position.distance(shared[rig.target_index("left_grip").unwrap()].position) > 0.1);
    // Seen from the rider (his root on the seat turned to face forward) the cruiser's left grip is 0.30 m ahead of the seat,
    // 0.45 m to the left and 0.40 m up: the rider's own axes are x left, -y forward, z up.
    assert!(own[grip].position.distance(Vec3::new(0.45, -0.30, 0.40)) < 1e-5, "{:?}", own[grip]);
}

#[test]
fn a_file_that_would_put_the_rider_somewhere_absurd_is_refused() {
    let parse = |text: String| Contacts::parse(text.as_bytes());
    assert!(parse(CRUISER.replace(r#""version": 1"#, r#""version": 2"#)).unwrap_err().to_string().contains("version"));
    assert!(parse(CRUISER.replace("[0.20, 0.45, 1.10]", "[0.20, 0.45, 9.0]")).is_err(), "9 m from the seat");
    assert!(
        parse(CRUISER.replace(
            r#"{"front": [0.80, 0.0, 0.30], "rear": [-0.78, 0.0, 0.31]}"#,
            r#"{"front": [-0.80, 0.0, 0.30], "rear": [0.78, 0.0, 0.31]}"#
        ))
        .is_err(),
        "hubs the wrong way round"
    );
    assert!(parse(CRUISER.replace("[0.0, 0.0, 0.7071068, 0.7071068]", "[0.0, 0.0, 0.0, 0.0]")).is_err());
    assert!(Contacts::parse(b"{}").is_err());
    assert!(Contacts::parse(b"nonsense").is_err());
}

#[test]
fn a_limb_left_without_a_target_is_refused() {
    let mut rig = rig();
    rig.limbs[0].target = "elbow_rest".into();
    let error = rig.with_contacts(&Contacts::parse(CRUISER.as_bytes()).unwrap()).unwrap_err();
    assert!(error.to_string().contains("elbow_rest"), "{error}");
}
