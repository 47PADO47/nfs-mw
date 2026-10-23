//! The event tile reader on synthetic tiles: no install is read.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use blackbox_feng::fe_hash_upper;
use blackbox_minimap::{TileSet, tile_name};
use blackbox_tpk::{AlphaUsage, PixelFormat, Texture};

use super::*;

fn tile(name: &str) -> Texture {
    Texture {
        name: name.to_owned(),
        name_hash: fe_hash_upper(name),
        width: 1,
        height: 1,
        mip_levels: 1,
        format: PixelFormat::Argb8888,
        compression_type: 32,
        alpha_usage: AlphaUsage::None,
        alpha_sorting: false,
        alpha_blend: 2,
        flags: 0,
        data: vec![0, 0, 0, 255],
        palette: Vec::new(),
    }
}

/// 64 tiles named after the stem, as a map file holds them.
fn tiles_of(stem: &str) -> TileSet {
    TileSet { tiles: (0..64).map(|n| tile(&tile_name(stem, n))).collect() }
}

/// A loader that serves the stems in `files`, counting every call.
fn loader(files: &'static [&'static str], calls: &Arc<AtomicUsize>) -> TileLoader {
    let calls = calls.clone();
    Arc::new(move |stem| {
        calls.fetch_add(1, Ordering::SeqCst);
        match files.contains(&stem) {
            true => Ok(tiles_of(stem)),
            false => Err(anyhow::anyhow!("{stem}.BIN is not in the install")),
        }
    })
}

/// Updates until the reader is no longer loading (the worker thread is quick, but it is a thread).
fn settle(tiles: &mut EventTiles, wanted: Option<&str>, assets: &UiAssets) {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        tiles.update(wanted, assets);
        let loading = matches!(tiles.state, State::Loading { .. });
        if !loading || Instant::now() > until {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn has(assets: &UiAssets, stem: &str, n: i32) -> bool {
    assets.texture(fe_hash_upper(&tile_name(stem, n))).is_some()
}

const FILES: &[&str] = &["MINI_MAP_10_2_1", "MINI_MAP_10_4_2_R"];

#[test]
fn nothing_is_read_until_a_race_asks() {
    let (calls, assets) = (Arc::new(AtomicUsize::new(0)), UiAssets::empty());
    let mut tiles = EventTiles::new(loader(FILES, &calls));
    settle(&mut tiles, None, &assets);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(tiles.ready_stem(), None);
    assert_eq!(assets.late_count(), 0);
}

#[test]
fn an_event_is_read_once_and_its_tiles_reach_the_assets() {
    let (calls, assets) = (Arc::new(AtomicUsize::new(0)), UiAssets::empty());
    let mut tiles = EventTiles::new(loader(FILES, &calls));
    settle(&mut tiles, Some("10.2.1"), &assets);
    assert_eq!(tiles.ready_stem(), Some("MINI_MAP_10_2_1"));
    assert_eq!(tiles.ready_event(), Some("10.2.1"));
    assert_eq!(assets.late_count(), 64);
    assert!((0..64).all(|n| has(&assets, "MINI_MAP_10_2_1", n)));
    for _ in 0..5 {
        tiles.update(Some("10.2.1"), &assets);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1, "a loaded event is not read again");
}

#[test]
fn the_reversed_route_reads_its_own_file() {
    let (calls, assets) = (Arc::new(AtomicUsize::new(0)), UiAssets::empty());
    let mut tiles = EventTiles::new(loader(FILES, &calls));
    settle(&mut tiles, Some("10.4.2.r"), &assets);
    assert_eq!(tiles.ready_stem(), Some("MINI_MAP_10_4_2_R"));
}

#[test]
fn the_tiles_go_when_the_race_does() {
    let (calls, assets) = (Arc::new(AtomicUsize::new(0)), UiAssets::empty());
    let mut tiles = EventTiles::new(loader(FILES, &calls));
    settle(&mut tiles, Some("10.2.1"), &assets);
    tiles.update(None, &assets);
    assert_eq!(tiles.ready_stem(), None);
    assert_eq!(assets.late_count(), 0, "the race ended: the tiles are dropped");
    assert!(!has(&assets, "MINI_MAP_10_2_1", 0));
}

#[test]
fn a_new_event_replaces_the_old_tiles() {
    let (calls, assets) = (Arc::new(AtomicUsize::new(0)), UiAssets::empty());
    let mut tiles = EventTiles::new(loader(FILES, &calls));
    settle(&mut tiles, Some("10.2.1"), &assets);
    settle(&mut tiles, Some("10.4.2.r"), &assets);
    assert_eq!(tiles.ready_stem(), Some("MINI_MAP_10_4_2_R"));
    assert_eq!(assets.late_count(), 64);
    assert!(!has(&assets, "MINI_MAP_10_2_1", 0));
    assert!(has(&assets, "MINI_MAP_10_4_2_R", 63));
}

#[test]
fn an_event_without_a_file_is_tried_once() {
    let (calls, assets) = (Arc::new(AtomicUsize::new(0)), UiAssets::empty());
    let mut tiles = EventTiles::new(loader(FILES, &calls));
    settle(&mut tiles, Some("19.8.1"), &assets);
    for _ in 0..5 {
        tiles.update(Some("19.8.1"), &assets);
    }
    assert_eq!(tiles.ready_stem(), None, "the stage's map stays");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(assets.late_count(), 0);
}

#[test]
fn a_race_that_ends_while_the_file_is_read_leaves_nothing_behind() {
    let (calls, assets) = (Arc::new(AtomicUsize::new(0)), UiAssets::empty());
    let mut tiles = EventTiles::new(loader(FILES, &calls));
    tiles.update(Some("10.2.1"), &assets);
    tiles.update(None, &assets);
    std::thread::sleep(Duration::from_millis(50));
    tiles.update(None, &assets);
    assert_eq!(tiles.ready_stem(), None);
    assert_eq!(assets.late_count(), 0);
}
