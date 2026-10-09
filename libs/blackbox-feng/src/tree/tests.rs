use glam::{Vec3, Vec4};

use super::*;
use crate::hash::fe_hash_upper;
use crate::package::Package;
use crate::package::synth::*;

fn build(objects: &[Obj], resources: &[(&str, u32)]) -> (Runtime, crate::runtime::PackageId) {
    let mut rt = Runtime::new();
    let id = rt.load(Package::parse(&package("T.fng", resources, objects, &[], &[])).unwrap());
    rt.update(1.0 / 60.0);
    (rt, id)
}

fn corner(n: &UiNode, x: f32, y: f32) -> Vec3 {
    (n.world * Vec4::new(x, y, 0.0, 1.0)).truncate()
}

#[test]
fn nested_transforms_compose_and_groups_do_not_scale() {
    let mut group = Obj::group(1, fe_hash_upper("G"));
    group.position = [100.0, 50.0, 10.0];
    group.size = [5.0, 5.0, 5.0];
    let mut image = Obj::image(2, fe_hash_upper("I"));
    image.parent = 1;
    image.position = [10.0, 20.0, 5.0];
    image.size = [40.0, 20.0, 1.0];
    let (rt, id) = build(&[group, image], &[]);
    let t = rt.tree(id);
    assert_eq!(t.nodes.len(), 2);
    let n = &t.nodes[1];
    assert_eq!(n.parent, Some(0));
    // The unit quad corners land at position + parent offset +- size / 2.
    assert_eq!(corner(n, -0.5, -0.5), Vec3::new(110.0 - 20.0, 70.0 - 10.0, 15.0));
    assert_eq!(corner(n, 0.5, 0.5), Vec3::new(110.0 + 20.0, 70.0 + 10.0, 15.0));
    assert_eq!(n.z, 15.0);
}

#[test]
fn a_negative_size_mirrors() {
    let mut image = Obj::image(1, fe_hash_upper("I"));
    image.position = [0.0, 0.0, 1.0];
    image.size = [-20.0, 10.0, 1.0];
    let (rt, id) = build(&[image], &[]);
    let n = &rt.tree(id).nodes[0];
    assert_eq!(corner(n, -0.5, 0.0).x, 10.0);
    assert_eq!(corner(n, 0.5, 0.0).x, -10.0);
}

#[test]
fn colours_multiply_down_and_transparent_groups_hide_children() {
    let mut group = Obj::group(1, fe_hash_upper("G"));
    group.colour = [128, 255, 255, 128];
    group.position = [0.0, 0.0, 1.0];
    let mut a = Obj::image(2, fe_hash_upper("A"));
    a.parent = 1;
    a.position = [0.0, 0.0, 1.0];
    let mut hidden_group = Obj::group(3, fe_hash_upper("H"));
    hidden_group.colour = [255, 255, 255, 0];
    hidden_group.position = [0.0, 0.0, 1.0];
    let mut b = Obj::image(4, fe_hash_upper("B"));
    b.parent = 3;
    b.position = [0.0, 0.0, 1.0];
    let (rt, id) = build(&[group, a, hidden_group, b], &[]);
    let t = rt.tree(id);
    // The stored colour order is blue, green, red, alpha, so [128, 255, 255, 128] is red 255, blue 128.
    assert_eq!(t.nodes[1].world_colour, [255, 255, 128, 128]);
    assert!(t.nodes[1].visible);
    assert!(!t.nodes[3].visible, "under an invisible group");
    assert_eq!(t.draw_order, vec![1]);
}

#[test]
fn draw_order_is_far_to_near_and_skips_non_positive_depth() {
    let mut back = Obj::image(1, fe_hash_upper("Back"));
    back.position = [0.0, 0.0, 200.0];
    let mut front = Obj::image(2, fe_hash_upper("Front"));
    front.position = [0.0, 0.0, 10.0];
    let mut same_a = Obj::image(3, fe_hash_upper("A"));
    same_a.position = [0.0, 0.0, 50.0];
    let mut same_b = Obj::image(4, fe_hash_upper("B"));
    same_b.position = [0.0, 0.0, 50.0];
    let mut behind_camera = Obj::image(5, fe_hash_upper("Gone"));
    behind_camera.position = [0.0, 0.0, 0.0];
    let (rt, id) = build(&[front, back, same_a, same_b, behind_camera], &[]);
    let t = rt.tree(id);
    let names: Vec<u32> = t.draw_order.iter().map(|i| t.nodes[*i].guid).collect();
    assert_eq!(names, vec![1, 3, 4, 2]);
}

