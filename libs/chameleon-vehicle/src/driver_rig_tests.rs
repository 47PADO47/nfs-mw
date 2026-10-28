//! The driver rig: posing a figure and reaching its hands to the wheel. The real-install test needs `CHAMELEON_GAME_DIR` (or
//! `MW2012_GAME_DIR`) and `--ignored`.

use std::path::PathBuf;

use chameleon_anim::quat::{self, IDENTITY, Quat};
use chameleon_anim::{AnimationList, Bone, Clip, Skeleton, Skin};
use chameleon_bundle::{Bundle, Pool};
use chameleon_gfx::Vertex;

use crate::driver_rig::{DriverPose, DriverRig, Grips};
use crate::{Appearance, Driver, PartMesh, SurfaceKind};
use crate::{driver_grips, driver_setup, load_driver, load_rig};

/// A turn about the z axis by `degrees`.
fn about_z(degrees: f32) -> Quat {
    let half = degrees.to_radians() * 0.5;
    [0.0, 0.0, half.sin(), half.cos()]
}

/// A two-bone figure, a root and a head a metre above it, with one clip of three keys (a lean of -30, 0 and 30 degrees about z
/// in the head) and one vertex skinned to the head.
fn rig() -> (DriverRig, Driver) {
    let bone = |parent: Option<usize>, y: f32| Bone { hash: 0, parent, position: [0.0, y, 0.0], rotation: IDENTITY };
    let clip = Clip {
        id: 1,
        keys: 3,
        rate: 30.0,
        reference: 0,
        bones: 2,
        rotation_track: vec![None, Some(0)],
        slide_track: vec![None, None],
        rotations: vec![vec![about_z(-30.0)], vec![IDENTITY], vec![about_z(30.0)]],
        slides: vec![Vec::new(); 3],
        positions: vec![Vec::new(); 3],
    };
    let rig = DriverRig {
        skeleton: Skeleton { bones: vec![bone(None, 0.0), bone(Some(0), 1.0)] },
        animations: AnimationList { clips: vec![clip] },
        skins: vec![vec![vec![Skin { bones: [1, 0, 0, 0], weights: [255, 0, 0, 0] }]]],
        arms: [[0, 1, 1], [0, 1, 1]],
    };
    let vertex = Vertex { position: [0.0, 2.0, 0.0], color: [255; 4], uv: [0.5; 2] };
    let mesh = PartMesh {
        vertices: vec![vertex],
        triangles: Vec::new(),
        appearance: Appearance {
            kind: SurfaceKind::Body,
            technique: "Character".into(),
            diffuse_texture: Some(1),
            diffuse_colour: None,
            alpha_test: false,
        },
    };
    (rig, Driver { character: 0x3D8E, lods: vec![vec![mesh]] })
}

#[test]
fn full_lock_leans_the_head_each_way_and_the_middle_is_upright() {
    let (rig, driver) = rig();
    let at = |steer: f32| {
        let pose = rig.pose(DriverPose::Steering(steer), None);
        rig.posed(&driver, &pose, [0.0; 3], 1)[0][0].vertices[0].position
    };
    let (left, middle, right) = (at(-1.0), at(0.0), at(1.0));
    assert!(middle.iter().zip([0.0, 2.0, 0.0]).all(|(a, b)| (a - b).abs() < 1e-4), "{middle:?}");
    // A point a metre above the head bone, turned 30 degrees about z, moves 0.5 sideways.
    assert!((left[0] - 0.5).abs() < 1e-3 && (right[0] + 0.5).abs() < 1e-3, "{left:?} {right:?}");
    assert!((left[1] - 1.866).abs() < 1e-3, "{left:?}");
}

#[test]
fn a_steering_input_beyond_full_lock_is_full_lock() {
    let (rig, _) = rig();
    assert_eq!(rig.pose(DriverPose::Steering(3.0), None), rig.pose(DriverPose::Steering(1.0), None));
}

