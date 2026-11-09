//! The rider kept out of the bike's body (`clearance.rs`), on a synthetic bike (a box for a tank) and a synthetic rider (a spine and two
//! legs), in a frame where the rider's space is the bike's.

use blackbox_skeleton::{Bone, Skeleton, Transform, TwoBoneChain, apply_two_bone};
use glam::{Quat, Vec3};

use super::super::fall_file::{BodyFile, Falls, RagdollFile};
use super::super::pose::{Frame, Limb, Resolved};
use super::super::reach::TorsoBones;
use super::*;

const PELVIS: usize = 1;
const SPINE_01: usize = 2;
const SPINE_03: usize = 3;
const THIGH_L: usize = 4;
const CALF_L: usize = 5;
const FOOT_L: usize = 6;
const THIGH_R: usize = 7;
const CALF_R: usize = 8;
const FOOT_R: usize = 9;
const POLE_L: usize = 10;
const POLE_R: usize = 11;

/// The tank: 0.35 m along the bike from x 0.05, from z 0.6 to 0.9, 0.13 m each side of the centre line.
const TANK: (f32, f32, f32, f32, f32) = (0.05, 0.40, 0.6, 0.9, 0.13);

fn section(half_width: f32) -> Section {
    let (x0, z0, step, nx, nz) = (-0.5, 0.0, 0.03, 40, 40);
    let mut cells = vec![0_u16; nx * nz];
    for i in 0..nx {
        for j in 0..nz {
            let (x, z) = (x0 + (i as f32 + 0.5) * step, z0 + (j as f32 + 0.5) * step);
            if (TANK.0..TANK.1).contains(&x) && (TANK.2..TANK.3).contains(&z) {
                cells[i * nz + j] = (half_width * 1000.0) as u16;
            }
        }
    }
    Section { x0, z0, step, nx, nz, half_width_mm: cells }
}

fn skeleton() -> Skeleton {
    let bone = |name: &str, parent: Option<usize>, t: Vec3| Bone {
        name: name.into(),
        parent,
        rest: Transform::from_rt(Quat::IDENTITY, t),
    };
    Skeleton::new(vec![
        bone("root", None, Vec3::ZERO),
        bone("pelvis", Some(0), Vec3::new(0.0, 0.0, 0.95)),
        bone("spine_01", Some(1), Vec3::new(0.05, 0.0, 0.1)),
        bone("spine_03", Some(2), Vec3::new(0.15, 0.0, 0.2)),
        bone("thigh_l", Some(1), Vec3::new(0.0, 0.12, 0.0)),
        bone("calf_l", Some(4), Vec3::new(0.25, 0.0, -0.30)),
        bone("foot_l", Some(5), Vec3::new(-0.10, 0.0, -0.35)),
        bone("thigh_r", Some(1), Vec3::new(0.0, -0.12, 0.0)),
        bone("calf_r", Some(7), Vec3::new(0.25, 0.0, -0.30)),
        bone("foot_r", Some(8), Vec3::new(-0.10, 0.0, -0.35)),
        bone("PV_leg_l", Some(0), Vec3::new(0.6, 0.12, 0.65)),
        bone("PV_leg_r", Some(0), Vec3::new(0.6, -0.12, 0.65)),
    ])
    .unwrap()
}

fn body(bone: &str, radius: f32, half_length: f32, axis: Vec3) -> BodyFile {
    BodyFile {
        bone: bone.into(),
        mass: 5.0,
        radius,
        half_length,
        centre: (axis * half_length).to_array(),
        axis: axis.to_array(),
    }
}

/// The thigh and the calf of each leg (their capsules run along the bone to its child) and the spine's.
fn ragdoll() -> RagdollFile {
    let thigh = Vec3::new(0.25, 0.0, -0.30).normalize();
    let calf = Vec3::new(-0.10, 0.0, -0.35).normalize();
    RagdollFile {
        bodies: vec![
            body("thigh_l", 0.09, 0.195, thigh),
            body("calf_l", 0.08, 0.18, calf),
            body("thigh_r", 0.09, 0.195, thigh),
            body("calf_r", 0.08, 0.18, calf),
            body("spine_01", 0.12, 0.05, Vec3::Z),
            body("spine_03", 0.15, 0.07, Vec3::Z),
            body("pelvis", 0.125, 0.055, Vec3::Y),
        ],
        joints: Vec::new(),
        contact_friction: 0.6,
        linear_damping: 0.0,
        angular_damping: 0.0,
        joint_damping: 0.0,
        falls: Falls::default(),
    }
}