#[test]
fn images_carry_the_resource_texture_and_the_host_can_swap_it() {
    let mut image = Obj::image(1, fe_hash_upper("I"));
    image.resource = 0;
    image.position = [0.0, 0.0, 1.0];
    let (mut rt, id) = build(&[image], &[("Meter_Backing.tga", 1)]);
    let o = rt.find(id, fe_hash_upper("I")).unwrap();
    let NodeKind::Image { texture, uv, .. } = rt.tree(id).nodes[0].kind.clone() else { panic!("not an image") };
    assert_eq!(texture, fe_hash_upper("METER_BACKING"));
    assert_eq!(uv, [0.0, 0.0, 1.0, 1.0]);
    rt.set_texture(o, 0x1234);
    let NodeKind::Image { texture, .. } = rt.tree(id).nodes[0].kind.clone() else { panic!("not an image") };
    assert_eq!(texture, 0x1234);
}

#[test]
fn strings_carry_font_and_style() {
    let mut s = Obj::string(1, fe_hash_upper("S"), "100");
    s.resource = 0;
    s.position = [0.0, 0.0, 1.0];
    let (rt, id) = build(&[s], &[("FONT_MW_BODY.ffn", 2)]);
    let n = &rt.tree(id).nodes[0];
    assert_eq!(n.kind, NodeKind::Text { font: 0x545570c6, justification: 0, leading: 0, max_width: 0 });
    assert_eq!(n.text.as_deref(), Some("100"));
}

#[test]
fn hidden_objects_and_pc_hidden_flags_do_not_draw() {
    let mut a = Obj::image(1, fe_hash_upper("A"));
    a.position = [0.0, 0.0, 1.0];
    let mut b = Obj::image(2, fe_hash_upper("B"));
    b.position = [0.0, 0.0, 1.0];
    let (mut rt, id) = build(&[a, b], &[]);
    let o = rt.find(id, fe_hash_upper("A")).unwrap();
    rt.set_hidden(o, true);
    assert_eq!(rt.tree(id).draw_order, vec![1]);
}

#[test]
fn multi_images_carry_their_mask_and_the_host_can_turn_it() {
    let mut multi = Obj::multi(1, fe_hash_upper("Bar"), 0xABCD);
    multi.position = [0.0, 0.0, 1.0];
    let plain = Obj::image(2, fe_hash_upper("Plain"));
    let (mut rt, id) = build(&[multi, plain], &[]);
    let bar = rt.find(id, fe_hash_upper("Bar")).unwrap();
    let NodeKind::Image { mask, mask_rotation, .. } = rt.tree(id).nodes[0].kind.clone() else { panic!("not an image") };
    assert_eq!((mask, mask_rotation), (Some(0xABCD), [0.5, 0.5, 0.0]));
    rt.set_mask_rotation(bar, 123.5);
    let NodeKind::Image { mask_rotation, .. } = rt.tree(id).nodes[0].kind.clone() else { panic!("not an image") };
    assert_eq!(mask_rotation, [0.5, 0.5, 123.5]);
    let NodeKind::Image { mask, .. } = rt.tree(id).nodes[1].kind.clone() else { panic!("not an image") };
    assert_eq!(mask, None, "a plain image has no mask");
}

#[test]
fn a_group_turns_about_the_pivot_the_host_sets() {
    let mut group = Obj::group(1, fe_hash_upper("G"));
    group.position = [100.0, 50.0, 1.0];
    let mut at_pivot = Obj::image(2, fe_hash_upper("AtPivot"));
    at_pivot.parent = 1;
    at_pivot.position = [20.0, 0.0, 1.0];
    let mut beside = Obj::image(3, fe_hash_upper("Beside"));
    beside.parent = 1;
    beside.position = [20.0, 10.0, 1.0];
    let (mut rt, id) = build(&[group, at_pivot, beside], &[]);
    let g = rt.find(id, fe_hash_upper("G")).unwrap();
    rt.set_pivot_xy(g, 20.0, 0.0);
    rt.set_rotation_z(g, 90f32.to_radians());
    let t = rt.tree(id);
    // The pivot is the one point of the group that stays where `position + pivot` puts it.
    let (a, b) = (t.nodes[1].position, t.nodes[2].position);
    assert!((a - Vec3::new(120.0, 50.0, 2.0)).length() < 1e-3, "{a:?}");
    // A positive rotation turns clockwise on the screen (y down): 10 units below the pivot lands to its left.
    assert!((b - Vec3::new(110.0, 50.0, 2.0)).length() < 1e-3, "{b:?}");
}

#[test]
fn the_host_moves_the_window_of_the_mask() {
    let multi = Obj::multi(1, fe_hash_upper("Piece"), 0xABCD);
    let (mut rt, id) = build(&[multi], &[]);
    let piece = rt.find(id, fe_hash_upper("Piece")).unwrap();
    let mask_uv = |rt: &Runtime| match rt.tree(id).nodes[0].kind.clone() {
        NodeKind::Image { mask_uv, .. } => mask_uv,
        _ => panic!("not an image"),
    };
    assert_eq!(mask_uv(&rt), [0.0, 0.0, 1.0, 1.0]);
    rt.set_mask_uv(piece, [-0.75, -0.25, 0.25, 0.75]);
    assert_eq!(mask_uv(&rt), [-0.75, -0.25, 0.25, 0.75]);
}
