//! Against a real install (set `NFSMW_GAME_DIR`). Counts are the measurements in
//! `docs/formats/road-network.md`.

use crate::{RoadNetwork, flags};

fn network() -> Option<RoadNetwork> {
    let dir = std::env::var_os("NFSMW_GAME_DIR")?;
    let path = std::path::Path::new(&dir).join("TRACKS/L2RA.BUN");
    let raw = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let data = ea_compress::unwrap(&raw).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Some(RoadNetwork::read(&data).expect("road network").expect("L2RA.BUN has a road network"))
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_road_network_counts() {
    let Some(net) = network() else { return };
    assert_eq!(net.nodes.len(), 4385);
    assert_eq!(net.segments.len(), 6538);
    assert_eq!(net.profiles.len(), 710);
    assert_eq!(net.roads.len(), 1308);
    let count = |flag| net.segments.iter().filter(|s| s.has(flag)).count();
    assert_eq!(count(flags::DECISION), 3463);
    assert_eq!(count(flags::CURVED), 5892);
    assert_eq!(count(flags::ONE_WAY), 104);
    assert_eq!(net.nodes.iter().filter(|n| n.segments.len() == 1).count(), 5);
}
