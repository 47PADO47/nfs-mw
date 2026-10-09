//! The original EA Trax chyron: `EA_TRAX.fng` (in `GLOBAL/GlobalB.lzc`), the disc and tab the game shows when a song
//! starts. The package runs its own sequence (the tab slides in, the disc turns, the text holds for six seconds), so
//! the rewrite only fills in the title, artist and album and posts the message that starts it.
//!
//! The radio's serial changes when a song starts, pauses or resumes, so the chyron comes up for each of them, as
//! the card does (`docs/specs/music-graph.md` section 7). The screen has no line for the pause state.

use blackbox_feng::{ObjectRef, PackageId, Runtime};
use game_install::GameDir;

use blackbox_feng::UiTree;

use super::radio::Card;
use super::viewport::HudViewport;
use crate::settings::HudLayout;
use crate::ui::Catalog;
use crate::ui::present::Screen;

/// The package and the file that holds it.
const TRAX_PACKAGE: &str = "EA_TRAX.fng";
const TRAX_FILES: [&str; 1] = ["GLOBAL/GlobalB.lzc"];
/// The text objects of the disc's block, by GUID (the labels of the screen's dump: title, artist, album).
const TITLE_GUID: u32 = 0xF457A;
const ARTIST_GUID: u32 = 0xF457F;
const ALBUM_GUID: u32 = 0xF4589;
/// The message that starts the sequence. Its targets are the tab, the disc and the six-second timer. The two
/// messages next to it (`0x742186E0`, `0x742186E1`) reach the same objects with different tab motion.
const SHOW_MESSAGE: u32 = 0x742186DF;
/// The layout message the screen's own layout group answers (the same as the HUD's `NORMAL_MODE`).
const NORMAL_MODE: u32 = 0x53EC068C;
/// The messages that fade the text of the chyron in after the show message (the text group answers them).
const FOLLOW_UP: [u32; 2] = [0x835C45E8, 0x73D17048];
/// The group of the text. Its own fade track stays at zero in this build, so the text is faded in and out here: up
/// over `FADE_IN_SECS`, held, and down over the last `FADE_OUT_SECS` of the screen's six-second timer.
const TEXT_GUID: u32 = 0xF4778;
const HOLD_SECS: f32 = 6.0;
const FADE_IN_SECS: f32 = 0.4;
const FADE_OUT_SECS: f32 = 1.0;

/// The chyron's objects and the announcement it last showed.
pub struct TraxBinding {
    package: PackageId,
    title: Option<ObjectRef>,
    artist: Option<ObjectRef>,
    album: Option<ObjectRef>,
    /// The radio serial of the announcement last started.
    started: Option<u32>,
    text: Option<ObjectRef>,
    shown_at: Option<std::time::Instant>,
    /// The package's placement on the HUD, as the HUD's left side is placed.
    viewport: HudViewport,
}

/// Loads the chyron into the HUD's runtime. `None` when the install has no `EA_TRAX.fng`.
pub fn load(dir: &GameDir, runtime: &mut Runtime) -> Option<TraxBinding> {
    let catalog = Catalog::load(dir, &TRAX_FILES);
    let Some(package) = catalog.find(TRAX_PACKAGE).cloned() else {
        log::warn!("the EA Trax chyron is off: the install has no {TRAX_PACKAGE}");
        return None;
    };
    let package = runtime.load(package);
    for guid in [0xF4651u32, 0xF61E5, 0xF67FC] {
        if let Some(o) = runtime.find_guid(package, guid) {
            runtime.set_hidden(o, false);
        }
    }
    // The screen's layout is set by the mode messages the HUD also gets (the normal 4:3 layout).
    runtime.post_to_package(package, NORMAL_MODE);
    Some(TraxBinding::new(runtime, package))
}

impl TraxBinding {
    fn new(rt: &Runtime, package: PackageId) -> Self {
        Self {
            viewport: HudViewport::all_left(&rt.tree(package)),
            package,
            title: rt.find_guid(package, TITLE_GUID),
            artist: rt.find_guid(package, ARTIST_GUID),
            album: rt.find_guid(package, ALBUM_GUID),
            started: None,
            text: rt.find_guid(package, TEXT_GUID),
            shown_at: None,
        }
    }

    /// After the runtime's update: sets the text group's alpha for the time since the announcement.
    pub fn keep_text(&self, rt: &mut Runtime) {
        let (Some(text), Some(at)) = (self.text, self.shown_at) else { return };
        let t = at.elapsed().as_secs_f32();
        if t >= HOLD_SECS {
            return;
        }
        let alpha = (255.0 * fade(t)).round() as u8;
        rt.set_alpha(text, alpha);
    }

    /// Puts the chyron where the HUD's left side goes: the layout's scale and, for widescreen, its shift.
    pub fn place(&self, tree: &mut UiTree, screen: Screen, layout: HudLayout) {
        self.viewport.apply(tree, screen, layout);
    }

    /// The package the chyron is drawn from.
    pub fn package(&self) -> PackageId {
        self.package
    }

    /// One frame, before `Runtime::update`. `now` is the song on the air and its radio serial; `None` when nothing
    /// plays. A new serial fills the text and starts the sequence again.
    pub fn apply(&mut self, rt: &mut Runtime, now: Option<(&Card, u32)>) {
        let Some((card, serial)) = now else {
            self.started = None;
            return;
        };
        if self.started == Some(serial) {
            return;
        }
        self.started = Some(serial);
        self.shown_at = Some(std::time::Instant::now());
        let album = album_line(card);
        let lines =
            [(self.title, card.title.as_str()), (self.artist, card.artist.as_str()), (self.album, album.as_str())];
        for (object, text) in lines {
            if let Some(object) = object {
                rt.set_text(object, text);
            }
        }
        rt.post_to_package(self.package, SHOW_MESSAGE);
        for message in FOLLOW_UP {
            rt.post_to_package(self.package, message);
        }
    }
}

/// The album line. The screen has no line for the pause state, so a held song says so after its album.
fn album_line(card: &Card) -> String {
    match (card.paused, card.album.is_empty()) {
        (false, _) => card.album.clone(),
        (true, true) => "PAUSED".into(),
        (true, false) => format!("{}  PAUSED", card.album),
    }
}

/// The text's alpha, from 0 to 1, `t` seconds after the announcement.
fn fade(t: f32) -> f32 {
    let up = (t / FADE_IN_SECS).clamp(0.0, 1.0);
    let down = ((HOLD_SECS - t) / FADE_OUT_SECS).clamp(0.0, 1.0);
    up.min(down)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_fades_in_holds_and_fades_out() {
        assert_eq!(fade(0.0), 0.0);
        assert!((fade(FADE_IN_SECS / 2.0) - 0.5).abs() < 1e-6);
        assert_eq!(fade(3.0), 1.0);
        assert!((fade(HOLD_SECS - FADE_OUT_SECS / 2.0) - 0.5).abs() < 1e-6);
        assert_eq!(fade(HOLD_SECS), 0.0);
    }

    #[test]
    fn a_paused_song_says_so_after_its_album() {
        let card = |album: &str, paused| Card {
            title: "T".into(),
            artist: "A".into(),
            album: album.into(),
            progress: "0:00 / 0:00".into(),
            paused,
        };
        assert_eq!(album_line(&card("Megadef", false)), "Megadef");
        assert_eq!(album_line(&card("Megadef", true)), "Megadef  PAUSED");
        assert_eq!(album_line(&card("", true)), "PAUSED");
    }
}
