//! Which parts of a bike steer, and which of those also slide with the fork: the front end is told from the rest of the
//! frame by the bike's own skeleton.
//!
//! The chassis is one skinned mesh in the bind pose; the game draws a car as a body and wheels, so before this module the
//! whole chassis was baked into the body solid and only the wheel turned. A real front end has two moving groups: the
//! **bars** (handlebar, levers, grips, hand guards, bar pad, headlamp and front fairing, gauges, the upper triple clamp)
//! turn with the steering and stay where they are along the steering axis, and the **fork** (the legs, the front fender,
//! the front caliper, the hub) turns *and* slides along the axis as the suspension works. The rule is data, in three
//! steps (`docs/modding/ride6-bikes.md`, "The steered front end"):
//!
//! 1. **Bones.** The bone hierarchy comes from the node tree of the exported chassis (`Glb::nodes`). A bone's class is
//!    that of its nearest ancestor, itself included, that is a [`Rules::slide_roots`] bone (RIDE 6: `BONE_F_SUSP`, whose
//!    child `BONE_F_VIS_WHEEL` carries the hub) or a [`Rules::steer_roots`] bone (RIDE 6: `BONE_HANDLE`, whose children
//!    are the bar pad, the hand guards, the levers, the grips and, in the file, also `BONE_F_SUSP`). The nearest root wins,
//!    so the suspension, a child of the handle, slides and the rest of the handle's group does not. A bone listed in
//!    [`Rules::slide_followers`] slides although the hierarchy does not put it below a root (RIDE 6: `BONE_FL_CALIPER`,
//!    which sits in the front brake plane and hangs on the file's root). Every other bone is the frame.
//! 2. **Vertices.** Every vertex of a skinned primitive carries up to four joints and weights. Its share of each class is
//!    the weight on joints of that class over its total weight (a vertex with no weight is all frame).
//! 3. **Triangles.** A triangle goes to the class with the largest mean share over its three vertices; a tie goes to the
//!    class that moves least (frame, then bars, then fork). A vertex shared by two classes is copied into each.
//!
//! A part that is not skinned (a separate exhaust, say) is judged by the bone it hangs on (`parts.json` anchor). A part
//! whose mesh is skinned is split by the weights, by the joints' **names**, so a part exported with a skeleton of its own
//! (the bar pad has only `BONE_BARPAD`) is judged by the chassis hierarchy all the same.

use std::collections::HashMap;

use crate::glb::{Glb, Node, Primitive};

/// How a bone or a triangle moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Class {
    /// Stays with the frame (the body solid).
    #[default]
    Frame,
    /// Turns with the handlebar (the bars solid).
    Bars,
    /// Turns with the handlebar and slides along the steering axis with the fork (the fork solid).
    Fork,
}

impl Class {
    pub const ALL: [Class; 3] = [Class::Frame, Class::Bars, Class::Fork];

    pub fn index(self) -> usize {
        self as usize
    }
}

/// Which bones belong to which moving group, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rules {
    /// Roots of the group that steers without sliding.
    pub steer_roots: Vec<String>,
    /// Roots of the group that steers and slides; below a steer root, the nearer root wins.
    pub slide_roots: Vec<String>,
    /// Bones that slide although the hierarchy does not put them below a slide root.
    pub slide_followers: Vec<String>,
}

impl Rules {
    /// The bones of RIDE 6's chassis skeleton.
    pub fn ride6() -> Self {
        Self {
            steer_roots: vec!["BONE_HANDLE".to_owned()],
            slide_roots: vec!["BONE_F_SUSP".to_owned()],
            slide_followers: vec!["BONE_FL_CALIPER".to_owned()],
        }
    }

    fn class_of_name(&self, name: &str) -> Option<Class> {
        let listed = |list: &[String]| list.iter().any(|n| n == name);
        if listed(&self.slide_roots) {
            return Some(Class::Fork);
        }
        listed(&self.steer_roots).then_some(Class::Bars)
    }
}

/// The class of every bone of `nodes` that is not the frame, by name, and the followers (which need not be among them).
pub fn classes(nodes: &[Node], rules: &Rules) -> HashMap<String, Class> {
    let mut out: HashMap<String, Class> = rules.slide_followers.iter().map(|n| (n.clone(), Class::Fork)).collect();
    for (at, node) in nodes.iter().enumerate() {
        if node.name.is_empty() || out.contains_key(&node.name) {
            continue;
        }
        let class = nearest_root(nodes, at, rules);
        if class != Class::Frame {
            out.insert(node.name.clone(), class);
        }
    }
    out
}

