use super::synth::*;
use crate::{CollisionPack, CollisionWorld, RayOptions};

/// A barrier across x = `x`, in an upright instance at the origin (the shape the filtered tests use).
fn barrier_pack(section: u32, x: f32) -> CollisionPack {
    let barrier = BarrierSpec { p0: [x, -1.0, -4.0], p1: [x, 3.0, 4.0], surface: 0, flags: 0 };
    let articles = [article(&[], &[barrier], &[7])];
    CollisionPack::parse(&pack(section, &articles, &[InstanceSpec::upright([0.0; 3], [4.0; 3], 0)])).unwrap()
}

#[test]
fn an_ungridded_pack_is_hit_by_ray_casts_and_removed_on_request() {
    let mut world = CollisionWorld::new(None);
    world.insert_ungridded(barrier_pack(3000, 1.0));
    let (from, to) = ([0.0, 1.0, 0.0], [3.0, 1.0, 0.0]);
    let hit = world.ray_cast(from, to, &RayOptions::default()).expect("the ungridded pack is tested");
    assert_eq!(hit.section, 3000);
    assert_eq!(hit.point[0], 1.0);

    let revision = world.revision();
    assert!(world.remove_ungridded(3000).is_some());
    assert_ne!(world.revision(), revision, "the removal changes the revision");
    assert!(world.ray_cast(from, to, &RayOptions::default()).is_none());
    assert!(world.remove_ungridded(3000).is_none(), "a second removal finds nothing");
}

#[test]
fn ungridded_packs_do_not_replace_gridded_ones_of_the_same_section() {
    let mut world = CollisionWorld::new(None);
    world.insert(barrier_pack(3000, 1.0));
    world.insert_ungridded(barrier_pack(3000, 2.0));
    assert!(world.pack(3000).is_some(), "the gridded pack stays");
    let hit = world.ray_cast([0.0, 1.0, 0.0], [3.0, 1.0, 0.0], &RayOptions::default()).unwrap();
    assert_eq!(hit.point[0], 1.0, "the nearer wall is the gridded one");
    assert!(world.remove_ungridded(3000).is_some());
    assert!(world.pack(3000).is_some(), "removing the ungridded pack leaves the gridded one");
}
