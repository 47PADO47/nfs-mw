use blackbox_collision::CollisionWorld;
use game_install::GameDir;
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};

use super::Zones;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_start_has_local_collision_while_unrelated_tiles_can_load() {
    let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(path).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut zones = Zones::new(index.visible.clone(), &index.sections);
    zones.update(333.0, 2517.0);
    let wanted = zones.wanted();
    let mut collision = CollisionWorld::new(index.collision_grid);
    for &i in &wanted {
        let tile = &index.sections[i];
        let bytes = dir.read_range(&index.stream_file, u64::from(tile.file_offset), tile.size as usize).unwrap();
        let packs = blackbox_collision::read_collision_packs(&bytes).unwrap();
        for pack in packs {
            collision.insert(pack);
        }
    }
    let point = super::super::space::to_physics(glam::Vec3::new(333.0, 2517.0, 0.0));
    assert_eq!(zones.name().as_deref(), Some("A1"));
    assert!(!index.sections.iter().any(|s| s.number == 101), "zone names are not resident tile identities");
    assert_eq!(collision.resident_near(point, 20.0), Some(true));
    assert!(collision.remove(118).is_some());
    assert_eq!(collision.resident_near(point, 20.0), Some(true), "distant loading cannot freeze the car");
    assert!(collision.remove(101).is_some());
    assert_eq!(collision.resident_near(point, 20.0), Some(false), "missing local road must pause simulation");
}
