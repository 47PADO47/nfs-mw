//! Private installed-fixture and scenery inventory.
use super::super::*;

#[test]
#[ignore = "requires installed scenery"]
fn inspect_signal_lamp_fixtures() {
    use nfsmw_data::world::{WorldIndex, load_section};
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let index = WorldIndex::open(&dir, "L2RA").unwrap();
    let mut stream = dir.open_file(&index.stream_file).unwrap();
    let mut seen = std::collections::HashSet::new();
    for (i, section) in index.sections.iter().enumerate() {
        let Ok(data) = load_section(&mut stream, i, section) else { continue };
        for scenery in &data.scenery {
            for instance in &scenery.instances {
                let Some(info) = scenery.info_of(instance) else { continue };
                let at = Vec3::from_array(instance.position);
                if at.truncate().distance(glam::Vec2::new(1204.0, 2130.0)) < 100.0 {
                    let name = info.name.to_ascii_uppercase();
                    if name.contains("TRAFFIC") || name.contains("SIGNAL") || name.contains("LIGHT") {
                        eprintln!("near fixture {name} at {at:?}");
                    }
                }
            }
        }
        for solid in &data.solids {
            let name = solid.name.to_ascii_uppercase();
            if !(name.contains("TRAFFIC") && name.contains("LIGHT")) || !seen.insert(name.clone()) {
                continue;
            }
            for (group_index, group) in solid.groups.iter().enumerate() {
                let Some(indices) =
                    solid.indices.get(group.first_index as usize..(group.first_index + group.num_indices) as usize)
                else {
                    continue;
                };
                let mut lo = Vec3::splat(f32::INFINITY);
                let mut hi = Vec3::splat(f32::NEG_INFINITY);
                for &idx in indices {
                    if let Some(v) = solid.vertices.get((u32::from(idx) + group.base_vertex) as usize) {
                        let p = Vec3::from_array(v.position);
                        lo = lo.min(p);
                        hi = hi.max(p);
                    }
                }
                eprintln!(
                    "signal mesh {name} group={group_index} effect={} indices={} min={lo:?} max={hi:?}",
                    group.effect_id, group.num_indices
                );
            }
        }
    }
}

#[test]
#[ignore = "requires installed scenery"]
fn inspect_downtown_night_buildings() {
    use nfsmw_data::world::{WorldIndex, load_section};
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let index = WorldIndex::open(&dir, "L2RA").unwrap();
    let mut stream = dir.open_file(&index.stream_file).unwrap();
    let mut names = std::collections::BTreeSet::new();
    let mut meshes = Vec::new();
    for (i, section) in index.sections.iter().enumerate() {
        let Ok(data) = load_section(&mut stream, i, section) else { continue };
        for scenery in &data.scenery {
            for instance in &scenery.instances {
                let Some(info) = scenery.info_of(instance) else { continue };
                let name = info.name.to_ascii_uppercase();
                let at = Vec3::from_array(instance.position);
                if (name.starts_with("XB_") || name.starts_with("PAN_"))
                    && at.truncate().distance(glam::Vec2::new(1530.0, 600.0)) < 500.0
                {
                    names.insert(name.clone());
                    eprintln!("downtown placement {name} {at:?}");
                }
            }
        }
        for solid in &data.solids {
            let name = solid.name.to_ascii_uppercase();
            if name.starts_with("XB_") || name.starts_with("PAN_") {
                let groups: Vec<_> =
                    solid.groups.iter().map(|g| (g.effect_id, g.num_indices, g.diffuse_texture(solid))).collect();
                meshes.push((name, groups));
            }
        }
    }
    for (name, groups) in meshes {
        if names.contains(&name) {
            eprintln!("downtown mesh {name} groups {groups:?}");
        }
    }
    assert!(!names.is_empty());
}

#[test]
#[ignore = "requires installed scenery"]
fn inspect_night_highway_buildings() {
    use nfsmw_data::world::{WorldIndex, load_section};
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let index = WorldIndex::open(&dir, "L2RA").unwrap();
    let mut stream = dir.open_file(&index.stream_file).unwrap();
    let mut found = Vec::new();
    for (i, section) in index.sections.iter().enumerate() {
        let Ok(data) = load_section(&mut stream, i, section) else { continue };
        for solid in &data.solids {
            let name = solid.name.to_ascii_uppercase();
            if name.starts_with("XB_HAPARTMENT") || name.starts_with("XB_BEACHHOTEL") {
                let groups: Vec<_> =
                    solid.groups.iter().map(|g| (g.effect_id, g.num_indices, g.diffuse_texture(solid))).collect();
                eprintln!("building mesh {name} groups {groups:?}");
            }
        }
        for scenery in &data.scenery {
            for instance in &scenery.instances {
                let Some(info) = scenery.info_of(instance) else { continue };
                let at = Vec3::from_array(instance.position);
                let distance = at.truncate().distance(glam::Vec2::new(-302.0, 3691.0));
                if distance < 750.0 {
                    found.push((distance, info.name.clone(), at));
                }
            }
        }
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (distance, name, at) in found {
        eprintln!("near {distance:.1} {name} {at:?}");
    }
}