#[test]
fn the_seat_moves_the_posed_figure_and_only_the_requested_levels_are_made() {
    let (rig, mut driver) = rig();
    driver.lods.push(driver.lods[0].clone());
    let pose = rig.pose(DriverPose::Steering(0.0), None);
    let lods = rig.posed(&driver, &pose, [0.3, -0.5, 0.1], 1);
    assert_eq!(lods.len(), 1, "one level asked for");
    let moved = lods[0][0].vertices[0].position;
    assert!(moved.iter().zip([0.3, 1.5, 0.1]).all(|(a, b)| (a - b).abs() < 1e-4), "{moved:?}");
    assert_eq!(lods[0][0].vertices[0].uv, [0.5; 2], "only positions change");
}

#[test]
fn the_look_back_is_the_clip_whatever_the_grips() {
    let (rig, _) = rig();
    let grips = Some(Grips { left: [1.0, 1.0, 1.0], right: [-1.0, 1.0, 1.0] });
    assert_eq!(rig.pose(DriverPose::LookBack, grips), rig.pose(DriverPose::LookBack, None));
    // The clip has one key row per time: the look-back time is past its last key, which is the +30 degree lean.
    let pose = rig.pose(DriverPose::LookBack, None);
    assert!((quat::angle_degrees(quat::mul(quat::conjugate(IDENTITY), pose.rotations[1])) - 30.0).abs() < 0.1);
}

fn install() -> Option<PathBuf> {
    let dir = std::env::var_os("CHAMELEON_GAME_DIR").or_else(|| std::env::var_os("MW2012_GAME_DIR"))?;
    Some(PathBuf::from(dir))
}

fn read(dir: &std::path::Path, rel: &str) -> Vec<u8> {
    std::fs::read(dir.join(rel)).unwrap_or_else(|error| panic!("{rel}: {error}"))
}

