//! Real pole contacts must release moving art, while authored static pieces stay put.

use super::*;
use crate::app::test_backend::NullBackend;
use crate::scenes::world::{
    drive::{STEP, walls::Impact},
    object_geometry::{BodyGeometryFrame, object_shapes},
    resident::{Placed, SectionResources, place},
};
use blackbox_collision::bounds_flags;
use blackbox_gfx::{Capabilities, GraphicsApi};
use blackbox_vehicle::{FlatGround, Vehicle};
use game_install::GameDir;
use nfsmw_data::world::{DEFAULT_TRACK, PropCatalog, SectionData, WorldIndex, parse_section};

fn section(dir: &GameDir, index: &WorldIndex, tile: usize) -> SectionData {
    let entry = &index.sections[tile];
    let bytes = dir.read_range(&index.stream_file, u64::from(entry.file_offset), entry.size as usize).unwrap();
    parse_section(tile, &bytes).unwrap()
}

fn matrix_difference(a: Mat4, b: Mat4) -> f32 {
    a.to_cols_array().into_iter().zip(b.to_cols_array()).map(|(x, y)| (x - y).abs()).fold(0.0, f32::max)
}

#[derive(Clone, Copy)]
struct Strike {
    position: Vec3,
    orientation: glam::Quat,
    direction: Vec3,
}

fn strike(world: &mut PropWorld, id: u32, data: &PhysicsData, strike: Strike, speed: f32) -> bool {
    let physics = data.car("BMWM3GTR").unwrap();
    let geometry = &physics.geometry.as_ref().unwrap().body;
    let body = world.colliders[&id].body.as_ref().unwrap();
    let velocity = strike.direction * speed;
    let mut car = Vehicle::new(physics.spec);
    car.body_mut().place(strike.position, strike.orientation);
    car.body_mut().linear_velocity = velocity;
    let frame = BodyGeometryFrame { geometry, previous: (strike.position - velocity / 120.0, strike.orientation) };
    let car_shapes = object_shapes(car.body(), Some(frame));
    let pole_shapes = object_shapes(&body.rigid, Some(body.geometry_frame()));
    let approaching = car_shapes
        .iter()
        .flat_map(|a| pole_shapes.iter().filter_map(move |b| a.contact(b)))
        .filter(|contact| -velocity.dot(contact.normal) > 0.1)
        .count();
    if approaching == 0 {
        return false;
    }
    let mut impact = Impact::default();
    world.collide_vehicle(&mut car, Some(frame), &mut impact);
    !impact.contacts.is_empty()
}

fn fixture(data: &PhysicsData, placed: &Placed) -> Strike {
    let mut source = PropWorld::default();
    let mut p = placed.clone();
    source.add_tile(0, std::slice::from_mut(&mut p));
    let id = p.prop_id;
    let body = source.colliders[&id].body.as_ref().unwrap();
    let base_height = placed.spawner.unwrap().position[1];
    let bottom = |b: &blackbox_vehicle::rigid_body::Obb| {
        blackbox_vehicle::rigid_body::BOX_CORNERS
            .into_iter()
            .map(|corner| (b.centre + b.axes * (b.half * corner)).y)
            .fold(f32::INFINITY, f32::min)
    };
    let mast = body
        .geometry
        .primitives
        .iter()
        .filter(|p| p.collides(bounds_flags::PRIM_VS_OBJECTS))
        .map(|p| body.obb(p))
        .filter(|b| bottom(b) <= base_height + 1.0 && b.centre.y > base_height + 1.0)
        .min_by(|a, b| (bottom(a) - base_height).abs().total_cmp(&(bottom(b) - base_height).abs()))
        .expect("fixture needs an object-colliding mast reaching the pole's base");
    let mast_bottom = bottom(&mast);
    let physics = data.car("BMWM3GTR").unwrap();
    let primitives = &physics.geometry.as_ref().unwrap().body.primitives;
    let base = glam::Quat::from_mat3(&mast.axes);
    let mut attempted = 0;
    // Derive contact poses from actual car primitives and mast faces. Small yaw,
    // height and overlap choices avoid relying on the assembly's broad root box.
    for yaw in [0.0, std::f32::consts::FRAC_PI_2, std::f32::consts::PI, -std::f32::consts::FRAC_PI_2] {
        let orientation = base * glam::Quat::from_rotation_y(yaw);
        for primitive in primitives.iter().filter(|p| p.collides(bounds_flags::PRIM_VS_OBJECTS)) {
            let axes = glam::Mat3::from_quat(orientation * primitive.rotation);
            let support = |direction: Vec3| {
                primitive.half.x * direction.dot(axes.x_axis).abs()
                    + primitive.half.y * direction.dot(axes.y_axis).abs()
                    + primitive.half.z * direction.dot(axes.z_axis).abs()
            };
            for (outward, radius) in [
                (mast.axes.x_axis, mast.half.x),
                (-mast.axes.x_axis, mast.half.x),
                (mast.axes.z_axis, mast.half.z),
                (-mast.axes.z_axis, mast.half.z),
            ] {
                for height in [0.0, 0.15, -0.15] {
                    for overlap in [0.08, 0.2, 0.4] {
                        attempted += 1;
                        let mut centre = mast.centre + outward * (radius + support(outward) - overlap);
                        centre.y = mast_bottom + support(Vec3::Y) + height;
                        let candidate = Strike {
                            position: centre - orientation * primitive.centre,
                            orientation,
                            direction: -outward,
                        };
                        let mut low = PropWorld::default();
                        let mut p = placed.clone();
                        low.add_tile(0, std::slice::from_mut(&mut p));
                        if !strike(&mut low, id, data, candidate, 1.0)
                            || !low.colliders[&id].body.as_ref().unwrap().attached
                        {
                            continue;
                        }
                        let mut high = PropWorld::default();
                        let mut p = placed.clone();
                        high.add_tile(0, std::slice::from_mut(&mut p));
                        if !strike(&mut high, id, data, candidate, 40.0)
                            || high.colliders[&id].body.as_ref().unwrap().attached
                        {
                            continue;
                        }
                        eprintln!(
                            "pole {id} contact fixture after {attempted} poses: position={:?}, orientation={:?}, direction={:?}",
                            candidate.position, candidate.orientation, candidate.direction
                        );
                        return candidate;
                    }
                }
            }
        }
    }
    panic!(
        "no low-contact/high-breakaway pose among {attempted} candidates for pole {id}: mast={mast:?}, car primitives={primitives:?}"
    );
}

