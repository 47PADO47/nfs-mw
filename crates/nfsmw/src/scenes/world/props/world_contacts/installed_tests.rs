//! Installed geometry integration, distinct from an original trajectory comparison.

use super::*;
use blackbox_attrib::vlt_hash;
use blackbox_collision::read_collision_packs;
use blackbox_scenery::{layout::MOST_WANTED, read_object_spawners};
use blackbox_vehicle::rigid_body::BodyState;
use game_install::GameDir;
use glam::Mat4;
use nfsmw_data::world::{DEFAULT_TRACK, PropCatalog, WorldIndex};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_parked_car_hits_an_authored_barrier_through_the_world_volume_query() {
    let dir = GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let stream = nfsmw_data::read_unwrapped(&dir, &index.stream_file).unwrap();
    let db = nfsmw_data::shared::attributes(&dir).unwrap();
    let catalog = PropCatalog::new(index.prop_bounds, &db);
    let spawn = read_object_spawners(&stream, &MOST_WANTED.spawners)
        .unwrap()
        .into_iter()
        .find(|spawn| spawn.attributes == vlt_hash("parked_car"))
        .expect("installed fixture needs an actual parked-car spawner");
    let shape = catalog.for_spawner("installed parked car", &spawn).expect("parked-car collision bounds");
    let spec = shape.dynamics.as_ref().unwrap();
    assert!(!spec.no_world_collisions, "selected parked-car source must enable world collisions");
    let mut body = PropBody::new(&spawn, shape.bounds.as_ref().unwrap(), spec, Mat4::IDENTITY).unwrap();
    let primitive = body
        .geometry
        .primitives
        .iter()
        .find(|p| p.collides(bounds_flags::PRIM_VS_WORLD) && p.shape == Shape::Box)
        .expect("parked-car fixture needs a source world collision box")
        .clone();
    let original = (body.rigid.mass(), body.rigid.inertia(), body.rigid.cog(), body.rigid.dimension());
    let packs = read_collision_packs(&stream).unwrap();
    assert!(!packs.is_empty(), "installed fixture selected no collision packs");
    let object_count: usize = packs.iter().map(|p| p.objects.len()).sum();
    let mut targets = Vec::new();
    for pack in &packs {
        for (i, instance) in pack.instances.iter().enumerate() {
            if instance.group != 0 || instance.flags != 0 {
                continue;
            }
            let article = pack.article_of(i).unwrap();
            for barrier in &article.barriers {
                let p0 = Vec3::from_array(instance.to_world(barrier.p0));
                let p1 = Vec3::from_array(instance.to_world(barrier.p1));
                let delta = p1 - p0;
                let normal = Vec3::new(delta.z, 0.0, -delta.x) * barrier.inv_xz_len;
                if Vec3::new(delta.x, 0.0, delta.z).length() > 20.0
                    && delta.y.abs() > 6.0
                    && (normal.length_squared() - 1.0).abs() < 0.001
                {
                    targets.push((pack.section, i, p0, p1, normal));
                }
            }
        }
    }
    assert!(!targets.is_empty(), "installed fixture needs an eligible full-height authored barrier");
    let mut collision = CollisionWorld::new(Some(index.collision_grid.expect("installed collision grid")));
    for pack in packs {
        collision.insert(pack);
    }
    let options = RayOptions { exclude: 0, skip_groups: true, ..RayOptions::default() };
    let mut scratch = Vec::new();
    // Select by authored geometry and query membership, never by whether the response passed.
    let target = targets
        .into_iter()
        .find(|(section, instance, p0, p1, _)| {
            collision.barriers_near_into(((*p0 + *p1) * 0.5).to_array(), 3.0, &options, &mut scratch);
            scratch.iter().any(|hit| {
                hit.section == *section
                    && hit.instance == *instance
                    && Vec3::from_array(hit.barrier.p0) == *p0
                    && Vec3::from_array(hit.barrier.p1) == *p1
            })
        })
        .expect("authored barrier must be reachable through the installed world grid");
    let (_, _, p0, p1, normal) = target;
    let rotation = body.rigid.rotation();
    let axes = primitive_at(&primitive, (Vec3::ZERO, body.rigid.orientation)).axes;
    let support = (axes.transpose() * normal).abs().dot(primitive.half);
    let position = (p0 + p1) * 0.5 + normal * (support - 0.2) - rotation * primitive.centre;
    body.rigid.position = position;
    body.rigid.linear_velocity = -normal * 20.0;
    body.rigid.angular_velocity = Vec3::ZERO;
    body.rigid.state = BodyState::Awake;
    body.previous = (position + normal * 0.5, body.rigid.orientation);
    body.attached = false;
    body.moved = true;
    let surfaces = SurfaceTable::from_database(&db);
    step(&mut body, spec, &collision, &surfaces, &mut scratch, &mut vec![], &mut vec![]);
    assert!(!scratch.is_empty(), "response must query actual nearby world volumes");
    assert!((body.rigid.position - position).dot(normal) > 0.05, "authored wall must correct penetration");
    assert!(body.rigid.linear_velocity.dot(normal) > -19.0, "authored wall must reduce incoming parked-car motion");
    assert!(
        body.rigid.position.is_finite()
            && body.rigid.linear_velocity.is_finite()
            && body.rigid.angular_velocity.is_finite()
    );
    assert_eq!((body.rigid.mass(), body.rigid.inertia(), body.rigid.cog(), body.rigid.dimension()), original);
    println!("installed parked-car world contact: 1 response, collision objects={object_count}");
}
