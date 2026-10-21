//! Full traffic/prop phase ordering after the authored delayed panel release.
use super::*;
use crate::scenes::world::ai::{Focus, TrafficWorld};
use crate::viewer::camera::Lens;
use blackbox_roads::SegmentIndex;

#[test]
#[ignore = "needs installed police geometry, sequences, fragment physics and streamed highway collision"]
fn installed_hood_dynamics_follow_live_parent_and_native_prop_contact_phases() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences.clone());
    let net = index.road_network.as_ref().unwrap();
    let (mut spawn, _, _) = highway(net);
    let collision = collision(&dir, &index, spawn);
    spawn.position.z =
        road::probe(&collision, spawn.position.x, spawn.position.y, spawn.position.z + 10.0).unwrap().height;
    let ground = WorldGround { collision: &collision, surfaces: &data.surfaces };
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan)).with_mesh_tracking();
    let installed =
        nfsmw_data::car::load_driving(&dir, "COPMIDSIZE", &LoadOptions { lod: 'A', all_parts: false, preset: None })
            .unwrap();
    let slot = nfsmw_data::car::LAYOUT.slots.iter().position(|s| s.name == "DAMAGE_HOOD").unwrap();
    let hood = installed
        .driving_lods
        .damage
        .levels
        .iter()
        .find(|l| l.level == 0)
        .unwrap()
        .placements
        .iter()
        .find(|p| p.slot == slot)
        .unwrap();
    let hood_name = installed.solids[&hood.solid].name.clone();
    let mut template = model(&dir, &data, "copmidsize", &mut renderer);
    template.rig = Rc::new(CarRig::upload(&mut renderer, 9878, installed, CarShading::default(), sun::to_sun()));
    let spec = template.physics.events.as_ref().unwrap();
    let geometry = template.physics.geometry.as_ref().unwrap();
    eprintln!("hood parent primitives={:?}", geometry.body.primitives);
    for node in &geometry.bounds.nodes {
        eprintln!("hood source bounds={node:?}");
    }
    let segments = SegmentIndex::build(net);
    for (moving, rotation) in [(0.0, 0.0), (10.0, 0.3)] {
        let start = Spawn { heading: spawn.heading + rotation, ..spawn };
        let mut nav = RoadNav::traffic();
        let forward = Vec3::new(start.heading.cos(), start.heading.sin(), 0.0);
        assert!(nav.init_at_point(
            net,
            &segments,
            Vec3::from(space::to_physics(start.position)),
            Vec3::from(space::to_physics(forward)),
            false
        ));
        let mut car = AiCar::place(
            &template,
            Role::Traffic,
            nav,
            start,
            Start { speed: moving, stagger: 0, cruise: Cruise::default(), patrol: true },
            &ground,
        )
        .unwrap();
        car.set_roadblock();
        let actor = car.key();
        let mut props = PropWorld::default();
        car.sims_mut().0.events.as_mut().unwrap().hit(DamageHit {
            arm: Vec3::Z * 3.0,
            normal: Vec3::NEG_Z,
            half: Vec3::new(1.0, 1.0, 4.0),
            closing_speed: 10.0,
            force: 1.1 * spec.strength / spec.visual_scale[0],
            shock_force: 0.0,
            before_velocity: Vec3::ZERO,
            other: OtherBody::default(),
        });
        car.update_events(&mut props, &data, start.position);
        assert!(car.destroy_by_breaker());
        let mut world = TrafficWorld::new(net.clone());
        world.cars.push(car);
        let mut born = None;
        let mut max_speed = 0.0_f32;
        let mut max_relative_height = f32::NEG_INFINITY;
        let mut max_parent_speed = 0.0_f32;
        for tick in 0..1500 {
            assert_eq!(world.cars.len(), 1, "fixture actor must stay in the actual world lifecycle");
            let at = world.cars[0].position();
            let lens = Lens::look_at(at - Vec3::Y * 12.0 + Vec3::Z * 4.0, at, 70.0, 0.1, 16.0 / 9.0);
            let focus = Focus { position: [at.x, at.y], heading: start.heading, speed: 0.0, body: None };
            props.begin_native(STEP);
            let before_car = props.fragment_bodies().first().map(|(_, body, _)| body.position);
            world.set_damage_view(Vec3::from(space::to_physics(lens.position)));
            world.step(focus, &collision, &mut props, &data.surfaces);
            let after_car = props.fragment_bodies().first().map(|(_, body, _)| body.position);
            world.collide(None);
            world.update_events(&mut props, &data, lens.position);
            world.finish_step(focus, &lens, &lens, &props, &collision, &data.surfaces);
            props.finish_native_world(&ground, STEP);
            assert_eq!(world.cars.len(), 1, "a visible wreck must not be silently replaced");
            assert_eq!(world.cars[0].key(), actor);
            assert!(world.cars[0].is_wrecked());
            let parent = world.cars[0].sims_mut().0.body();
            let parent_position = parent.position;
            let parent_velocity = parent.linear_velocity;
            max_parent_speed = max_parent_speed.max(parent_velocity.length());
            let hidden = world.cars[0].sims_mut().0.events.as_ref().unwrap().hidden_slots.contains(&slot);
            let mut meshes = Vec::new();
            props.fragment_instances(&mut meshes);
            let hood_mesh = meshes.iter().find(|p| renderer.mesh_labels.get(&p.mesh) == Some(&hood_name));
            if !hidden {
                assert!(hood_mesh.is_none());
                continue;
            }
            let birth = *born.get_or_insert(tick);
            assert!(hood_mesh.is_some(), "the hidden parent hood must have a live detached model");
            let fragments = props.fragment_bodies();
            assert_eq!(fragments.len(), 1, "this controlled front-zone action releases one hood");
            let (id, body, _) = fragments[0];
            assert!(body.position.is_finite() && body.linear_velocity.is_finite() && body.angular_velocity.is_finite());
            assert!(hood_mesh.unwrap().transform.is_finite());
            max_speed = max_speed.max(body.linear_velocity.length());
            let height = body.position.y - parent_position.y;
            max_relative_height = max_relative_height.max(height);
            if [0, 1, 30, 60, 120, 299].contains(&(tick - birth)) {
                eprintln!(
                    "hood phases before_car={before_car:?} after_car={after_car:?} after_world={:?}",
                    body.position
                );
                eprintln!(
                    "hood dynamics moving={moving} yaw={rotation} tick={} actor={actor} fragment={id} parent={parent_position:?} parent_v={parent_velocity:?} fragment_p={:?} fragment_v={:?} omega={:?} state={:?} half={:?} mass={} mesh_at={:?}",
                    tick - birth,
                    body.position,
                    body.linear_velocity,
                    body.angular_velocity,
                    body.state,
                    body.dimension(),
                    body.mass(),
                    hood_mesh.unwrap().transform.w_axis
                );
            }
        }
        assert!(born.is_some(), "actual traffic/event clock must release the authored hood");
        eprintln!(
            "hood dynamic bounds moving={moving} yaw={rotation} birth={born:?} max_fragment_speed={max_speed} max_relative_height={max_relative_height} max_parent_speed={max_parent_speed}"
        );
    }
}