/// The class of the nearest root at or above node `at`.
fn nearest_root(nodes: &[Node], mut at: usize, rules: &Rules) -> Class {
    // A well-formed tree ends at a node without a parent within `nodes.len()` steps; a cycle in a damaged file would not.
    for _ in 0..=nodes.len() {
        let node = &nodes[at];
        if let Some(class) = rules.class_of_name(&node.name) {
            return class;
        }
        match node.parent {
            Some(parent) if parent < nodes.len() => at = parent,
            _ => return Class::Frame,
        }
    }
    Class::Frame
}

/// The class of the bone named `name`.
pub fn class_named(classes: &HashMap<String, Class>, name: &str) -> Class {
    classes.get(name).copied().unwrap_or_default()
}

/// For each joint of `glb`, the class of its bone.
pub fn joint_classes(glb: &Glb, classes: &HashMap<String, Class>) -> Vec<Class> {
    (0..glb.joints.len())
        .map(|joint| glb.joint_name(joint).map_or(Class::Frame, |name| class_named(classes, name)))
        .collect()
}

/// The share of each vertex's weight on joints of each class (frame, bars, fork), each 0 to 1 and summing to 1; a vertex
/// with no weight is all frame.
pub fn shares(primitive: &Primitive, joints: &[Class]) -> Vec<[f32; 3]> {
    primitive
        .joints
        .iter()
        .zip(&primitive.weights)
        .map(|(vertex_joints, weights)| {
            let (mut sum, mut total) = ([0.0f32; 3], 0.0f32);
            for (&joint, &weight) in vertex_joints.iter().zip(weights) {
                if weight <= 0.0 {
                    continue;
                }
                total += weight;
                sum[joints.get(usize::from(joint)).copied().unwrap_or_default().index()] += weight;
            }
            if total <= 0.0 {
                return [1.0, 0.0, 0.0];
            }
            sum.map(|s| s / total)
        })
        .collect()
}

/// The triangles of `primitive` by class (indexed by [`Class::index`]). A primitive that is not skinned is all frame.
pub fn split_triangles(primitive: &Primitive, joints: &[Class]) -> [Vec<[u32; 3]>; 3] {
    let triangles = primitive.indices.as_chunks::<3>().0;
    let mut out: [Vec<[u32; 3]>; 3] = Default::default();
    if !primitive.is_skinned() {
        out[Class::Frame.index()] = triangles.to_vec();
        return out;
    }
    let shares = shares(primitive, joints);
    for &triangle in triangles {
        let mut mean = [0.0f32; 3];
        for &corner in &triangle {
            for (m, s) in mean.iter_mut().zip(shares[corner as usize]) {
                *m += s;
            }
        }
        let mut best = Class::Frame;
        for class in Class::ALL {
            if mean[class.index()] > mean[best.index()] {
                best = class;
            }
        }
        out[best.index()].push(triangle);
    }
    out
}

/// Which triangles of a primitive a solid takes.
#[derive(Debug, Clone, Copy)]
pub enum Take<'a> {
    /// Every triangle.
    All,
    /// The triangles of one class (`joints`: the class of each joint of the file).
    Of(Class, &'a [Class]),
}

impl Take<'_> {
    pub fn triangles(&self, primitive: &Primitive) -> Vec<[u32; 3]> {
        match *self {
            Take::All => primitive.indices.as_chunks::<3>().0.to_vec(),
            Take::Of(class, joints) => {
                let [frame, bars, fork] = split_triangles(primitive, joints);
                [frame, bars, fork].into_iter().nth(class.index()).unwrap_or_default()
            }
        }
    }
}

/// Where a part of the body goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// All of it goes to the solid of this class.
    Whole(Class),
    /// Its mesh is skinned: each triangle goes by its weights.
    Split,
}