fn resolved() -> Resolved {
    let limb = |root, mid, tip, pole, release| Limb {
        chain: TwoBoneChain { root, mid, tip },
        pole,
        target: 0,
        offset: Vec3::ZERO,
        frame: None,
        anchor: None,
        on_torso: false,
        release,
    };
    Resolved {
        limbs: vec![limb(THIGH_L, CALF_L, FOOT_L, POLE_L, Some(0)), limb(THIGH_R, CALF_R, FOOT_R, POLE_R, Some(1))],
        shift_bone: PELVIS,
        torso: Some(TorsoBones { lean: vec![SPINE_01, SPINE_03], head: Vec::new(), head_share: 0.8 }),
        seat: None,
        clearance: None,
    }
}

fn frame() -> Frame {
    Frame::new(&Transform::IDENTITY, Vec3::ZERO)
}

fn clearance(half_width: f32) -> Clearance {
    Clearance::new(section(half_width), &ragdoll(), &resolved(), &skeleton()).expect("a clearance")
}

/// The rider's rest pose with the legs solved to put each foot at `foot` (mirrored for the right) with the pole at `pole` (likewise).
fn legs(foot: Vec3, pole: Vec3) -> Vec<Transform> {
    let skeleton = skeleton();
    let mut local = skeleton.rest_pose().local;
    for (chain, side) in [((THIGH_L, CALF_L, FOOT_L), 1.0), ((THIGH_R, CALF_R, FOOT_R), -1.0)] {
        let mirrored = |p: Vec3| Vec3::new(p.x, p.y * side, p.z);
        let chain = TwoBoneChain { root: chain.0, mid: chain.1, tip: chain.2 };
        apply_two_bone(&skeleton, &mut local, chain, mirrored(foot), mirrored(pole), 1.0, None).unwrap();
    }
    let mut out = Vec::new();
    skeleton.component_transforms(&local, &mut out).unwrap();
    out
}

#[test]
fn the_section_reads_its_cells_and_takes_a_point_near_a_wall_for_being_at_it() {
    let section = section(0.13);
    assert!((section.half_width(0.2, 0.75) - 0.13).abs() < 1e-6, "inside the tank");
    assert_eq!(section.half_width(-0.3, 0.75), 0.0, "behind it");
    assert_eq!(section.half_width(0.2, 0.2), 0.0, "below it");
    assert_eq!(section.half_width(50.0, 0.75), 0.0, "off the grid");
    assert_eq!(section.half_width(f32::NAN, 0.75), 0.0);
    // Within half a cell of the tank's end the wall is still felt.
    assert!(section.half_width(TANK.1 + 0.01, 0.75) > 0.0);
    assert_eq!(section.half_width(TANK.1 + 0.06, 0.75), 0.0);
    assert!(section.is_valid());
    assert!(!Section { half_width_mm: vec![0; 3], ..section.clone() }.is_valid(), "the cells must fill the grid");
    assert!(!Section { step: 0.0, ..section }.is_valid());
}

#[test]
fn a_rider_clear_of_the_bike_is_left_exactly_as_he_is() {
    // The knees and thighs outside a narrow tank (9 cm each side; the thigh's skin is 9 cm from its axis): the feet 0.40 m out.
    let pose = legs(Vec3::new(0.1, 0.40, 0.30), Vec3::new(0.6, 0.40, 0.65));
    let clearance = clearance(0.09);
    for (n, limb) in resolved().limbs.iter().enumerate() {
        assert_eq!(clearance.limb_pole((n, limb), &pose, &frame()), None);
    }
    assert_eq!(clearance.torso(&pose, &frame()), Vec3::ZERO);
    assert_eq!(clearance.deepest(&pose, &frame()), 0.0);
}

