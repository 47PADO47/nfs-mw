//! The rows of the front-end screen dump (`parity/fe_dump.rs`): one per object of a screen's package, read from the
//! tree the presenter is handed this frame and from the runtime's state, and one per node the host added to that tree.
//! An object the tree does not hold, or holds but does not draw, says so: the dump is what is drawn, not what the
//! package file holds.

use std::sync::Arc;

use blackbox_feng::package::ObjectKind;
use blackbox_feng::{NodeKind, ObjectRef, UiNode, UiTree};
use serde_json::{Value, json};

use super::{Screen, Screens};
use crate::parity::fe_dump;

fn hex(value: u32) -> String {
    format!("{value:#010x}")
}

/// The package file's number for an object type.
fn type_id(kind: ObjectKind) -> u32 {
    match kind {
        ObjectKind::Image => 1,
        ObjectKind::String => 2,
        ObjectKind::Group => 5,
        ObjectKind::Movie => 7,
        ObjectKind::ColoredImage => 9,
        ObjectKind::SimpleImage => 11,
        ObjectKind::MultiImage => 12,
        ObjectKind::Other(id) => id,
    }
}

fn round(v: f32) -> f64 {
    (f64::from(v) * 1000.0).round() / 1000.0
}

impl Screens {
    /// Hands the frame's presented screens (file name and the tree the presenter got, bottom first) to the dump, and
    /// writes a snapshot of each one that is due. The frame list starts with the packages other systems drew (the HUD).
    pub fn dump_frame(&self, presented: &[(String, Arc<UiTree>)]) {
        let busy: Vec<(String, bool)> = presented
            .iter()
            .map(|(name, _)| (name.clone(), self.screen_named(name).is_some_and(|s| self.busy(s))))
            .collect();
        let mut frame = fe_dump::take_others();
        // As the original counts its draw calls: the drawn objects that are no groups.
        let drawn = |tree: &UiTree| {
            tree.draw_order
                .iter()
                .filter(|&&i| !matches!(tree.nodes[i].kind, NodeKind::Group | NodeKind::Other))
                .count()
        };
        frame.extend(presented.iter().map(|(name, tree)| (name.clone(), drawn(tree))));
        for (at, due) in fe_dump::frame(&busy) {
            let (name, tree) = &presented[at];
            let Some(screen) = self.screen_named(name) else { continue };
            fe_dump::write(name, due, &frame, self.rows(screen, tree));
        }
    }

    fn screen_named(&self, name: &str) -> Option<&Screen> {
        self.stack.iter().rev().find(|s| s.name == *name)
    }

    /// Whether an object of the screen plays a play-once script that has not reached its end.
    fn busy(&self, screen: &Screen) -> bool {
        let Some(def) = self.runtime.package(screen.id) else { return false };
        (0..def.objects.len()).any(|index| {
            let Some(state) = self.runtime.object(ObjectRef { package: screen.id, index }) else { return false };
            let Some(script) = state.current.and_then(|s| state.scripts.get(s)) else { return false };
            script.end_behaviour() == 0 && state.time < script.length
        })
    }

    fn rows(&self, screen: &Screen, tree: &UiTree) -> Vec<Value> {
        let Some(def) = self.runtime.package(screen.id) else { return Vec::new() };
        let mut drawn = vec![false; tree.nodes.len()];
        for &i in &tree.draw_order {
            if let Some(slot) = drawn.get_mut(i) {
                *slot = true;
            }
        }
        let mut in_tree = vec![None; def.objects.len()];
        let mut rows = Vec::with_capacity(tree.nodes.len());
        for (k, node) in tree.nodes.iter().enumerate() {
            let owned = def.objects.get(node.index).is_some_and(|o| o.guid == node.guid);
            if owned && in_tree[node.index].is_none() {
                in_tree[node.index] = Some(k);
                continue;
            }
            rows.push(added_row(node, drawn[k]));
        }
        let mut objects = Vec::with_capacity(def.objects.len());
        for (index, object) in def.objects.iter().enumerate() {
            let Some(state) = self.runtime.object(ObjectRef { package: screen.id, index }) else { continue };
            let parent = object.parent.and_then(|g| def.objects.iter().find(|o| o.guid == g));
            let script = state
                .current
                .and_then(|s| state.scripts.get(s))
                .map(|s| json!({"id": hex(s.id), "time": state.time, "length": s.length, "mode": s.end_behaviour()}));
            let data = &state.data;
            let (pos, pivot, rot, size) = (data.position(), data.pivot(), data.rotation(), data.size());
            let mut row = json!({
                "name_hash": hex(object.name_hash),
                "guid": hex(object.guid),
                "type": type_id(object.kind),
                "parent": parent.map(|p| hex(p.name_hash)),
                "parent_guid": object.parent.map(hex),
                "flags": hex(object.flags),
                "hidden": state.hidden,
                "colour": data.colour_rgba(),
                "pos": [round(pos.x), round(pos.y), round(pos.z)],
                "pivot": [round(pivot.x), round(pivot.y), round(pivot.z)],
                "rot": [round(rot.x), round(rot.y), round(rot.z), round(rot.w)],
                "size": [round(size.x), round(size.y), round(size.z)],
                "label": object.string.as_ref().map(|_| hex(state.label)),
                "script": script,
                "host_added": false,
            });
            if object.kind.is_image() {
                row["uv"] = json!(data.uv().map(round));
            }
            match in_tree[index].map(|k| (&tree.nodes[k], drawn[k])) {
                Some((node, is_drawn)) => {
                    row["in_tree"] = json!(true);
                    row["visible"] = json!(node.visible);
                    row["drawn"] = json!(is_drawn);
                    row["world"] = json!([round(node.position.x), round(node.position.y)]);
                    row["text"] = json!(node.text);
                    if let Some(corners) = node.corner_colours {
                        row["corners"] = json!(corners);
                    }
                    row["world_colour"] = json!(node.world_colour);
                    row["z"] = json!(round(node.z));
                    if let NodeKind::Image { texture, .. } = node.kind {
                        row["texture"] = json!(hex(texture));
                    }
                }
                None => {
                    row["in_tree"] = json!(false);
                    row["visible"] = json!(false);
                    row["drawn"] = json!(false);
                }
            }
            objects.push(row);
        }
        objects.extend(rows);
        objects
    }
}

/// A node the host put into the tree that is no object of the package (footer hints, generated rows).
fn added_row(node: &UiNode, drawn: bool) -> Value {
    let (pos, size) = (node.local_position, node.size);
    let mut row = json!({
        "name_hash": hex(node.name_hash),
        "guid": hex(node.guid),
        "type": match node.kind { NodeKind::Image { .. } => 1, NodeKind::Text { .. } => 2, NodeKind::Group => 5, NodeKind::Other => 0 },
        "colour": node.colour,
        "pos": [round(pos.x), round(pos.y), round(pos.z)],
        "size": [round(size.x), round(size.y), round(size.z)],
        "world": [round(node.position.x), round(node.position.y)],
        "text": node.text,
        "visible": node.visible,
        "drawn": drawn,
        "in_tree": true,
        "host_added": true,
    });
    if let NodeKind::Image { texture, .. } = node.kind {
        row["texture"] = json!(hex(texture));
    }
    row
}
