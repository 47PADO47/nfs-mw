//! Oblique barrier materials, independent of tire/ground friction.
use super::*;
use blackbox_collision::{Article, Barrier, CollisionPack, Instance};
use blackbox_vehicle::VehicleSpec;
use blackbox_vehicle::rigid_body::FrictionState;

fn barrier_world(surface: u32) -> CollisionWorld {
    let mut world = CollisionWorld::new(None);
    world.insert(CollisionPack {
        section: 7,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            half_width: 10.0,
            half_height: 4.0,
            half_length: 10.0,
            translation: [0.0; 3],
            radius: 16.0,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        }],
        articles: vec![Article {
            strips: vec![],
            barriers: vec![Barrier {
                p0: [0.8, -3.0, 8.0],
                p1: [0.8, 3.0, -8.0],
                surface: 0,
                flags: BARRIER_TWO_SIDED,
                inv_xz_len: 1.0 / 16.0,
            }],
            surfaces: vec![surface],
            intermediate_object: 0,
            flags: 0,
        }],
        objects: vec![],
    });
    world
}

fn no_props(_: Vec3, _: Mat3, _: Vec3) -> Vec<WallContact> {
    vec![]
}

fn compare(spec: VehicleSpec, walls: WallSpec, surface: u32, scale: f32, tangent: f32, player: bool) -> FrictionState {
    let world = barrier_world(surface);
    let mut actual = Vehicle::new(spec);
    actual.body_mut().linear_velocity = Vec3::new(8.0, 0.0, tangent);
    let mut expected = actual.clone();
    let cast = world_cast_with_mask(&world, 0);
    let half = actual.body().dimension() + actual.body().spec().collision_box_pad;
    let contacts = find(&cast, &no_props, actual.body().position, actual.body().rotation(), half);
    assert!(!contacts.is_empty(), "real barrier query must return contact metadata");
    assert!(contacts.iter().all(|c| c.info.is_some_and(|i| i.barrier && i.surface == surface)));
    let contact = contacts[0];
    let mut state = None;
    for contact in &contacts {
        let local = expected.body().rotation().transpose() * contact.normal;
        let (centre, params) = expected.wall_reaction(
            contact.normal,
            ContactParams { restitution: (local * walls.elasticity).length(), inertia_scale: walls.moment_scale },
            player,
        );
        let plane = PlaneContact {
            point: contact.point,
            normal: contact.normal,
            friction_static: walls.friction[0] * scale,
            friction_kinetic: walls.friction[0].min(walls.friction[1]) * scale,
        };
        if let Some(reaction) = expected.body_mut().react_plane_at(&plane, &params, Some(centre)) {
            state = Some(reaction.friction);
            break;
        }
    }
    expected.body_mut().position += contact.normal * contact.depth;
    let lookups = std::cell::Cell::new(0);
    let previous = actual.body().position;
    let impact = resolve_into(
        &mut actual,
        previous,
        &cast,
        &no_props,
        &walls,
        &|tag| {
            assert_eq!(tag, surface);
            lookups.set(lookups.get() + 1);
            scale
        },
        player,
        &mut Vec::new(),
    );
    assert!(lookups.get() > 0 && impact.impulse > 0.0);
    assert!(actual.body().linear_velocity.abs_diff_eq(expected.body().linear_velocity, 0.00001));
    assert!(actual.body().angular_velocity.abs_diff_eq(expected.body().angular_velocity, 0.00001));
    assert!(actual.body().position.abs_diff_eq(expected.body().position, 0.00001));
    assert!(actual.body().linear_velocity.length_squared() <= Vec3::new(8.0, 0.0, tangent).length_squared() + 0.001);
    state.unwrap()
}

#[test]
fn wall_material_scales_both_friction_modes_and_retains_zero_surface_identity() {
    let walls = WallSpec { friction: [0.5, 0.2], ..Default::default() };
    let mut states = std::collections::BTreeSet::new();
    for surface in [0, 7] {
        for scale in [0.0, 0.25, 2.0] {
            for tangent in [0.05, 30.0] {
                for player in [false, true] {
                    let state = compare(VehicleSpec::example(), walls, surface, scale, tangent, player);
                    states.insert(format!("{state:?}"));
                }
            }
        }
    }
    assert!(states.contains("Static") && states.contains("Dynamic"), "exercise both Coulomb branches: {states:?}");
}

#[test]
#[ignore = "needs installed cop physics, surfaces and track barrier material identities"]
fn installed_cop_wall_contacts_use_each_encountered_barrier_material() {
    use nfsmw_data::car::physics::PhysicsData;
    use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let stream = nfsmw_data::read_unwrapped(&dir, &index.stream_file).unwrap();
    let packs = blackbox_collision::read_collision_packs(&stream).unwrap();
    let mut tags = std::collections::BTreeSet::new();
    for pack in &packs {
        for article in &pack.articles {
            for barrier in &article.barriers {
                tags.insert(article.surfaces[usize::from(barrier.surface)]);
            }
        }
    }
    assert!(!tags.is_empty());
    let data = PhysicsData::load(&dir).unwrap();
    let mut checked = 0;
    let mut nonunit = 0;
    for tag in &tags {
        let multiplier = data.surfaces.world_friction(*tag);
        eprintln!("barrier surface {tag:08X}: WORLD_FRICTION {multiplier}");
        nonunit += usize::from(multiplier != 1.0);
        for name in ["copmidsize", "copgto", "copsport", "copsuv"] {
            let physics = data.car(name).unwrap();
            compare(physics.spec, physics.walls, *tag, multiplier, 30.0, false);
            checked += 1;
        }
    }
    assert_eq!(checked, tags.len() * 4);
    eprintln!("installed cop barrier-material responses: {checked}; material tags {}, nonunit {nonunit}", tags.len());
}
