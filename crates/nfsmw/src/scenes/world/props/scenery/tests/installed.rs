//! Installed art is checked on the CPU; GPU handles are unnecessary for solver traces.

use super::super::ram::strike;
use super::*;
use crate::scenes::world::{
    drive::{FixedClock, STEP},
    resident::Placed,
    space,
};
use blackbox_scenery::{layout::MOST_WANTED, read_object_spawners, read_scenery_sections};
use blackbox_vehicle::FlatGround;
use game_install::GameDir;
use nfsmw_data::world::{DEFAULT_TRACK, PropCatalog, WorldIndex};

struct Installed {
    data: PhysicsData,
    placed: Vec<Placed>,
}

fn installed() -> Option<Installed> {
    let _ = env_logger::try_init();
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR")?).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    assert!(index.model_hierarchies.len() > 100);
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences.clone());
    let catalog =
        PropCatalog::new(index.prop_bounds.clone(), data.database()).with_hierarchies(index.model_hierarchies.clone());
    let engines: BTreeSet<_> = [
        "gaspumpexplosion_seq",
        "gasstationsupport_seq",
        "gasstationroof_seq",
        "donutshoproof_seq",
        "donutshoproofsupport_seq",
        "scaffoldsupport_seq",
        "woodenscaffold_seq",
        "cranesupport_seq",
        "watertower_seq",
        "watertowersupport_seq",
    ]
    .into_iter()
    .map(vlt_hash)
    .collect();
    let mut available = BTreeSet::<u32>::new();
    let mut placed = Vec::new();
    for section in &index.sections {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        let local: BTreeSet<_> =
            blackbox_solid::read_solids(&bytes).unwrap().into_iter().map(|s| s.name_hash).collect();
        if !section.is_spatial() {
            available.extend(local.iter().copied());
        }
        let scenery = read_scenery_sections(&bytes, &MOST_WANTED).unwrap();
        let mut hooks = std::collections::HashMap::new();
        for sc in &scenery {
            for hook in &sc.override_hooks {
                let instance = &sc.instances[usize::from(hook.instance_index)];
                hooks.insert(u32::from(hook.override_index), (instance, sc.info_of(instance).unwrap()));
            }
        }
        for spawn in read_object_spawners(&bytes, &MOST_WANTED.spawners).unwrap() {
            let (instance, info) = hooks[&spawn.override_index];
            let Some(shape) = catalog.for_spawner(&info.name, &spawn) else {
                continue;
            };
            if !engines.contains(&shape.dynamics.as_ref().unwrap().event_sequencer) {
                continue;
            }
            let mut p = Placed::prop_only(Mat4::from_cols_array_2d(&instance.matrix_columns()), shape);
            p.bounds = blackbox_scene::Aabb::new(instance.bbox_min, instance.bbox_max);
            p.spawner = Some(spawn);
            p.parts = catalog.hierarchy(info.hierarchy_hash).map(|hierarchy| {
                let available = hierarchy
                    .nodes
                    .iter()
                    .enumerate()
                    .filter_map(|(i, n)| local.contains(&n.model_hash).then_some(i))
                    .collect();
                Arc::new(ModelParts { hierarchy, available, meshes: BTreeMap::new() })
            });
            placed.push(p);
        }
    }
    for p in &mut placed {
        if let Some(parts) = &mut p.parts {
            let parts = Arc::get_mut(parts).unwrap();
            parts.available.extend(
                parts
                    .hierarchy
                    .nodes
                    .iter()
                    .enumerate()
                    .filter_map(|(i, n)| available.contains(&n.model_hash).then_some(i))
                    .collect::<BTreeSet<_>>(),
            );
        }
    }
    Some(Installed { data, placed })
}

fn world(install: &Installed, centre: Vec3) -> (PropWorld, Vec<Placed>) {
    let mut placed: Vec<_> = install
        .placed
        .iter()
        .filter(|p| Vec3::from_array(p.spawner.unwrap().position).distance(centre) < 35.0)
        .cloned()
        .collect();
    let mut world = PropWorld::default();
    world.add_tile(0, &mut placed);
    (world, placed)
}

#[derive(Debug, PartialEq)]
struct Trace {
    bodies: Vec<u32>,
    models: Vec<u32>,
    stops: Vec<u32>,
    fragments: usize,
    hidden_roofs: usize,
}

fn run(install: &Installed, target: &Placed, fps: u32) -> Trace {
    run_frames(install, target, std::iter::repeat_n(1.0 / fps as f32, (fps * 10) as usize))
}

