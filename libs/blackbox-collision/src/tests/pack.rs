use super::synth::*;
use crate::carp::{Carp, tag4};
use crate::{CollisionPack, Error, STRIP_UP_FACING, read_collision_packs};

const ASPHALT: u32 = 0x19DB_2F1E;
const GRASS: u32 = 0x772F_B736;

fn strip_article() -> Vec<u8> {
    article(
        &[StripSpec {
            center: [1.0, 2.0, 3.0],
            radius: 40,
            flags: STRIP_UP_FACING,
            verts: vec![
                vert(0, 0, 0, 0, 0),
                vert(128, 0, 0, 0, 0),
                vert(0, 64, 128, 1, 0x04),
                vert(128, 64, 128, 0, 0x08),
            ],
        }],
        &[BarrierSpec { p0: [0.0, 0.0, 0.0], p1: [3.0, 2.0, 4.0], surface: 1, flags: 0x12 }],
        &[ASPHALT, GRASS],
    )
}

#[test]
fn parses_articles_and_instances() {
    let payload = pack(101, &[strip_article()], &[InstanceSpec::upright([10.0, 5.0, -20.0], [4.0, 2.0, 4.0], 0)]);
    let p = CollisionPack::parse(&payload).unwrap();
    assert_eq!(p.section, 101);
    assert_eq!(p.instances.len(), 1);
    assert!(p.objects.is_empty());

    let inst = &p.instances[0];
    assert_eq!((inst.half_width, inst.half_length, inst.half_height), (4.0, 4.0, 2.0));
    assert_eq!((inst.group, inst.article, inst.flags, inst.iter_stamp), (0, 0, 0, 1234));
    assert_eq!(inst.position(), [10.0, 5.0, -20.0]);
    assert_eq!(inst.translation, [-10.0, -5.0, 20.0]);

    let a = p.article_of(0).unwrap();
    assert_eq!(a.surfaces, [ASPHALT, GRASS]);
    assert_eq!((a.intermediate_object, a.flags), (7, 0));
    let s = &a.strips[0];
    assert_eq!((s.center, s.radius, s.flags), ([1.0, 2.0, 3.0], 2.5, STRIP_UP_FACING));
    assert_eq!(s.verts.len(), 4);
    assert_eq!(s.triangle_count(), 2);
    assert_eq!(s.point(2), [1.0, 2.5, 4.0]);
    let tris: Vec<_> = s.triangles().collect();
    assert_eq!(tris[0].pts, [[1.0, 2.0, 3.0], [2.0, 2.0, 3.0], [1.0, 2.5, 4.0]]);
    // A triangle takes its surface from its third vertex.
    assert_eq!((tris[0].surface, tris[0].flags), (1, 0x04));
    assert_eq!((tris[1].surface, tris[1].flags), (0, 0x08));

    let b = a.barriers[0];
    assert_eq!((b.p1, b.surface, b.flags), ([3.0, 2.0, 4.0], 1, 0x12));
    assert!((b.inv_xz_len - 0.2).abs() < 1e-6);
    assert_eq!((b.y_bottom(), b.y_top()), (0.0, 2.0));
    assert_eq!(b.normal(), [0.8, 0.0, -0.6]);
    assert_eq!(a.surface_hash(1), Some(GRASS));
    assert_eq!(a.surface_hash(2), None);
}

#[test]
fn reads_packs_from_a_stream_with_alignment_padding() {
    let blob = |n: u32| {
        let payload = pack(n, &[strip_article()], &[InstanceSpec::upright([0.0; 3], [4.0; 3], 0)]);
        // `pack_chunk` wants the blob; strip the 16-byte header the helper put in front.
        pack_chunk(n, &payload[16..])
    };
    let mut file = blob(101);
    file.extend(blob(102));
    let packs = read_collision_packs(&file).unwrap();
    assert_eq!(packs.iter().map(|p| p.section).collect::<Vec<_>>(), [101, 102]);
}

#[test]
fn carp_tree_navigation() {
    let blob = carp(tag4(b"Arti"), &[(tag4(b"Name"), 0, vec![1, 2, 3, 4]), (7, 2, vec![9; 8])]);
    let root = Carp::new(&blob).root().unwrap();
    assert_eq!((root.tag, root.group_count(), root.record_count()), (tag4(b"CARP"), 1, 0));
    let arti = root.group(tag4(b"Arti")).unwrap().unwrap();
    assert_eq!(arti.record_count(), 2);
    assert_eq!(arti.record(7).unwrap().unwrap().data, [9; 8]);
    assert!(arti.record(8).unwrap().is_none());
    assert!(root.group(tag4(b"Nope")).unwrap().is_none());
    assert!(matches!(arti.require(8, "thing"), Err(Error::Missing("thing"))));
}

#[test]
fn group_exclusion_marks_grouped_instances() {
    let wall = article(&[], &[BarrierSpec { p0: [0.0; 3], p1: [1.0, 1.0, 0.0], surface: 0, flags: 0x02 }], &[ASPHALT]);
    let mut grouped = InstanceSpec::upright([0.0; 3], [4.0; 3], 0);
    grouped.group = 3;
    let plain = InstanceSpec::upright([0.0; 3], [4.0; 3], 1);
    let mut p = CollisionPack::parse(&pack(5, &[wall.clone(), wall], &[grouped, plain])).unwrap();
    p.apply_group_exclusion();
    assert_eq!(p.instances[0].flags, 0xC0);
    assert_eq!(p.articles[0].barriers[0].flags, 0xC2);
    assert_eq!(p.instances[1].flags, 0);
    assert_eq!(p.articles[1].barriers[0].flags, 0x02);
}

#[test]
fn rejects_bad_data() {
    let good = pack(1, &[strip_article()], &[InstanceSpec::upright([0.0; 3], [4.0; 3], 0)]);
    assert!(matches!(CollisionPack::parse(&good[..good.len() - 100]), Err(Error::Truncated { .. })));
    // An instance pointing past the article list.
    let bad = pack(1, &[strip_article()], &[InstanceSpec::upright([0.0; 3], [4.0; 3], 3)]);
    assert!(matches!(CollisionPack::parse(&bad), Err(Error::Malformed { .. })));
    // No Arti group at all.
    let blob = carp(tag4(b"Zzzz"), &[]);
    assert!(matches!(CollisionPack::parse(&pack_payload(1, &blob)), Err(Error::Missing(_))));
}
