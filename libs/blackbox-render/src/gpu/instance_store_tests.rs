// The dirty ranges are compared as lists; a list holding one range is what is meant.
#![allow(clippy::single_range_in_vec_init)]
use super::*;
use crate::gpu::draw_list::instance_view::instance_source;

fn matrix(value: f32) -> [u32; 20] {
    bytemuck::cast([value; 20])
}

#[test]
fn receiver_metadata_preserves_every_transform_component_and_updates_keyed_slots() {
    use blackbox_gfx::{Instance, MeshHandle, ShadowFlags};
    let transform = glam::Mat4::from_cols_array(&std::array::from_fn(|i| i as f32 * 0.25));
    let instance = Instance::keyed(MeshHandle::from_raw(0), transform, InstanceKey::new(1, 0));
    let enabled = crate::gpu::instances::instance_words(&instance);
    assert_eq!(&enabled[..16], &transform.to_cols_array());
    assert_eq!(enabled[16..], [1.0, 0.0, 0.0, 0.0]);
    let disabled =
        crate::gpu::instances::instance_words(&instance.with_shadow(ShadowFlags { cast: true, receive: false }));
    assert_eq!(&disabled[..16], &enabled[..16]);
    assert_eq!(disabled[16..], [0.0; 4]);
    let mut table = SlotTable::default();
    table.begin_epoch();
    let slot = table.assign(instance.key, bytemuck::cast(enabled));
    dirty(&mut table);
    table.begin_epoch();
    assert_eq!(table.assign(instance.key, bytemuck::cast(disabled)), slot);
    assert_eq!(dirty(&mut table), [slot..slot + 1], "receiver changes upload even when the matrix stays still");
}

fn dirty(table: &mut SlotTable) -> Vec<Range<u32>> {
    let mut out = Vec::new();
    table.take_dirty(&mut out);
    out
}

#[test]
fn a_keyed_instance_keeps_its_slot_and_is_written_only_when_it_moves() {
    let mut table = SlotTable::default();
    let key = InstanceKey::new(1, 2);
    table.begin_epoch();
    let slot = table.assign(key, matrix(1.0));
    assert_eq!(dirty(&mut table), [slot..slot + 1]);
    for _ in 0..3 {
        table.begin_epoch();
        assert_eq!(table.assign(key, matrix(1.0)), slot);
        assert!(dirty(&mut table).is_empty(), "an unchanged matrix is not written again");
    }
    table.begin_epoch();
    assert_eq!(table.assign(key, matrix(2.0)), slot);
    assert_eq!(dirty(&mut table), [slot..slot + 1]);
    assert_eq!(table.data()[slot as usize], matrix(2.0));
}

#[test]
fn a_negative_zero_is_a_change() {
    let mut table = SlotTable::default();
    let key = InstanceKey::new(0, 0);
    table.begin_epoch();
    let slot = table.assign(key, matrix(0.0));
    let _ = dirty(&mut table);
    table.begin_epoch();
    table.assign(key, matrix(-0.0));
    assert_eq!(dirty(&mut table), [slot..slot + 1]);
}

#[test]
fn transient_instances_take_last_epochs_slots_in_order() {
    let mut table = SlotTable::default();
    table.begin_epoch();
    let first: Vec<_> = (0..3).map(|i| table.assign(InstanceKey::TRANSIENT, matrix(i as f32))).collect();
    let _ = dirty(&mut table);
    assert_eq!(first.len(), 3);
    assert!(first[0] != first[1] && first[1] != first[2] && first[0] != first[2], "one slot each in an epoch");
    table.begin_epoch();
    let again: Vec<_> = (0..3).map(|i| table.assign(InstanceKey::TRANSIENT, matrix(i as f32))).collect();
    assert_eq!(again, first);
    assert!(dirty(&mut table).is_empty(), "the same stream rewrites nothing");
    // Fewer transients: the unused slot is free for a new key next epoch, with no growth.
    table.begin_epoch();
    table.assign(InstanceKey::TRANSIENT, matrix(0.0));
    table.begin_epoch();
    let size = table.len();
    table.assign(InstanceKey::new(9, 9), matrix(5.0));
    table.assign(InstanceKey::new(9, 10), matrix(5.0));
    assert_eq!(table.len(), size, "freed transient slots are reused");
}

