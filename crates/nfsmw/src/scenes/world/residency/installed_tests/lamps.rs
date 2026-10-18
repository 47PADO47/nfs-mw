//! Installed scenery stays visible without inventing a physics placement.

use std::collections::HashMap;

use blackbox_attrib::Database;
use blackbox_gfx::{Capabilities, GraphicsApi};
use game_install::GameDir;
use glam::Mat4;
use nfsmw_data::world::{DEFAULT_TRACK, PropCatalog, PropKind, SectionData, WorldIndex, parse_section};

use crate::app::test_backend::NullBackend;
use crate::scenes::world::props::PropWorld;
use crate::scenes::world::resident::{SectionResources, place};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_gray_point_lamps_need_a_spawner_to_have_a_prop_body() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let catalog = PropCatalog::new(index.prop_bounds.clone(), &db);
    let tile = index.by_name("Q51").expect("fixture needs the Gray Point scenery tile");
    let data = read_section(&dir, &index, tile);
    let spawners: HashMap<_, _> = data.spawners.iter().map(|s| (s.override_index, s)).collect();
    let mut static_positions = Vec::new();
    let mut dynamic_positions = Vec::new();
    for section in &data.scenery {
        let hooks: HashMap<_, _> = section
            .override_hooks
            .iter()
            .map(|h| (usize::from(h.instance_index), u32::from(h.override_index)))
            .collect();
        for (i, inst) in section.instances.iter().enumerate() {
            let Some(info) = section.info_of(inst) else { continue };
            let rules = &blackbox_scenery::layout::MOST_WANTED.visibility;
            if info.name != "XO_StreetLightCb_1b_00" || !inst.visible_in(rules.player_view, rules) {
                continue;
            }
            assert!(catalog.shape(&info.name).is_some(), "fixture must expose the tempting name-based fallback");
            let at = Mat4::from_cols_array_2d(&inst.matrix_columns()).w_axis.truncate();
            if hooks.get(&i).is_some_and(|id| spawners.contains_key(id)) {
                dynamic_positions.push(at);
                continue;
            }
            static_positions.push(at);
        }
    }
    assert!(
        !static_positions.is_empty() && !dynamic_positions.is_empty(),
        "tile must include both real placement types"
    );
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let empty = SectionResources::default();
    let mut shared = SectionResources::default();
    for i in index.shared() {
        let data = read_section(&dir, &index, i);
        shared.upload_textures(&mut renderer, &data);
        shared.upload_meshes(&mut renderer, &data, &empty);
    }
    let mut own = SectionResources::default();
    own.upload_textures(&mut renderer, &data);
    own.upload_meshes(&mut renderer, &data, &shared);
    let (mut placed, _) = place(&data, &own, &shared, &catalog, &Default::default());
    for &at in &static_positions {
        let p = placed
            .iter()
            .find(|p| p.transform.w_axis.truncate().distance(at) < 0.002)
            .expect("lamp must stay rendered");
        assert!(p.lods.iter().any(Option::is_some), "installed lamp must retain an uploaded mesh");
        assert!(p.spawner.is_none(), "fixture must lack the authored physics placement");
        assert!(p.prop.is_none(), "an unspawned visible lamp must not gain a guessed permanent barrier");
    }
    for &at in &dynamic_positions {
        let p = placed
            .iter()
            .find(|p| p.transform.w_axis.truncate().distance(at) < 0.002)
            .expect("dynamic lamp must stay rendered");
        let shape = p.prop.as_ref().expect("authored dynamic lamp retains collision");
        let dynamics = shape.dynamics.as_ref().expect("dynamic lamp retains its exact attributes");
        assert!(p.spawner.is_some() && p.lods.iter().any(Option::is_some));
        assert!(matches!(shape.kind, PropKind::Movable { .. }));
        assert!(!dynamics.immobile && dynamics.mass > 0.0 && dynamics.detach_impulse > 0.0);
    }
    let mut props = PropWorld::default();
    props.add_tile(tile, &mut placed);
    for at in static_positions {
        let p = placed.iter().find(|p| p.transform.w_axis.truncate().distance(at) < 0.002).unwrap();
        assert_eq!(p.prop_id, 0, "static visible lamp must not enter the prop collision grid");
    }
    for at in dynamic_positions {
        let p = placed.iter().find(|p| p.transform.w_axis.truncate().distance(at) < 0.002).unwrap();
        assert_eq!(p.prop_id, p.spawner.unwrap().unique_id + 1);
        let actual = props.pose(p.prop_id).expect("dynamic lamp needs an actual runtime body").0;
        assert!(actual.w_axis.truncate().distance(p.transform.w_axis.truncate()) < 0.002);
    }
    own.release(&mut renderer);
    shared.release(&mut renderer);
}

fn read_section(dir: &GameDir, index: &WorldIndex, i: usize) -> SectionData {
    let section = &index.sections[i];
    let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
    parse_section(i, &bytes).unwrap()
}
