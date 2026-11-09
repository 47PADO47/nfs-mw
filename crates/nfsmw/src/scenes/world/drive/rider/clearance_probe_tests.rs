//! The numeric check of the rider against his bike's body (`docs/specs/ride6-rider-clearance.md`, section 3): a measuring tool, not a test.
//! It poses the rider of a scratch mod folder on one bike at lean angles of 0 to 60 degrees to each side, as the game does, and prints how far his
//! skin is inside the bike's section with the keeping-out off and on: the deepest capsule point, and the deepest skin vertex per group of bones.
//!
//! `R6_CLIP_FILES=<mod>/files R6_CLIP_CAR=<ID> [R6_CLIP_GRAPH=graph_race.json] [R6_CLIP_MAX_LEAN_DEG=55] [R6_CLIP_SPEED=20]
//! cargo test -p nfsmw clearance_probe -- --ignored --nocapture`
//!
//! The body's shift is the largest the rig allows (`attach.json` `inertia`, forward and sideways) times the lean, to the outside and to the
//! inside of the lean in turn, which holds every state the physics' own rider can reach.

use blackbox_skeleton::{Graph, Pack, skin_matrices, skin_vertices};
use glam::{Mat4, Vec3};

use super::attach::Attach;
use super::clearance::{Clearance, TOLERANCE_M};
use super::contacts::Contacts;
use super::fall_file::RagdollFile;
use super::params::{RiderSample, Smoothed};
use super::pose::{self, Frame, Goals, Resolved};

/// A vertex deeper than this plus [`TOLERANCE_M`] (m) counts as a clip in the table: the cell's own slack and the suit's padding.
const PROBE_SLACK_M: f32 = 0.01;
/// The bones whose skin is meant to be in contact with the bike: the seat, the grips and the pegs.
const IN_CONTACT: [&str; 8] = ["pelvis", "hand", "foot", "ball", "index", "middle", "ring", "pinky"];

fn read(path: &str) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

struct Rig {
    attach: Attach,
    pack: Pack,
    graph: Graph,
    ragdoll: RagdollFile,
}

fn load(files: &str, car: &str, graph: &str) -> Rig {
    let rig = format!("{files}/RIDER");
    let mut attach = Attach::parse(&read(&format!("{rig}/attach.json"))).unwrap();
    let own = format!("{files}/CARS/{car}/RIDER/contacts.json");
    attach = attach.with_contacts(&Contacts::parse(&read(&own)).unwrap()).unwrap();
    Rig {
        attach,
        pack: Pack::parse(&read(&format!("{rig}/rider.bbsk"))).unwrap(),
        graph: serde_json::from_slice(&read(&format!("{rig}/{graph}"))).unwrap(),
        ragdoll: RagdollFile::parse(&read(&format!("{rig}/ragdoll.json"))).unwrap(),
    }
}

/// The pose at `angle` degrees of lean to `side` (1 right, -1 left), the body shifted to `shift_side` of it.
fn pose_at(
    rig: &Rig,
    keep_out: bool,
    (side, angle, shift_side): (f32, f32, f32),
    max_lean_deg: f32,
) -> (Vec<blackbox_skeleton::Transform>, Frame) {
    let attach = &rig.attach;
    let skeleton = &rig.pack.skeleton;
    let mut resolved = Resolved::new(attach, skeleton).unwrap();
    resolved.seat = super::seat::Seat::new(attach, &rig.pack);
    if keep_out {
        resolved.clearance = Clearance::of_rig(attach, Some(&rig.ragdoll), &resolved, skeleton);
    }
    let frame = Frame::new(&attach.rider_root_in_bike.transform(), Vec3::ZERO);
    let fraction = (angle / max_lean_deg).min(1.0);
    let speed: f32 = std::env::var("R6_CLIP_SPEED").ok().and_then(|v| v.parse().ok()).unwrap_or(20.0);
    let sample = RiderSample {
        lean: side * angle.to_radians(),
        max_lean: max_lean_deg.to_radians(),
        bike_lean: side * fraction,
        rider_lean: side * fraction,
        speed,
        front_down: true,
        rear_down: true,
        // The physics' axes: x right, y up, z forward; the shift is held to the rig's range anyway.
        offset: Vec3::new(-shift_side * attach.inertia.max[1] * fraction, 0.0, attach.inertia.max[0] * fraction),
        ..RiderSample::default()
    };
    let mut smoothed = Smoothed::default();
    for _ in 0..240 {
        smoothed.step(&sample, &attach.inertia, 1.0 / 60.0);
    }
    let mut targets = pose::rider_targets(attach, &frame, None);
    pose::roll_wrist(attach, &mut targets, smoothed.values().wrist);
    let body = pose::body(&frame, smoothed.shift(), smoothed.rotation());
    let params = super::param_values(&rig.graph.param_names, &smoothed.values());
    let goals = Goals { targets: &targets, body, frame: &frame, release: smoothed.values().foot_release };
    let component = pose::animate(skeleton, &rig.pack.clips, &rig.graph, (&params, 0.0), &resolved, &goals).unwrap();
    (component, frame)
}

