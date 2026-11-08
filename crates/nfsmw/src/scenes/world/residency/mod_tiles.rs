//! The map packs of the mods in the residency (docs/specs/mod-world-content.md, 2.5 and 2.9). A pack is a tile that no zone
//! lists: it loads while the camera is inside its anchor plus a margin, or while an interior holds it (`pin`). Each pack
//! has its own streamer, reading its own file, and its indices come after the install's (`mod_base`).

use std::sync::Arc;

use blackbox_collision::CollisionPack;
use game_install::GameDir;
use nfsmw_data::world::{SectionData, SectionReader, Streamer, StreamerEvent};

use super::{Residency, TileState};
use crate::modding::content::scenery::{MapPack, PackSource, assemble};

/// Loader threads of the streamer the installed packs share (the pieces of a world made when they are asked for).
const INSTALLED_THREADS: usize = 3;

/// The streamers of a world's map packs, the route to each pack (streamer, section in it) and the pack of each section.
pub(super) struct ModStreamers {
    pub streamers: Vec<Streamer>,
    /// Per pack: its streamer and its section in that streamer.
    pub routes: Vec<(usize, usize)>,
    /// Per streamer: the pack of each of its sections.
    pub owners: Vec<Vec<usize>>,
}

/// Starts the streamers of a world's map packs. A pack
/// that is a file has a streamer of its own, reading it from offset 0 as entry 0. The packs read through the install
/// (`PackSource::Installed`: a world made of many pieces, some made when asked for) share one streamer, whose sections are those
/// packs in order, so a world of hundreds of pieces does not start hundreds of threads.
pub(super) fn spawn_streamers(packs: &[MapPack]) -> ModStreamers {
    let mut streamers = Vec::new();
    let mut owners: Vec<Vec<usize>> = Vec::new();
    let mut routes = vec![(0, 0); packs.len()];
    let mut installed: Vec<(usize, &GameDir, &str)> = Vec::new();
    for (k, pack) in packs.iter().enumerate() {
        match &pack.source {
            PackSource::File(path) => {
                routes[k] = (streamers.len(), 0);
                owners.push(vec![k]);
                streamers.push(Streamer::spawn(path.clone(), vec![pack.section.clone()], 1));
            }
            PackSource::Installed { dir, file } => installed.push((k, dir, file)),
        }
    }
    if installed.is_empty() {
        return ModStreamers { streamers, routes, owners };
    }
    let shared = streamers.len();
    let sections = installed.iter().map(|&(k, _, _)| packs[k].section.clone()).collect();
    let files: Vec<(GameDir, String)> =
        installed.iter().map(|&(_, dir, file)| (dir.clone(), file.to_owned())).collect();
    let reader: Arc<SectionReader> = Arc::new(move |section| {
        let (dir, file) = files.get(section).ok_or_else(|| format!("no piece {section}"))?;
        dir.read(file).map_err(|error| format!("{file}: {error}"))
    });
    for (section, &(k, _, _)) in installed.iter().enumerate() {
        routes[k] = (shared, section);
    }
    owners.push(installed.iter().map(|&(k, _, _)| k).collect());
    streamers.push(Streamer::spawn_made(reader, sections, INSTALLED_THREADS));
    ModStreamers { streamers, routes, owners }
}

/// How far outside its anchor a pack stays loaded, metres (the spec's design value).
pub const MOD_MARGIN: f32 = 150.0;

impl Residency {
    /// Makes the collision grid the dynamic one of a second world (`Grid::empty` was passed to `adopt`): its map packs' instances
    /// are listed in it when they arrive.
    pub fn set_dynamic_grid(&mut self, on: bool) {
        self.dynamic_grid = on;
    }

    /// Whether the residency index `tile` belongs to a mod's map pack.
    pub(super) fn is_mod(&self, tile: usize) -> bool {
        tile >= self.mod_base
    }

    /// Whether the map pack at residency index `tile` is loaded and drawn.
    pub fn pack_resident(&self, tile: usize) -> bool {
        matches!(self.tiles.get(&tile), Some(TileState::Resident { .. }))
    }

