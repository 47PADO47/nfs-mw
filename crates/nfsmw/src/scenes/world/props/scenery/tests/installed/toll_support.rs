//! Lamps, stop signs and barrier arms on a collapsed toll booth must not hang where its roof was (installed data).

use super::*;
use crate::scenes::world::props::release::rests_on;

/// Every placed prop of one streaming section, with its render parts: art from the section or the shared packs.
fn section_placements(index: &WorldIndex, data: &PhysicsData, bytes: &[u8], shared: &BTreeSet<u32>) -> Vec<Placed> {
    let catalog =
        PropCatalog::new(index.prop_bounds.clone(), data.database()).with_hierarchies(index.model_hierarchies.clone());
    let mut local: BTreeSet<_> = blackbox_solid::read_solids(bytes).unwrap().into_iter().map(|s| s.name_hash).collect();
    local.extend(shared);
    let scenery = read_scenery_sections(bytes, &MOST_WANTED).unwrap();
    let mut hooks = std::collections::HashMap::new();
    for sc in &scenery {
        for hook in &sc.override_hooks {
            let instance = &sc.instances[usize::from(hook.instance_index)];
            hooks.insert(u32::from(hook.override_index), (instance, sc.info_of(instance).unwrap()));
        }
    }
    let mut placed = Vec::new();
    for spawn in read_object_spawners(bytes, &MOST_WANTED.spawners).unwrap() {
        let Some(&(instance, info)) = hooks.get(&spawn.override_index) else { continue };
        let Some(shape) = catalog.for_spawner(&info.name, &spawn) else { continue };
        let mut p = Placed::prop_only(Mat4::from_cols_array_2d(&instance.matrix_columns()), shape);
        p.bounds = blackbox_scene::Aabb::new(instance.bbox_min, instance.bbox_max);
        p.spawner = Some(spawn);
        p.parts = catalog.hierarchy(info.hierarchy_hash).map(|hierarchy| {
            let available = (0..hierarchy.nodes.len()).filter(|i| local.contains(&hierarchy.nodes[*i].model_hash));
            let available = available.collect();
            Arc::new(ModelParts { hierarchy, available, meshes: BTreeMap::new() })
        });
        placed.push(p);
    }
    placed
}

fn booth_sequence(p: &Placed) -> bool {
    p.prop.as_ref().unwrap().dynamics.as_ref().unwrap().event_sequencer == vlt_hash("tollbooth_seq")
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_collapsed_toll_booths_leave_nothing_hanging_after_offscreen_release() {
    let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(path).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences.clone());
    let read = |section: &blackbox_streaming::StreamingSection| {
        dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap()
    };
    let shared: BTreeSet<u32> = index
        .sections
        .iter()
        .filter(|section| !section.is_spatial())
        .flat_map(|section| blackbox_solid::read_solids(&read(section)).unwrap().into_iter().map(|s| s.name_hash))
        .collect();
    let (mut booths, mut supported, mut knocked) = (0, 0, 0);
    for section in &index.sections {
        let placed = section_placements(&index, &data, &read(section), &shared);
        let targets: Vec<_> =
            placed.iter().filter(|p| booth_sequence(p)).map(|p| (p.spawner.unwrap(), p.bounds)).collect();
        for (booth, booth_bounds) in targets {
            booths += 1;
            let centre = Vec3::from_array(booth.position);
            let mut local: Vec<_> = placed
                .iter()
                .filter(|p| Vec3::from_array(p.spawner.unwrap().position).distance(centre) < 40.0)
                .cloned()
                .collect();
            let mut world = PropWorld::default();
            world.add_tile(0, &mut local);
            let other_booths: Vec<_> = local
                .iter()
                .filter(|p| p.spawner.unwrap().unique_id != booth.unique_id && booth_sequence(p))
                .map(|p| p.bounds)
                .collect();
            // Props on this booth alone: a lamp on its roof, an arm on its side, not one also held by the next booth.
            let on_booth: Vec<_> = local
                .iter()
                .filter(|p| p.spawner.unwrap().unique_id != booth.unique_id && rests_on(&p.bounds, &booth_bounds))
                .filter(|p| !other_booths.iter().any(|other| rests_on(&p.bounds, other)))
                .map(|p| (p.prop_id, world.colliders[&p.prop_id].body.as_ref().unwrap().rigid.position))
                .collect();
            supported += on_booth.len();
            // The detachment a fast car causes: the booth leaves its base and its sequence collapses it.
            let id = booth.unique_id + 1;
            let collider = world.colliders.get_mut(&id).unwrap();
            let spec = collider.shape.dynamics.clone().unwrap();
            let body = collider.body.as_mut().unwrap();
            body.attached = false;
            body.moved = true;
            body.rigid.linear_velocity = body.rigid.orientation * Vec3::Z * 25.0 + Vec3::Y * 2.0;
            body.rigid.wake();
            body.lifetime.activate(&spec);
            world.contact_event(id, true, 30.0);
            let ground = FlatGround::new(centre.y - 0.02);
            // Nobody looks: every offscreen allowance runs out.
            let away = Mat4::from_translation(Vec3::splat(100000.0));
            let mut fell = BTreeSet::new();
            for _ in 0..1800 {
                world.step(&ground, STEP);
                world.update_scenery(&data);
                world.update_lifetimes(&ground, away, STEP);
                for &(prop, start) in &on_booth {
                    let body = world.colliders[&prop].body.as_ref().unwrap();
                    if body.rigid.position.y < start.y - 1.0 {
                        fell.insert(prop);
                    }
                }
            }
            assert!(world.scenery.models[&id].hidden, "booth {} must collapse", booth.unique_id);
            knocked += fell.len();
            for &(prop, start) in &on_booth {
                let (model, body) = (&world.scenery.models[&prop], world.colliders[&prop].body.as_ref().unwrap());
                let hanging = !model.hidden && body.rigid.position.distance(start) < 0.01;
                assert!(
                    !hanging,
                    "section {} booth {}: prop {prop} was put back at {start:?} over the collapsed booth",
                    section.name, booth.unique_id
                );
            }
        }
    }
    eprintln!("{booths} toll booths, {supported} props on them, {knocked} knocked down");
    assert!(booths >= 40 && supported >= 20 && knocked >= 10, "{booths} booths, {supported} on them, {knocked} fell");
}