fn run_frames(install: &Installed, target: &Placed, frames: impl Iterator<Item = f32>) -> Trace {
    let centre = Vec3::from_array(target.spawner.unwrap().position);
    let (mut world, placed) = world(install, centre);
    let id = target.spawner.unwrap().unique_id + 1;
    strike(&mut world, id);
    let mut clock = FixedClock::default();
    let mut trace = Trace { bodies: Vec::new(), models: Vec::new(), stops: Vec::new(), fragments: 0, hidden_roofs: 0 };
    let ground = FlatGround::new(centre.y - 0.5);
    for dt in frames {
        for _ in 0..clock.advance(dt) {
            world.step(&ground, STEP);
            for area in world.update_scenery(&install.data) {
                trace
                    .stops
                    .extend(area.position.to_array().into_iter().chain([area.radius, area.duration]).map(f32::to_bits));
            }
            let mut ids: Vec<_> = world.colliders.keys().copied().collect();
            ids.sort_unstable();
            for id in ids {
                let body = &world.colliders[&id].body.as_ref().unwrap().rigid;
                trace.bodies.push(id);
                trace.bodies.extend(
                    body.position
                        .to_array()
                        .into_iter()
                        .chain(body.orientation.to_array())
                        .chain(body.linear_velocity.to_array())
                        .chain(body.angular_velocity.to_array())
                        .chain(body.pending_force().to_array())
                        .map(f32::to_bits),
                );
                assert!(
                    body.position.is_finite() && body.linear_velocity.is_finite() && body.angular_velocity.is_finite()
                );
            }
            for (&id, model) in &world.scenery.models {
                assert!(!model.failed, "sequence failed for model {id}");
                trace.models.extend([id, model.hidden as u32, model.bounds_root as u32, model.render_root as u32]);
                trace.models.extend(&model.hidden_parts);
                if let (Some(seq), Some(player)) = (&model.sequence, &model.player) {
                    trace
                        .models
                        .extend(seq.package.engines[seq.engine].systems.keys().map(|&s| player.state(s).unwrap()));
                }
            }
            world.set_alpha(0.23);
        }
    }
    trace.fragments = world.fragment_count();
    trace.hidden_roofs = placed
        .iter()
        .filter(|p| {
            let seq = p.prop.as_ref().unwrap().dynamics.as_ref().unwrap().event_sequencer;
            [vlt_hash("donutshoproof_seq"), vlt_hash("gasstationroof_seq")].contains(&seq)
                && world.scenery.models[&p.prop_id].hidden
        })
        .count();
    world.remove_tile(0);
    assert_eq!(world.fragment_count(), 0);
    assert_eq!(world.len(), 0);
    assert!(world.scenery.models.is_empty());
    assert!(world.grid.is_empty());
    trace
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_breakers_follow_area_chains_and_fragment_traces_at_every_frame_rate() {
    let Some(install) = installed() else {
        return;
    };
    for (engine, minimum) in [("donutshoproofsupport_seq", 6), ("scaffoldsupport_seq", 2), ("gaspumpexplosion_seq", 15)]
    {
        let target = install
            .placed
            .iter()
            .find(|p| p.prop.as_ref().unwrap().dynamics.as_ref().unwrap().event_sequencer == vlt_hash(engine))
            .unwrap();
        let baseline = run(&install, target, 60);
        eprintln!(
            "{engine}: {} fragments, {} hidden roofs, {} stop words",
            baseline.fragments,
            baseline.hidden_roofs,
            baseline.stops.len()
        );
        assert!(baseline.fragments >= minimum, "{engine}: no installed collapse");
        if engine != "scaffoldsupport_seq" {
            assert!(baseline.hidden_roofs > 0, "{engine}: area chain did not hide the roof");
        }
        for fps in [5, 15, 30, 120, 144, 240] {
            assert_eq!(run(&install, target, fps), baseline, "{engine} at {fps} FPS");
        }
        let uneven = [0.001, 0.199, 0.008, 0.092, 0.003, 0.097].into_iter().cycle().take(150);
        assert_eq!(run_frames(&install, target, uneven), baseline, "{engine} with uneven frames");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_gas_pump_explosion_reports_a_petrol_blast() {
    let Some(install) = installed() else {
        return;
    };
    let target = install
        .placed
        .iter()
        .find(|p| {
            p.prop.as_ref().unwrap().dynamics.as_ref().unwrap().event_sequencer == vlt_hash("gaspumpexplosion_seq")
        })
        .unwrap();
    let centre = Vec3::from_array(target.spawner.unwrap().position);
    let (mut world, _) = world(&install, centre);
    strike(&mut world, target.spawner.unwrap().unique_id + 1);
    let ground = FlatGround::new(centre.y - 0.5);
    let mut clock = FixedClock::default();
    let mut blasts = Vec::new();
    let mut audio = crate::scenes::world::explosion_fx::ExplosionFx::default();
    let mut authored_count = 0;
    for _ in 0..600 {
        for _ in 0..clock.advance(1.0 / 60.0) {
            world.step(&ground, STEP);
            world.update_scenery(&install.data);
            let fallback = world.take_blasts();
            let authored = world.take_authored_blasts();
            assert!(
                authored.iter().all(|point| !fallback.contains(point)),
                "an authored blast must not also request the fallback visual/audio path"
            );
            authored_count += authored.len();
            let expected = fallback.len() + authored.len();
            // Mirror both production channels: authored particles suppress the
            // fallback visuals, while each physical blast still announces its audio.
            audio.request(fallback.into_iter().map(|point| space::to_render(point.to_array())));
            audio.announce_authored(authored.into_iter().map(|point| space::to_render(point.to_array())));
            let announced = audio.take_started();
            assert_eq!(announced.len(), expected, "each blast is announced once independently of visual loading");
            assert!(audio.take_started().is_empty(), "audio announcements drain once");
            blasts.extend(announced);
        }
    }
    assert!(!blasts.is_empty(), "a gas pump explosion must report a blast point");
    assert!(authored_count > 0, "installed authored gas-station particles must retain their audio notification");
    let rendered_centre = space::to_render(centre.to_array());
    assert!(blasts.iter().all(|p| p.distance(rendered_centre) < 20.0), "the blast starts at the pump, not far away");
}