struct Motion {
    pole: u32,
    child: u32,
    attached: bool,
    frozen: bool,
    travel: f32,
    rotation: f32,
    render_change: f32,
}

fn run(data: &PhysicsData, mut placed: Placed) -> Vec<Motion> {
    let mut world = PropWorld::default();
    let id = placed.spawner.unwrap().unique_id + 1;
    let base_height = placed.spawner.unwrap().position[1];
    assert!(placed.parts.as_ref().is_some_and(|p| !p.meshes.is_empty()), "fixture needs uploaded hierarchy art");
    let strike_pose = fixture(data, &placed);
    if id == 6638 {
        let mut low = PropWorld::default();
        let mut control = placed.clone();
        low.add_tile(0, std::slice::from_mut(&mut control));
        assert!(strike(&mut low, id, data, strike_pose, 1.0), "low-speed car must physically contact pole {id}");
        low.update_scenery(data);
        assert!(low.colliders[&id].body.as_ref().unwrap().attached, "low impact must not release the pole");
        assert!(low.fragments.is_empty(), "this installed traffic pole responds to detachment, not mere impact");
    }
    world.add_tile(0, std::slice::from_mut(&mut placed));
    let original = world.colliders[&id].body.as_ref().unwrap();
    assert!(original.attached && original.rigid.state == blackbox_vehicle::rigid_body::BodyState::Asleep);
    assert!(strike(&mut world, id, data, strike_pose, 40.0), "high-speed car must physically contact pole {id}");
    let root = world.colliders[&id].body.as_ref().unwrap();
    assert!(!root.attached, "strike must reach the real breakaway threshold");
    let root_position = root.rigid.position;
    let root_visual = root.transform(1.0);
    // The real shared tick dispatches collision stimuli after the vehicle contacts.
    world.update_scenery(data);
    assert!(!world.scenery.models[&id].failed, "installed pole sequencer must initialize");
    assert!(!world.fragments.is_empty(), "physical collision must emit actual installed fragments for pole {id}");
    assert!(
        world.scenery.models[&id].hidden || !world.scenery.models[&id].hidden_parts.is_empty(),
        "the damaged art must replace the parent art"
    );
    world.set_alpha(1.0);
    let initial: Vec<_> = world
        .fragments
        .keys()
        .map(|&child| {
            let body = world.colliders[&child].body.as_ref().unwrap();
            let frozen = body.rigid.state == blackbox_vehicle::rigid_body::BodyState::Frozen;
            let mut drawn = Vec::new();
            assert!(world.model_instances(child, world.pose(child).unwrap().0, &mut drawn));
            assert!(!drawn.is_empty(), "child {child} needs actual uploaded replacement meshes");
            for instance in &drawn {
                assert!(matrix_difference(instance.transform, body.transform(1.0)) < 0.002);
            }
            if frozen {
                assert!(!world.colliders[&child].collidable, "authored no-physics pieces stay out of contacts");
            }
            (child, body.attached, frozen, body.rigid.position, body.rigid.orientation, body.transform(1.0))
        })
        .collect();
    // A lower floor isolates release/integration/rendering from the map's separate ground support.
    let ground = FlatGround::new(base_height - 30.0);
    for _ in 0..12 {
        world.step(&ground, STEP);
        world.update_scenery(data);
    }
    let root = world.colliders[&id].body.as_ref().unwrap();
    eprintln!(
        "pole {id} root: attached={}, state={:?}, travel={} m, render change={}, hidden={}",
        root.attached,
        root.rigid.state,
        root.rigid.position.distance(root_position),
        matrix_difference(root.transform(1.0), root_visual),
        world.scenery.models[&id].hidden
    );
    assert!(root.rigid.position.distance(root_position) > 0.02, "released root body must integrate");
    let observations = initial
        .into_iter()
        .map(|(child, attached, frozen, position, orientation, visual)| {
            let body = world.colliders[&child].body.as_ref().unwrap();
            let transform = world.pose(child).unwrap().0;
            let mut drawn = Vec::new();
            assert!(world.model_instances(child, transform, &mut drawn));
            assert!(!drawn.is_empty(), "moving child must remain rendered");
            assert!(drawn.iter().all(|i| matrix_difference(i.transform, transform) < 0.002));
            let motion = Motion {
                pole: id,
                child,
                attached,
                frozen,
                travel: body.rigid.position.distance(position),
                rotation: body.rigid.orientation.angle_between(orientation),
                render_change: matrix_difference(transform, visual),
            };
            eprintln!(
                "pole {id}, child {child}: attached={attached}, frozen={frozen}, travel={} m, rotation={} rad, render change={}",
                motion.travel, motion.rotation, motion.render_change
            );
            motion
        })
        .collect();
    // Restoring the streamed placement also invalidates its replacement meshes/bodies.
    world.update_lifetimes(&ground, Mat4::from_translation(Vec3::splat(100_000.0)), 9.0);
    assert!(world.fragments.is_empty(), "offscreen restoration must remove detached children");
    assert!(!world.scenery.models[&id].hidden, "restoration must reveal the intact pole again");
    assert!(matrix_difference(world.pose(id).unwrap().0, placed.transform) < 0.002);
    assert!(world.colliders[&id].body.as_ref().unwrap().attached, "restored placement regains its original attachment");
    world.remove_tile(0);
    assert!(world.colliders.is_empty() && world.scenery.models.is_empty() && world.grid.is_empty());
    observations
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_struck_traffic_and_street_poles_release_moving_meshes() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences.clone());
    let catalog =
        PropCatalog::new(index.prop_bounds.clone(), data.database()).with_hierarchies(index.model_hierarchies.clone());
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let empty = SectionResources::default();
    let mut shared = SectionResources::default();
    for tile in index.shared() {
        let section = section(&dir, &index, tile);
        shared.upload_textures(&mut renderer, &section);
        shared.upload_meshes(&mut renderer, &section, &empty);
    }
    let mut observations = Vec::new();
    for (tile, unique_id) in [("I65", 6637), ("P46", 8482)] {
        let tile = index.by_name(tile).expect("fixture needs the authored pole tile");
        let section = section(&dir, &index, tile);
        let mut own = SectionResources::default();
        own.upload_textures(&mut renderer, &section);
        own.upload_meshes(&mut renderer, &section, &shared);
        let (placed, _) = place(&section, &own, &shared, &catalog);
        let target = placed
            .into_iter()
            .find(|p| p.spawner.is_some_and(|s| s.unique_id == unique_id))
            .expect("fixture needs the uploaded spawner-backed pole");
        observations.extend(run(&data, target));
        own.release(&mut renderer);
    }
    shared.release(&mut renderer);
    for pole in [6638, 8483] {
        assert!(observations.iter().any(|m| m.pole == pole && !m.frozen), "pole {pole} must emit a physical child");
    }
    for m in observations {
        if m.frozen {
            assert!(m.travel < 0.001 && m.render_change < 0.002, "authored static stump {} must stay still", m.child);
            continue;
        }
        assert!(!m.attached, "released pole {} child {} must not remain attached to the world", m.pole, m.child);
        assert!(m.travel > 0.02, "released pole {} child {} must integrate its motion", m.pole, m.child);
        assert!(m.rotation > 0.01, "released pole {} child {} must rotate after the base strike", m.pole, m.child);
        assert!(m.render_change > 0.02, "released child {} meshes must follow the simulated body", m.child);
    }
}
