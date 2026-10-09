//! The minimap's data: the map tiles in `TRACKS/L2RA/` and the calibration of the open city in the track table
//! (`GLOBAL/GlobalB.lzc`). Layouts: `docs/formats/minimap.md`.

use anyhow::{Context, Result, anyhow};
use blackbox_chunk::{find, ids::TRACK_INFOS};
use blackbox_minimap::{Calibration, Grid, TileSet};
use game_install::GameDir;

use crate::read_unwrapped;

/// The track number of the open city (free roam and the races inside it).
pub const OPEN_CITY: i32 = 2000;
/// The file with the track table.
const TRACK_TABLE_FILE: &str = "GLOBAL/GlobalB.lzc";
/// The whole-city map file, without folder and extension.
pub const FULL_MAP: &str = "MINI_MAP";
/// The map picture is 8 x 8 tiles and the HUD draws a tile 128 units wide.
pub const GRID: Grid = Grid { tiles_per_side: 8, tile_size: 128.0 };
/// The game's `MinimapMaxSpeed`, m/s: the speed at which its zoom rule would end (it has no effect).
pub const MAX_SPEED: f32 = 100.0;

/// One record of the track table.
const RECORD_SIZE: usize = 0x120;
const NAME: std::ops::Range<usize> = 0x00..0x20;
const DIRECTORY: std::ops::Range<usize> = 0x20..0x40;
const NUMBER: usize = 0x8A;
const MAP_ORIGIN: usize = 0xAC;
const MAP_WIDTH: usize = 0xB4;

/// What the minimap needs of a track.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackInfo {
    pub name: String,
    pub directory: String,
    pub number: i32,
    pub map: Calibration,
}

fn text(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn f32_at(record: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([record[at], record[at + 1], record[at + 2], record[at + 3]])
}

/// The records of a `TrackInfos` payload. A trailing partial record is ignored.
pub fn parse_track_infos(payload: &[u8]) -> Vec<TrackInfo> {
    payload
        .as_chunks::<RECORD_SIZE>()
        .0
        .iter()
        .map(|r| TrackInfo {
            name: text(&r[NAME]),
            directory: text(&r[DIRECTORY]),
            number: i32::from(i16::from_le_bytes([r[NUMBER], r[NUMBER + 1]])),
            map: Calibration {
                origin: [f32_at(r, MAP_ORIGIN), f32_at(r, MAP_ORIGIN + 4)],
                width: f32_at(r, MAP_WIDTH),
            },
        })
        .collect()
}

/// The track table of the install.
pub fn read_track_infos(dir: &GameDir) -> Result<Vec<TrackInfo>> {
    let file = read_unwrapped(dir, TRACK_TABLE_FILE)?;
    let chunk = find(&file, TRACK_INFOS).ok_or_else(|| anyhow!("{TRACK_TABLE_FILE} has no TrackInfos chunk"))?;
    Ok(parse_track_infos(chunk.payload))
}

/// The calibration of the open city's map.
pub fn open_city_calibration(dir: &GameDir) -> Result<Calibration> {
    let infos = read_track_infos(dir)?;
    let info = infos.iter().find(|t| t.number == OPEN_CITY).ok_or_else(|| anyhow!("no track {OPEN_CITY}"))?;
    anyhow::ensure!(info.map.is_valid(), "track {OPEN_CITY} has invalid map calibration: {:?}", info.map);
    Ok(info.map)
}

/// The tiles of a map file of `TRACKS/L2RA/` (`MINI_MAP` for the whole city), 64 of them.
pub fn read_tiles(dir: &GameDir, file_stem: &str) -> Result<TileSet> {
    let rel = format!("TRACKS/L2RA/{file_stem}.BIN");
    let tiles = TileSet::parse(&read_unwrapped(dir, &rel)?).with_context(|| format!("reading {rel}"))?;
    let expected = (GRID.tiles_per_side * GRID.tiles_per_side) as usize;
    anyhow::ensure!(tiles.len() == expected, "{rel} has {} tiles, not {expected}", tiles.len());
    Ok(tiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(name: &str, number: i16, origin: [f32; 2], width: f32) -> Vec<u8> {
        let mut r = vec![0u8; RECORD_SIZE];
        r[..name.len()].copy_from_slice(name.as_bytes());
        r[0x20..0x20 + 4].copy_from_slice(b"Dir\\");
        r[NUMBER..NUMBER + 2].copy_from_slice(&number.to_le_bytes());
        r[MAP_ORIGIN..MAP_ORIGIN + 4].copy_from_slice(&origin[0].to_le_bytes());
        r[MAP_ORIGIN + 4..MAP_ORIGIN + 8].copy_from_slice(&origin[1].to_le_bytes());
        r[MAP_WIDTH..MAP_WIDTH + 4].copy_from_slice(&width.to_le_bytes());
        r
    }

    #[test]
    fn records_give_the_name_the_number_and_the_calibration() {
        let mut payload = record("Most Wanted World", 2000, [-1224.5, -1591.25], 6659.0);
        payload.extend(record("Other", 2200, [1.0, 2.0], 3.0));
        payload.extend([0u8; 17]);
        let infos = parse_track_infos(&payload);
        assert_eq!(infos.len(), 2, "the partial record is dropped");
        assert_eq!(infos[0].name, "Most Wanted World");
        assert_eq!(infos[0].directory, "Dir\\");
        assert_eq!(infos[0].number, OPEN_CITY);
        assert_eq!(infos[0].map, Calibration { origin: [-1224.5, -1591.25], width: 6659.0 });
        assert_eq!(infos[1].number, 2200);
    }
}
