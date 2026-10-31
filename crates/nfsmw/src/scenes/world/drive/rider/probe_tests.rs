//! A measuring tool, not a test: poses the rider of a scratch mod folder on one bike exactly as the game does and writes where
//! its bones and skin end up (in the bike skeleton's frame) to a text file, for the Python plots of the fitting work.
//!
//! `R6_PROBE_FILES=<mod>/files R6_PROBE_CAR=<ID> R6_PROBE_OUT=<file> cargo test -p nfsmw probe -- --ignored --nocapture`

use std::fmt::Write as _;

use blackbox_skeleton::{Graph, Pack, skin_matrices, skin_vertices};
use glam::{Mat4, Vec3};

use super::attach::Attach;
use super::contacts::Contacts;
use super::params::Values;
use super::pose::{self, Body, Frame, Goals, Resolved};

fn read(path: &str) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
#[ignore = "needs a scratch mod folder (R6_PROBE_FILES)"]
fn probe() {
    let (Ok(files), Ok(car), Ok(out)) =
        (std::env::var("R6_PROBE_FILES"), std::env::var("R6_PROBE_CAR"), std::env::var("R6_PROBE_OUT"))
    else {
        return;
    };
    let params_in: Vec<f32> = std::env::var("R6_PROBE_PARAMS")
        .ok()
        .map(|s| s.split(',').filter_map(|v| v.parse().ok()).collect())
        .unwrap_or_default();
    let rig = format!("{files}/RIDER");
    let mut attach = Attach::parse(&read(&format!("{rig}/attach.json"))).unwrap();
    let own = format!("{files}/CARS/{car}/RIDER/contacts.json");
    if std::path::Path::new(&own).exists() {
        attach = attach.with_contacts(&Contacts::parse(&read(&own)).unwrap()).unwrap();
    }
    let pack = Pack::parse(&read(&format!("{rig}/rider.bbsk"))).unwrap();
    let graph: Graph = serde_json::from_slice(&read(&format!("{rig}/graph.json"))).unwrap();
    let mut resolved = Resolved::new(&attach, &pack.skeleton).unwrap();
    resolved.seat = super::seat::Seat::new(&attach, &pack);
    // The model's frame is the skeleton's frame for this probe (no offset): the plots use the skeleton's own.
    let frame = Frame::new(&attach.rider_root_in_bike.transform(), Vec3::ZERO);
    let targets = pose::rider_targets(&attach, &frame, None);
    let values = Values { speed_kmh: params_in.first().copied().unwrap_or(0.0), ..Values::default() };
    let names = &graph.param_names;
    let params = super::param_values(names, &values);
    let goals = Goals { targets: &targets, body: Body::REST, frame: &frame };
    let mut text = String::new();
    if let Some(seat) = resolved.seat.take() {
        // The pose with the rider at the socket: how far above the seat's surface his lowest seat point is.
        let bare = pose::animate(&pack.skeleton, &pack.clips, &graph, (&params, 0.0), &resolved, &goals).unwrap();
        writeln!(text, "clearance {:?}", seat.clearance(&bare, &frame.to_bike)).unwrap();
        resolved.seat = Some(seat);
    }
    let component = pose::animate(&pack.skeleton, &pack.clips, &graph, (&params, 0.0), &resolved, &goals).unwrap();
    for limb in &resolved.limbs {
        let goal = pose::tip_goal(limb, &targets[limb.target]).0;
        let miss = component[limb.chain.tip].translation.distance(goal);
        writeln!(text, "miss {} {:.2} cm", pack.skeleton.bones()[limb.chain.tip].name, miss * 100.0).unwrap();
    }
    let to_bike = |p: Vec3| frame.to_model.transform_point3(p);
    for (i, bone) in pack.skeleton.bones().iter().enumerate() {
        let p = to_bike(component[i].translation);
        writeln!(text, "bone {} {:.5} {:.5} {:.5}", bone.name, p.x, p.y, p.z).unwrap();
        let q = frame.to_model.to_scale_rotation_translation().1 * component[i].rotation;
        writeln!(text, "rot {} {:.5} {:.5} {:.5} {:.5}", bone.name, q.x, q.y, q.z, q.w).unwrap();
    }
    for (n, target) in attach.targets.iter().enumerate() {
        let t = to_bike(targets[n].position);
        writeln!(text, "target {} {:.5} {:.5} {:.5}", target.name, t.x, t.y, t.z).unwrap();
        let q = target.rotation.unwrap_or([0.0, 0.0, 0.0, 1.0]);
        writeln!(text, "trot {} {:.5} {:.5} {:.5} {:.5}", target.name, q[0], q[1], q[2], q[3]).unwrap();
    }
    let mut matrices: Vec<Mat4> = Vec::new();
    let (mut positions, mut normals) = (Vec::new(), Vec::new());
    for mesh in &pack.meshes {
        skin_matrices(&component, &mesh.bone_map, &mesh.inverse_bind, &mut matrices).unwrap();
        let lod = &mesh.lods[0];
        skin_vertices(&lod.vertices, &matrices, &mut positions, &mut normals);
        for (v, p) in lod.vertices.iter().zip(&positions) {
            let p = to_bike(*p);
            let (mut best, mut weight) = (0, 0.0);
            for (slot, w) in v.influences.weights.iter().enumerate() {
                if *w > weight {
                    (best, weight) = (slot, *w);
                }
            }
            let bone = mesh.bone_map[usize::from(v.influences.bones[best])]
                .map_or("none", |b| pack.skeleton.bones()[b].name.as_str());
            writeln!(text, "v {} {:.4} {:.4} {:.4} {} {:.2}", mesh.name, p.x, p.y, p.z, bone, weight).unwrap();
        }
    }
    std::fs::write(out, text).unwrap();
}