/// The deepest skin vertex of each group of bones (the torso and head, the arms, the legs), metres, in `component`.
fn skin_depths(
    rig: &Rig,
    clearance: &Clearance,
    component: &[blackbox_skeleton::Transform],
    frame: &Frame,
) -> [f32; 3] {
    let skeleton = &rig.pack.skeleton;
    let (mut matrices, mut positions, mut normals): (Vec<Mat4>, Vec<Vec3>, Vec<Vec3>) =
        (Vec::new(), Vec::new(), Vec::new());
    let mut worst = [0.0_f32; 3];
    for mesh in &rig.pack.meshes {
        skin_matrices(component, &mesh.bone_map, &mesh.inverse_bind, &mut matrices).unwrap();
        let lod = &mesh.lods[0];
        skin_vertices(&lod.vertices, &matrices, &mut positions, &mut normals);
        for (vertex, position) in lod.vertices.iter().zip(&positions) {
            let strongest = (0..vertex.influences.weights.len()).fold(0, |best, k| {
                if vertex.influences.weights[k] > vertex.influences.weights[best] { k } else { best }
            });
            let Some(bone) = mesh.bone_map[usize::from(vertex.influences.bones[strongest])] else { continue };
            let name = skeleton.bones()[bone].name.as_str();
            if IN_CONTACT.iter().any(|prefix| name.starts_with(prefix)) {
                continue;
            }
            let group = match (name.contains("thigh") || name.contains("calf"), name.contains("arm")) {
                (true, _) => 2,
                (_, true) => 1,
                _ => 0,
            };
            let depth = clearance.vertex_depth(frame.to_bike.transform_point3(*position));
            worst[group] = worst[group].max(depth);
        }
    }
    worst
}

#[test]
#[ignore = "needs a scratch mod folder (R6_CLIP_FILES)"]
fn clearance_probe() {
    let (Ok(files), Ok(car)) = (std::env::var("R6_CLIP_FILES"), std::env::var("R6_CLIP_CAR")) else { return };
    let graph = std::env::var("R6_CLIP_GRAPH").unwrap_or_else(|_| "graph.json".into());
    let max_lean: f32 = std::env::var("R6_CLIP_MAX_LEAN_DEG").ok().and_then(|v| v.parse().ok()).unwrap_or(55.0);
    let rig = load(&files, &car, &graph);
    let reference = {
        let resolved = Resolved::new(&rig.attach, &rig.pack.skeleton).unwrap();
        Clearance::of_rig(&rig.attach, Some(&rig.ragdoll), &resolved, &rig.pack.skeleton)
    };
    let Some(reference) = reference else { panic!("{car} has no body section or the rider no ragdoll") };
    println!("{car}: tolerance {TOLERANCE_M} m, a vertex deeper than {} m is a clip", TOLERANCE_M + PROBE_SLACK_M);
    println!("side angle shift | off: capsule torso arms legs | on: capsule torso arms legs   (cm)");
    for side in [-1.0, 1.0] {
        for step in 0..=12 {
            let angle = step as f32 * 5.0;
            for shift_side in [-1.0, 1.0] {
                let mut row = String::new();
                for keep_out in [false, true] {
                    let (component, frame) = pose_at(&rig, keep_out, (side, angle, shift_side), max_lean);
                    let capsule = reference.deepest(&component, &frame);
                    let [torso, arms, legs] = skin_depths(&rig, &reference, &component, &frame);
                    row += &format!(
                        " | {:5.1} {:5.1} {:5.1} {:5.1}",
                        capsule * 100.0,
                        torso * 100.0,
                        arms * 100.0,
                        legs * 100.0
                    );
                }
                println!("{side:+.0} {angle:3.0} {shift_side:+.0}{row}");
            }
        }
    }
}