#[test]
fn a_key_drawn_twice_in_one_epoch_with_another_matrix_gets_a_second_slot() {
    let mut table = SlotTable::default();
    let key = InstanceKey::new(3, 4);
    table.begin_epoch();
    let slot = table.assign(key, matrix(1.0));
    assert_eq!(table.assign(key, matrix(1.0)), slot, "the same matrix shares the slot");
    let other = table.assign(key, matrix(2.0));
    assert_ne!(other, slot, "the first view keeps the matrix it was drawn with");
    assert_eq!(table.data()[slot as usize], matrix(1.0));
    assert_eq!(table.data()[other as usize], matrix(2.0));
    // Next epoch the key moves for real and keeps its own slot.
    table.begin_epoch();
    assert_eq!(table.assign(key, matrix(2.0)), slot);
}

#[test]
fn a_key_not_drawn_for_long_gives_its_slot_back() {
    let mut table = SlotTable::default();
    let old = InstanceKey::new(1, 1);
    table.begin_epoch();
    let slot = table.assign(old, matrix(1.0));
    for _ in 0..RETAIN_EPOCHS {
        table.begin_epoch();
    }
    let kept = InstanceKey::new(2, 2);
    let fresh = table.assign(kept, matrix(1.0));
    assert_ne!(fresh, slot, "still retained after RETAIN_EPOCHS epochs");
    table.begin_epoch();
    let reused = table.assign(InstanceKey::new(5, 5), matrix(7.0));
    assert_eq!(reused, slot, "expired slots are reused");
    assert_eq!(table.data()[slot as usize], matrix(7.0));
}

#[test]
fn dirty_slots_coalesce_into_sorted_ranges() {
    let mut out = Vec::new();
    let mut dirty = vec![5, 1, 2, 3, 9, 20, 21, 2];
    coalesce(&mut dirty, 0, &mut out);
    assert_eq!(out, [1..4, 5..6, 9..10, 20..22]);
    assert!(dirty.is_empty());
    let mut dirty = vec![5, 1, 2, 3, 9, 20, 21];
    coalesce(&mut dirty, 1, &mut out);
    assert_eq!(out, [1..6, 9..10, 20..22]);
    let mut dirty = vec![5, 1, 2, 3, 9, 20, 21];
    coalesce(&mut dirty, 4, &mut out);
    assert_eq!(out, [1..10, 20..22]);
    let mut dirty = vec![];
    coalesce(&mut dirty, 4, &mut out);
    assert!(out.is_empty());
    let mut dirty = vec![u32::MAX - 1];
    coalesce(&mut dirty, u32::MAX, &mut out);
    assert_eq!(out, [u32::MAX - 1..u32::MAX]);
}

/// The scene shaders with the store's instance input, as the pipelines build them, parse and validate.
#[test]
fn store_shader_variants_validate() {
    let receiver = include_str!("../shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "2");
    let sources = [
        (
            "scene",
            format!(
                "{}\n{}\n{}",
                instance_source(include_str!("../shaders/scene.wgsl"), Some(3)),
                include_str!("../shaders/world_surface.wgsl"),
                receiver
            ),
        ),
        ("normal", format!("{}\n{}", instance_source(include_str!("../shaders/normal.wgsl"), Some(3)), receiver)),
        ("window", format!("{}\n{}", instance_source(include_str!("../shaders/window.wgsl"), Some(3)), receiver)),
        ("sky", instance_source(include_str!("../shaders/sky.wgsl"), Some(2))),
        ("shadow depth", instance_source(include_str!("../shaders/shadow_depth.wgsl"), Some(2))),
        ("contact prepass", instance_source(include_str!("../shaders/contact_prepass.wgsl"), Some(2))),
        ("motion", instance_source(include_str!("../shaders/motion.wgsl"), Some(2))),
    ];
    for (name, source) in sources {
        assert!(source.contains("instance_models"), "{name} reads the store");
        let module =
            naga::front::wgsl::parse_str(&source).unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(&source)));
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::default())
            .validate(&module)
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    }
}

#[test]
fn without_the_store_the_shader_is_unchanged() {
    let source = include_str!("../shaders/scene.wgsl");
    assert_eq!(instance_source(source, None), source);
}