    /// Keep (`on`) or release the map pack at `tile` for an interior. A pinned pack loads whatever the camera does.
    pub fn pin(&mut self, tile: usize, on: bool) {
        match on {
            true => self.pinned.insert(tile),
            false => self.pinned.remove(&tile),
        };
    }

    /// Which map packs the camera at (x, y) wants: those whose anchor is within [`MOD_MARGIN`], and the pinned ones.
    pub(super) fn update_mod_wanted(&mut self, x: f32, y: f32) {
        if self.mod_packs.is_empty() {
            return;
        }
        let near = (0..self.mod_packs.len())
            .map(|k| self.mod_base + k)
            .filter(|&tile| self.sections[tile].distance_to(x, y) <= MOD_MARGIN);
        let mut wanted = std::mem::take(&mut self.mod_wanted);
        wanted.clear();
        wanted.extend(near);
        wanted.extend(self.pinned.iter().copied());
        wanted.sort_unstable();
        wanted.dedup();
        self.mod_wanted = wanted;
    }

    /// Request the wanted map packs that are not in flight or loaded yet.
    pub(super) fn request_mod(&mut self) {
        for k in 0..self.mod_wanted.len() {
            let tile = self.mod_wanted[k];
            if self.tiles.contains_key(&tile) {
                continue;
            }
            self.tiles.insert(tile, TileState::Requested);
            let (streamer, section) = self.mod_routes[tile - self.mod_base];
            self.mod_streamers[streamer].request(section);
        }
    }

    /// Take in the results of the map packs' streamers. A parsed pack becomes its scenery and its collision here.
    pub(super) fn receive_mod(&mut self) {
        for streamer in 0..self.mod_streamers.len() {
            let events: Vec<StreamerEvent> = std::iter::from_fn(|| self.mod_streamers[streamer].poll()).collect();
            for event in events {
                match event {
                    StreamerEvent::Collision { .. } => {}
                    StreamerEvent::Loaded(mut data) => {
                        let Some(&k) = self.mod_owners[streamer].get(data.index) else { continue };
                        data.index = self.mod_base + k;
                        self.assemble_mod(&mut data, k);
                        self.handle(StreamerEvent::Loaded(data));
                    }
                    StreamerEvent::Failed { index, error } => {
                        let Some(&k) = self.mod_owners[streamer].get(index) else { continue };
                        log::warn!("map pack {}: {error}", self.mod_packs[k].key);
                        self.handle(StreamerEvent::Failed { index: self.mod_base + k, error });
                    }
                }
            }
        }
    }

    fn assemble_mod(&self, data: &mut SectionData, k: usize) {
        if let Err(reason) = assemble(data, &self.mod_packs[k]) {
            log::warn!("map pack {}: its objects are not placed: {reason}", self.mod_packs[k].key);
            data.scenery.clear();
            data.collision.clear();
        }
    }

    /// Insert a pack's collision: the grid knows the install's packs, a mod's pack is tested on every query.
    pub(super) fn insert_collision(&mut self, pack: CollisionPack, tile: usize) {
        if self.is_mod(tile) && self.dynamic_grid {
            return self.collision.insert_into_grid(pack);
        }
        if self.is_mod(tile) {
            return self.collision.insert_ungridded(pack);
        }
        self.collision.insert(pack);
    }

