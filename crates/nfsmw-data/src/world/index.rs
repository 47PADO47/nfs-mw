//! The streaming index of a track, and its zones.

use anyhow::{Context, Result};
use blackbox_streaming::{StreamingSection, VisibleSections};
use game_install::GameDir;

use crate::read_unwrapped;

pub struct WorldIndex {
    pub track: String,
    pub sections: Vec<StreamingSection>,
    /// Zones and what each one loads and draws (`docs/specs/visible-sections.md`).
    pub visible: VisibleSections,
    /// Install-relative path of the stream file.
    pub stream_file: String,
}

impl WorldIndex {
    pub fn open(dir: &GameDir, track: &str) -> Result<Self> {
        let meta_file = format!("TRACKS/{track}.BUN");
        let meta = read_unwrapped(dir, &meta_file)?;
        let sections = blackbox_streaming::read_sections(&meta, &blackbox_streaming::layout::MOST_WANTED)
            .with_context(|| format!("reading the streaming index in {meta_file}"))?;
        let visible =
            blackbox_streaming::read_visible_sections(&meta, &blackbox_streaming::layout::MOST_WANTED_VISIBLE)
                .with_context(|| format!("reading the visible sections in {meta_file}"))?;
        log::info!(
            "{track}: {} sections ({} map tiles, {} shared), {} zones",
            sections.len(),
            sections.iter().filter(|s| s.is_spatial()).count(),
            sections.iter().filter(|s| !s.is_spatial()).count(),
            visible.drivable.len()
        );
        Ok(Self { track: track.to_owned(), sections, visible, stream_file: format!("TRACKS/STREAM{track}.BUN") })
    }

    /// Indices of the shared (non-spatial) sections.
    pub fn shared(&self) -> impl Iterator<Item = usize> + '_ {
        self.sections.iter().enumerate().filter(|(_, s)| !s.is_spatial()).map(|(i, _)| i)
    }

    /// Indices of the map tiles.
    pub fn tiles(&self) -> impl Iterator<Item = usize> + '_ {
        self.sections.iter().enumerate().filter(|(_, s)| s.is_spatial()).map(|(i, _)| i)
    }

    /// Centre of all map tiles, as a starting point for cameras.
    pub fn centre(&self) -> [f32; 2] {
        let tiles: Vec<_> = self.tiles().map(|i| self.sections[i].center).collect();
        let n = tiles.len().max(1) as f32;
        [tiles.iter().map(|c| c[0]).sum::<f32>() / n, tiles.iter().map(|c| c[1]).sum::<f32>() / n]
    }

    pub fn by_name(&self, name: &str) -> Option<usize> {
        self.sections.iter().position(|s| s.name.eq_ignore_ascii_case(name))
    }

    /// The index of the section with this number, if the stream has one.
    pub fn by_number(&self, number: i16) -> Option<usize> {
        self.sections.iter().position(|s| s.number == number)
    }
}