/// All three characters (the officer too) load with a skin for every vertex of every level; every vertex's weights add to about 255 (or are all zero) and
/// name bones of the skeleton; the unposed skeleton leaves the figure where the model has it; and in every car with grips the
/// hands reach them at both full locks.
#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR or MW2012_GAME_DIR)"]
fn the_three_characters_pose_and_reach_the_wheels_of_all_cars() {
    let Some(dir) = install() else { return };
    let characters = read(&dir, "CHARACTERS/ALL_CHARS.BNDL");
    let shaders = read(&dir, "SHADERS.BNDL");
    let mut pool = Pool::new();
    pool.add(Bundle::parse(&characters).unwrap());
    pool.add(Bundle::parse(&shaders).unwrap());
    let mut rigs = std::collections::HashMap::new();
    for character in [0x3D8E_u32, 0x17785D, 0x3D8D] {
        let rig = load_rig(&pool, character).unwrap();
        let driver = load_driver(&pool, character).unwrap();
        assert_eq!(rig.skeleton.bones.len(), 46);
        assert_eq!(rig.animations.clips.len(), 28);
        assert_eq!(rig.skins.len(), driver.lods.len());
        for (skins, lod) in rig.skins.iter().zip(&driver.lods) {
            assert_eq!(skins.len(), lod.len());
            for (skins, mesh) in skins.iter().zip(lod) {
                assert_eq!(skins.len(), mesh.vertices.len());
                for skin in skins {
                    let total: u32 = skin.weights.iter().map(|&w| u32::from(w)).sum();
                    assert!(total == 0 || (253..=257).contains(&total), "{skin:?}");
                    for (&bone, &weight) in skin.bones.iter().zip(&skin.weights) {
                        assert!(weight == 0 || usize::from(bone) < 46, "{skin:?}");
                    }
                }
            }
        }
        // The rest skeleton leaves the figure as modelled.
        let mut pose = chameleon_anim::Pose::new(46);
        rig.skeleton.pose(&rig.skeleton.rest_deltas(), &mut pose);
        let worst = driver.lods[0]
            .iter()
            .zip(&rig.skins[0])
            .flat_map(|(mesh, skins)| mesh.vertices.iter().zip(skins))
            .map(|(vertex, skin)| {
                let moved = rig.skeleton.skin_point(&pose, skin, vertex.position);
                (0..3).map(|k| (moved[k] - vertex.position[k]).abs()).fold(0.0, f32::max)
            })
            .fold(0.0, f32::max);
        assert!(worst < 1e-3, "character {character:X}: the rest pose moves a vertex by {worst}");
        rigs.insert(character, rig);
    }
    let (mut with_grips, mut worst_miss, mut worst_neutral, mut worst_head) = (0, 0.0f32, 0.0f32, 0.0f32);
    for entry in std::fs::read_dir(dir.join("VEHICLES")).unwrap().flatten() {
        let file = entry.file_name().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let data = std::fs::read(entry.path()).unwrap();
        let bundle = Bundle::parse(&data).unwrap();
        let Some(setup) = driver_setup(&bundle) else { continue };
        let Some(seat) = setup.seat else { continue };
        let Some(grips) = driver_grips(&bundle, &setup, seat) else { continue };
        with_grips += 1;
        // The grips are on the wheel: their middle is within 20 cm of the car's steering wheel bone (a wheel's diameter is
        // 31 to 50 cm between them).
        if let Some(wheel) = crate::skeleton::Skeleton::read(&bundle)
            .and_then(|skeleton| skeleton.find("SteeringWheel").map(|bone| skeleton.bones[bone].position))
        {
            let middle = [0, 1, 2].map(|k| (grips.left[k] + grips.right[k]) * 0.5 + seat[k]);
            let off = (0..3).map(|k| (middle[k] - wheel[k]).powi(2)).sum::<f32>().sqrt();
            assert!(off < 0.2, "{file}: the grips' middle is {off} m from the steering wheel bone");
            let apart = (0..3).map(|k| (grips.left[k] - grips.right[k]).powi(2)).sum::<f32>().sqrt();
            assert!((0.25..0.6).contains(&apart), "{file}: the grips are {apart} m apart");
        }
        let rig = &rigs[&setup.character];
        let wrist = |pose: &chameleon_anim::Pose, bone: usize| pose.positions[bone];
        let neutral = rig.pose(DriverPose::Steering(0.0), None);
        let reach = |pose: &chameleon_anim::Pose, target: [f32; 3], bone: usize| {
            let at = wrist(pose, bone);
            (0..3).map(|k| (at[k] - target[k]).powi(2)).sum::<f32>().sqrt()
        };
        let (left, right) = (rig.arms[0][2], rig.arms[1][2]);
        worst_neutral = worst_neutral.max(reach(&neutral, grips.left, left)).max(reach(&neutral, grips.right, right));
        let head = rig.skeleton.find("Head").unwrap();
        let (a, b) =
            (rig.pose(DriverPose::Steering(-1.0), Some(grips)), rig.pose(DriverPose::Steering(1.0), Some(grips)));
        let moved = (0..3).map(|k| (a.positions[head][k] - b.positions[head][k]).powi(2)).sum::<f32>().sqrt();
        worst_head = worst_head.max(moved);
        assert!(moved > 0.002 && moved < 0.2, "{file}: the head moves {moved} m between the locks");
        for steer in [-1.0, 0.0, 1.0] {
            let pose = rig.pose(DriverPose::Steering(steer), Some(grips));
            let miss = reach(&pose, grips.left, left).max(reach(&pose, grips.right, right));
            worst_miss = worst_miss.max(miss);
            assert!(miss < 0.05, "{file}: the hands are {miss} m from the grips at steer {steer}");
        }
    }
    println!(
        "{with_grips} cars with grips; worst neutral FK distance {worst_neutral:.3} m; worst reach miss {worst_miss:.4} m; head travel up to {worst_head:.3} m"
    );
    assert!(with_grips > 80);
}