    /// Remove the collision a tile put in, the way it went in.
    pub(super) fn remove_collision(&mut self, section: u32, tile: usize) {
        if self.is_mod(tile) && self.dynamic_grid {
            self.collision.remove_from_grid(section);
            return;
        }
        if self.is_mod(tile) {
            self.collision.remove_ungridded(section);
            return;
        }
        if !self.traffic_owns_collision(section) {
            self.collision.remove(section);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use blackbox_chunk::ChunkWriter;
    use blackbox_scenery::build::{Placement, section as build_section};
    use blackbox_scenery::{layout::MOST_WANTED, write_section};
    use game_install::{GameDir, LazySource, lazy_stub};

    use super::*;
    use crate::modding::content::ModDir;
    use crate::modding::content::scenery::world_packs;

    const OWNER: &str = "pieces";

    /// A source that makes the section file of a piece: one scenery instance, numbered 1 (the game renumbers it).
    struct Pieces(std::sync::atomic::AtomicUsize);

    impl LazySource for Pieces {
        fn read(&self, key: &str) -> std::io::Result<Vec<u8>> {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if key != "piece/7" {
                return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "no such piece"));
            }
            let placement = Placement {
                name: "P7".to_owned(),
                solids: [1, 1, 0, 0],
                local_min: [-1.0; 3],
                local_max: [1.0; 3],
                radius: 2.0,
                position: [10.0, 20.0, 30.0],
                rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                exclude_flags: 0x60,
            };
            let section = build_section(1, &[placement]).unwrap();
            let mut out = ChunkWriter::new();
            write_section(&mut out, &section, &MOST_WANTED);
            Ok(out.finish().unwrap())
        }
    }

    #[test]
    fn a_piece_a_file_places_is_not_read_when_the_world_opens_and_is_made_when_its_streamer_asks() {
        let root = std::env::temp_dir().join(format!("nfsmw-pieces-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (install, mod_dir) = (root.join("install"), root.join("mod"));
        std::fs::create_dir_all(&install).unwrap();
        std::fs::create_dir_all(mod_dir.join("worlds/w")).unwrap();
        std::fs::create_dir_all(mod_dir.join("files/WORLDS/W")).unwrap();
        std::fs::write(
            mod_dir.join("worlds/w/p7.toml"),
            "[pack]\nfile = \"WORLDS/W/P7.BUN\"\nsection = 2707\nanchor = [10.0, 20.0, 0.0, 50.0]\nplaced = \"file\"\n",
        )
        .unwrap();
        std::fs::write(mod_dir.join("files/WORLDS/W/P7.BUN"), lazy_stub(OWNER, "piece/7")).unwrap();
        let mut dir = GameDir::open(&install).unwrap();
        dir.add_overlay(OWNER, mod_dir.join("files"), 0).unwrap();
        let source = Arc::new(Pieces(Default::default()));
        dir.add_lazy_source(OWNER, source.clone());

        let owner = ModDir { id: OWNER.to_owned(), dir: mod_dir, priority: 0 };
        let packs = world_packs(&dir, &owner, "w", &mut Default::default());
        assert_eq!(packs.len(), 1, "the piece is a pack");
        assert!(matches!(packs[0].source, PackSource::Installed { .. }));
        assert_eq!(source.0.load(std::sync::atomic::Ordering::SeqCst), 0, "nothing is made when the world opens");

        let ModStreamers { streamers, routes, owners } = spawn_streamers(&packs);
        assert_eq!((streamers.len(), routes, owners), (1, vec![(0, 0)], vec![vec![0]]));
        streamers[0].request(0);
        let mut loaded = None;
        for _ in 0..5000 {
            match streamers[0].poll() {
                Some(StreamerEvent::Loaded(data)) => {
                    loaded = Some(data);
                    break;
                }
                Some(StreamerEvent::Failed { error, .. }) => panic!("the piece did not load: {error}"),
                _ => std::thread::sleep(std::time::Duration::from_millis(1)),
            }
        }
        let mut data = loaded.expect("the piece loaded");
        assert_eq!(source.0.load(std::sync::atomic::Ordering::SeqCst), 1, "made once, when asked for");
        assemble(&mut data, &packs[0]).unwrap();
        assert_eq!(data.scenery.len(), 1);
        assert_eq!(data.scenery[0].section_number, 2707, "the file's scenery takes the pack's number");
        assert_eq!(data.scenery[0].instances[0].position, [10.0, 20.0, 30.0]);
        drop(streamers);
        let _ = std::fs::remove_dir_all(&root);
    }
}