/// The side of a part with the mesh `glb` that hangs on bone `anchor`.
pub fn side_of(anchor: Option<&str>, glb: &Glb, classes: &HashMap<String, Class>) -> Side {
    if glb.is_skinned() {
        return Side::Split;
    }
    Side::Whole(anchor.map_or(Class::Frame, |bone| class_named(classes, bone)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, parent: Option<usize>) -> Node {
        Node { name: name.to_owned(), parent }
    }

    /// The shape of the chassis skeleton: a base, a handle with a grip, a lever and the suspension below it, a hub below
    /// the suspension, a caliper and a swingarm hung on the base.
    fn skeleton() -> Vec<Node> {
        vec![
            node("BASE", None),
            node("BONE_HANDLE", Some(0)),
            node("BONE_GRIP", Some(1)),
            node("BONE_F_SUSP", Some(1)),
            node("BONE_HUB", Some(3)),
            node("BONE_FL_CALIPER", Some(0)),
            node("BONE_SWING", Some(0)),
            node("BONE_REAR_HUB", Some(6)),
        ]
    }

    fn class(map: &HashMap<String, Class>, name: &str) -> Class {
        class_named(map, name)
    }

    #[test]
    fn the_nearest_root_decides_so_the_suspension_slides_and_the_rest_of_the_handle_does_not() {
        let map = classes(&skeleton(), &Rules::ride6());
        assert_eq!(class(&map, "BONE_HANDLE"), Class::Bars);
        assert_eq!(class(&map, "BONE_GRIP"), Class::Bars);
        assert_eq!(class(&map, "BONE_F_SUSP"), Class::Fork);
        assert_eq!(class(&map, "BONE_HUB"), Class::Fork, "below the suspension, which is below the handle");
        assert_eq!(class(&map, "BONE_FL_CALIPER"), Class::Fork, "a follower");
        for frame in ["BASE", "BONE_SWING", "BONE_REAR_HUB", "unknown"] {
            assert_eq!(class(&map, frame), Class::Frame, "{frame}");
        }
    }

    #[test]
    fn a_skeleton_without_the_roots_has_only_the_followers_moving() {
        let nodes = vec![node("BASE", None), node("BONE_SWING", Some(0))];
        let map = classes(&nodes, &Rules::ride6());
        assert_eq!(map.len(), 1);
        assert_eq!(class(&map, "BONE_FL_CALIPER"), Class::Fork);
    }

    #[test]
    fn a_damaged_tree_with_a_cycle_ends() {
        let nodes = vec![node("A", Some(1)), node("B", Some(0))];
        let map = classes(&nodes, &Rules::ride6());
        assert_eq!(class(&map, "A"), Class::Frame);
    }

    #[test]
    fn the_rules_can_be_the_files_own() {
        let rules =
            Rules { steer_roots: vec!["BASE".into()], slide_roots: vec!["BONE_SWING".into()], slide_followers: vec![] };
        let map = classes(&skeleton(), &rules);
        assert_eq!(class(&map, "BONE_GRIP"), Class::Bars);
        assert_eq!(class(&map, "BONE_REAR_HUB"), Class::Fork);
    }

    /// A skinned primitive of five triangles over fifteen vertices, each fully weighted to one joint (0 frame, 1 bars, 2 fork).
    /// Triangle 0 is all frame, 1 all bars, 2 all fork, 3 is two corners on the fork and one on the bars (fork wins), and
    /// 4 is one corner of each class (a tie, which goes to the class that moves least).
    fn primitive() -> Primitive {
        let on = [0u16, 0, 0, 1, 1, 1, 2, 2, 2, 2, 2, 1, 0, 1, 2];
        let joints: Vec<[u16; 4]> = on.iter().map(|&j| [j, 0, 0, 0]).collect();
        let count = joints.len();
        Primitive {
            positions: vec![[0.0; 3]; count],
            normals: vec![[0.0, 1.0, 0.0]; count],
            uvs: vec![[0.0; 2]; count],
            tangents: Vec::new(),
            indices: (0..count as u32).collect(),
            material: 0,
            joints,
            weights: vec![[1.0, 0.0, 0.0, 0.0]; count],
        }
    }

    const JOINTS: [Class; 3] = [Class::Frame, Class::Bars, Class::Fork];

    #[test]
    fn a_vertex_shared_between_classes_has_the_share_of_its_weight_in_each() {
        let mut p = primitive();
        p.joints[0] = [0, 1, 2, 0];
        p.weights[0] = [0.25, 0.5, 0.25, 0.0];
        let s = shares(&p, &JOINTS);
        assert_eq!(s[0], [0.25, 0.5, 0.25]);
        assert_eq!(s[3], [0.0, 1.0, 0.0]);
        // A vertex without weight, or on a joint the table does not have, is frame.
        p.weights[1] = [0.0; 4];
        p.joints[2] = [9, 0, 0, 0];
        let s = shares(&p, &JOINTS);
        assert_eq!((s[1], s[2]), ([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]));
    }

    #[test]
    fn triangles_go_to_the_class_with_the_largest_mean_share_and_a_tie_to_the_one_that_moves_least() {
        let [frame, bars, fork] = split_triangles(&primitive(), &JOINTS);
        assert_eq!(frame, vec![[0, 1, 2], [12, 13, 14]]);
        assert_eq!(bars, vec![[3, 4, 5]]);
        assert_eq!(fork, vec![[6, 7, 8], [9, 10, 11]]);
    }

    #[test]
    fn the_three_sides_cover_every_triangle_exactly_once() {
        let p = primitive();
        for joints in [JOINTS, [Class::Fork, Class::Bars, Class::Frame], [Class::Frame; 3], [Class::Fork; 3]] {
            let parts = split_triangles(&p, &joints);
            assert_eq!(parts.iter().map(Vec::len).sum::<usize>(), p.indices.len() / 3);
            let mut all: Vec<[u32; 3]> = parts.into_iter().flatten().collect();
            all.sort_unstable();
            assert_eq!(all, p.indices.as_chunks::<3>().0.to_vec());
        }
    }

    #[test]
    fn a_primitive_that_is_not_skinned_stays_in_the_frame() {
        let mut p = primitive();
        p.joints.clear();
        p.weights.clear();
        assert!(Take::Of(Class::Fork, &JOINTS).triangles(&p).is_empty());
        assert!(Take::Of(Class::Bars, &JOINTS).triangles(&p).is_empty());
        assert_eq!(Take::Of(Class::Frame, &JOINTS).triangles(&p).len(), 5);
        assert_eq!(Take::All.triangles(&p).len(), 5);
    }

    #[test]
    fn take_selects_a_class() {
        let p = primitive();
        assert_eq!(Take::Of(Class::Bars, &JOINTS).triangles(&p), vec![[3, 4, 5]]);
        assert_eq!(Take::Of(Class::Fork, &JOINTS).triangles(&p).len(), 2);
        assert_eq!(Take::All.triangles(&p).len(), 5);
    }

    fn glb(nodes: Vec<Node>, joints: Vec<usize>, skinned: bool) -> Glb {
        let mut p = primitive();
        if !skinned {
            p.joints.clear();
            p.weights.clear();
        }
        Glb { primitives: vec![p], materials: Vec::new(), nodes, joints }
    }

    #[test]
    fn joints_are_classed_by_the_name_of_their_bone() {
        let map = classes(&skeleton(), &Rules::ride6());
        // A part exported with a skeleton of its own: joint 0 is the grip, 1 the suspension, 2 the swingarm.
        let own = vec![node("BONE_GRIP", None), node("BONE_F_SUSP", None), node("BONE_SWING", None)];
        let part = glb(own, vec![0, 1, 2], true);
        assert_eq!(joint_classes(&part, &map), vec![Class::Bars, Class::Fork, Class::Frame]);
    }

    #[test]
    fn a_skinned_part_is_split_and_a_rigid_one_goes_by_its_anchor() {
        let map = classes(&skeleton(), &Rules::ride6());
        let skinned = glb(skeleton(), vec![1, 6], true);
        assert_eq!(side_of(Some("BONE_CHASSIS"), &skinned, &map), Side::Split);
        let rigid = glb(Vec::new(), Vec::new(), false);
        assert_eq!(side_of(Some("BONE_HANDLE"), &rigid, &map), Side::Whole(Class::Bars));
        assert_eq!(side_of(Some("BONE_GRIP"), &rigid, &map), Side::Whole(Class::Bars));
        assert_eq!(side_of(Some("BONE_HUB"), &rigid, &map), Side::Whole(Class::Fork));
        assert_eq!(side_of(Some("BONE_SWING"), &rigid, &map), Side::Whole(Class::Frame));
        assert_eq!(side_of(None, &rigid, &map), Side::Whole(Class::Frame));
    }

    #[test]
    fn a_whole_bike_splits_into_three_with_every_triangle_kept() {
        let nodes = skeleton();
        let map = classes(&nodes, &Rules::ride6());
        let mut p = primitive();
        // Joints: 0 handle (bars), 1 swingarm (frame), 2 suspension (fork), 3 base (frame).
        let joint_of = [0u16, 0, 0, 2, 2, 2, 1, 1, 1, 3, 3, 3, 0, 0, 0];
        p.joints = joint_of.iter().map(|&j| [j, 0, 0, 0]).collect();
        let part = Glb { primitives: vec![p], materials: Vec::new(), nodes, joints: vec![1, 6, 3, 0] };
        let joints = joint_classes(&part, &map);
        assert_eq!(joints, vec![Class::Bars, Class::Frame, Class::Fork, Class::Frame]);
        let [frame, bars, fork] = split_triangles(&part.primitives[0], &joints);
        assert_eq!((frame.len(), bars.len(), fork.len()), (2, 2, 1));
    }
}