#[test]
fn a_knee_inside_the_tank_is_swung_out_to_its_own_side_and_the_foot_stays_on_its_peg() {
    let clearance = clearance(0.13);
    let foot = Vec3::new(0.1, 0.12, 0.30);
    let pole = Vec3::new(0.6, 0.12, 0.65);
    let before = legs(foot, pole);
    assert!(clearance.deepest(&before, &frame()) > 0.05, "{}", clearance.deepest(&before, &frame()));
    let skeleton = skeleton();
    let resolved = resolved();
    for (n, limb) in resolved.limbs.iter().enumerate() {
        let side = if n == 0 { 1.0 } else { -1.0 };
        let knee = before[limb.chain.mid].translation;
        let moved = clearance.limb_pole((n, limb), &before, &frame()).expect("this knee is inside");
        // Outward is the limb's own side, and by no more than the bound.
        assert!((moved.y - knee.y) * side > 0.01, "limb {n}: {knee:?} to {moved:?}");
        assert!(moved.distance(knee) <= MAX_PUSH_M + 1e-5);
        // Solved again with that pole, the leg's foot is still on its peg and the knee is shallower.
        let mut local = skeleton.rest_pose().local;
        let chain = limb.chain;
        let (target, pole) = (Vec3::new(foot.x, foot.y * side, foot.z), Vec3::new(pole.x, pole.y * side, pole.z));
        apply_two_bone(&skeleton, &mut local, chain, target, pole, 1.0, None).unwrap();
        apply_two_bone(&skeleton, &mut local, chain, target, moved, 1.0, None).unwrap();
        let mut after = Vec::new();
        skeleton.component_transforms(&local, &mut after).unwrap();
        assert!(after[chain.tip].translation.distance(target) < 0.01, "limb {n}: the foot left its peg");
        let deeper = |pose: &[Transform]| depth_of_limb(&clearance, pose, n);
        assert!(deeper(&after) < 0.75 * deeper(&before), "limb {n}: {} then {}", deeper(&before), deeper(&after));
    }
}

/// The deepest point of limb `n`'s capsules in `pose`.
fn depth_of_limb(clearance: &Clearance, pose: &[Transform], n: usize) -> f32 {
    let frame = frame();
    let mut worst = 0.0_f32;
    for guard in clearance.guards.iter().filter(|g| matches!(g.group, Group::Limb { limb, .. } if limb == n)) {
        for (_, point) in Clearance::points(guard, pose, &frame) {
            worst = worst.max(depth(&clearance.section, point, guard.radius).0);
        }
    }
    worst
}

#[test]
fn a_push_is_held_to_the_bound_and_a_point_near_the_hip_is_not_chased() {
    // A tank as wide as a barrel: the knee cannot be asked to move more than the bound.
    let wide = clearance(0.45);
    let pose = legs(Vec3::new(0.1, 0.12, 0.30), Vec3::new(0.6, 0.12, 0.65));
    let limb = resolved().limbs[0];
    let knee = pose[CALF_L].translation;
    let moved = wide.limb_pole((0, &limb), &pose, &frame()).expect("inside");
    assert!(moved.distance(knee) <= MAX_PUSH_M + 1e-5, "{}", moved.distance(knee));
}

#[test]
fn a_chest_on_one_side_of_the_tank_is_moved_over_and_one_above_the_centre_line_is_not() {
    let clearance = clearance(0.13);
    let mut local = skeleton().rest_pose().local;
    let mut pose = Vec::new();
    // The chest 0.1 m to the left of the centre line, at the tank's height: on one side of it, so moved to the left.
    local[SPINE_01].translation = Vec3::ZERO;
    local[SPINE_03].translation = Vec3::new(0.2, 0.10, -0.25);
    skeleton().component_transforms(&local, &mut pose).unwrap();
    let over = clearance.torso(&pose, &frame());
    assert!(over.y > 0.0 && over.length() <= MAX_TORSO_PUSH_M + 1e-6, "{over:?}");
    // Straight over the centre line the sideways push has nothing to cure: it is left to the chest's own height.
    local[SPINE_03].translation = Vec3::new(0.2, 0.0, -0.25);
    skeleton().component_transforms(&local, &mut pose).unwrap();
    assert_eq!(clearance.torso(&pose, &frame()), Vec3::ZERO);
}

#[test]
fn only_the_limbs_and_the_torsos_capsules_are_guarded_and_a_damaged_section_guards_nothing() {
    let clearance = clearance(0.13);
    // Two legs' thighs and calves and the torso's second bone: the pelvis, which sits on the seat, and the torso's first bone are not.
    assert_eq!(clearance.guard_count(), 5);
    assert!(Clearance::new(Section { nx: 3, ..section(0.13) }, &ragdoll(), &resolved(), &skeleton()).is_none());
    let none = RagdollFile { bodies: Vec::new(), ..ragdoll() };
    assert!(Clearance::new(section(0.13), &none, &resolved(), &skeleton()).is_none());
}
