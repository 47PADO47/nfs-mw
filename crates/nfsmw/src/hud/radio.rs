//! The radio's card on the HUD (the `custom` radio HUD, see `settings::RadioHudStyle`): the song on the air (title,
//! artist and album, the time) for a few seconds when it changes, is paused or is resumed. The original shows the song the same way when it starts (a "chyron", spec
//! `docs/specs/music-graph.md` section 7); the package has no object for it, so the card is drawn as text.
//!
//! [`RadioHud`] is plain data (what to show and for how long); [`placement`] puts it where the HUD layout setting
//! puts the left side of the HUD; [`show`] is the thin egui view. Nothing here asks the radio anything: the plugin
//! copies `Audio::now_playing()` into the data every frame.

use bevy_ecs::prelude::*;
use bevy_time::Time;
use egui::{Color32, RichText, pos2};

use super::viewport;
use crate::audio::{Audio, NowPlaying};
use crate::settings::HudLayout;
use crate::ui::present::Screen;

/// How long a card stays after the radio changed, in seconds, and how much of that it fades out over.
const SHOW_SECS: f32 = 6.0;
const FADE_SECS: f32 = 1.0;

/// Where the card sits in the authored 4:3 HUD, in FEng units from the centre of the canvas: the left margin of the
/// HUD and the top of the screen. The layout moves and scales it like the left side of the HUD (the map).
const AUTHORED_LEFT: f32 = -300.0;
const AUTHORED_TOP: f32 = -228.0;
/// Text sizes in FEng units.
const TITLE_SIZE: f32 = 15.0;
const DETAIL_SIZE: f32 = 12.0;
/// Never closer to the window's edge than this many points.
const EDGE: f32 = 4.0;

/// What the card says.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    pub title: String,
    pub artist: String,
    pub album: String,
    /// `m:ss / m:ss`.
    pub progress: String,
    pub paused: bool,
}

impl Card {
    fn of(now: &NowPlaying) -> Self {
        Self {
            title: now.title.clone(),
            artist: now.artist.clone(),
            album: now.album.clone(),
            progress: now.progress(),
            paused: now.paused,
        }
    }

    /// The artist, and the album when the song has one.
    pub fn credit(&self) -> String {
        if self.album.is_empty() {
            return self.artist.clone();
        }
        format!("{} - {}", self.artist, self.album)
    }

    /// The time played, with the state when the song is held.
    pub fn status(&self) -> String {
        if self.paused {
            return format!("{}  PAUSED", self.progress);
        }
        self.progress.clone()
    }
}

/// The radio's card and how long it has been up.
#[derive(Resource, Debug, Default)]
pub struct RadioHud {
    card: Option<Card>,
    /// The `serial` of the song event shown last.
    seen: Option<u32>,
    /// Seconds since the card was last announced.
    age: f32,
}

impl RadioHud {
    /// One frame: `now` is the song on the air, `dt` the seconds since the last frame.
    pub fn update(&mut self, now: Option<&NowPlaying>, dt: f32) {
        let Some(now) = now else {
            *self = Self::default();
            return;
        };
        if self.seen != Some(now.serial) {
            self.seen = Some(now.serial);
            self.age = 0.0;
        }
        self.age += dt.max(0.0);
        self.card = Some(Card::of(now));
    }

    /// The song on the air, whether or not its card is up.
    pub fn song(&self) -> Option<&Card> {
        self.card.as_ref()
    }

    /// The serial of the announcement shown last (see `NowPlaying::serial`).
    pub fn serial(&self) -> Option<u32> {
        self.seen
    }

    /// The card to draw, while it is up.
    pub fn card(&self) -> Option<&Card> {
        self.card.as_ref().filter(|_| self.age < SHOW_SECS)
    }

    /// 1 while the card is up, falling to 0 over its last second.
    pub fn opacity(&self) -> f32 {
        ((SHOW_SECS - self.age) / FADE_SECS).clamp(0.0, 1.0)
    }
}

/// Copy the song on the air into the card.
pub fn update(audio: Option<NonSend<Audio>>, time: Res<Time>, mut hud: ResMut<RadioHud>) {
    hud.update(audio.as_deref().and_then(Audio::now_playing), time.delta_secs());
}

/// Where the card goes, in points from the window's top left, and how many points one FEng unit is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub left: f32,
    pub top: f32,
    pub unit: f32,
}

/// The layout's place for the card: the authored spot moved and scaled like the HUD's left side.
pub fn placement(screen: Screen, layout: HudLayout) -> Placement {
    let (scale, offset) = viewport::parameters(screen, layout);
    let unit = screen.scale() * scale;
    let left = screen.width / 2.0 + (AUTHORED_LEFT - offset) * unit;
    let top = screen.height / 2.0 + AUTHORED_TOP * unit;
    Placement { left: left.max(EDGE), top: top.max(EDGE), unit }
}

/// Draw the card, if one is up.
pub fn show(ctx: &egui::Context, hud: &RadioHud, screen: Screen, layout: HudLayout) {
    let Some(card) = hud.card() else { return };
    let at = placement(screen, layout);
    let opacity = hud.opacity();
    let ink = |c: Color32| c.gamma_multiply(opacity);
    egui::Area::new(egui::Id::new("radio_card")).fixed_pos(pos2(at.left, at.top)).interactable(false).show(ctx, |ui| {
        egui::Frame::new()
            .fill(Color32::from_black_alpha(150).gamma_multiply(opacity))
            .corner_radius(3.0 * at.unit)
            .inner_margin(egui::Margin::same((6.0 * at.unit) as i8))
            .show(ui, |ui| {
                let line = |text: String, size: f32, color: Color32| {
                    egui::Label::new(RichText::new(text).size(size * at.unit).color(ink(color)))
                        .wrap_mode(egui::TextWrapMode::Extend)
                };
                ui.add(line(card.title.clone(), TITLE_SIZE, Color32::WHITE));
                ui.add(line(card.credit(), DETAIL_SIZE, Color32::from_gray(200)));
                ui.add(line(card.status(), DETAIL_SIZE, Color32::from_gray(150)));
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now(serial: u32, paused: bool) -> NowPlaying {
        NowPlaying {
            song: 3,
            artist: "Artist".into(),
            title: "Title".into(),
            album: "Album".into(),
            elapsed_secs: 65.0,
            length_secs: 200.0,
            paused,
            serial,
        }
    }

    fn screen(width: f32, height: f32) -> Screen {
        Screen { width, height, pixels_per_point: 1.0 }
    }

    #[test]
    fn a_new_song_shows_the_card_for_a_few_seconds_then_fades_it() {
        let mut hud = RadioHud::default();
        assert!(hud.card().is_none());
        hud.update(Some(&now(1, false)), 0.016);
        let card = hud.card().unwrap();
        assert_eq!((card.title.as_str(), card.credit().as_str()), ("Title", "Artist - Album"));
        assert_eq!(card.status(), "1:05 / 3:20");
        assert_eq!(hud.opacity(), 1.0);
        hud.update(Some(&now(1, false)), SHOW_SECS - 0.5);
        assert!(hud.card().is_some() && (hud.opacity() - 0.5).abs() < 0.05, "{}", hud.opacity());
        hud.update(Some(&now(1, false)), 1.0);
        assert!(hud.card().is_none(), "the card is gone after {SHOW_SECS} s");
    }

    #[test]
    fn a_pause_or_a_new_song_brings_the_card_back() {
        let mut hud = RadioHud::default();
        hud.update(Some(&now(1, false)), 10.0);
        assert!(hud.card().is_none());
        hud.update(Some(&now(2, true)), 0.016);
        assert_eq!(hud.card().unwrap().status(), "1:05 / 3:20  PAUSED");
        assert_eq!(hud.opacity(), 1.0);
    }

    #[test]
    fn the_card_goes_when_the_song_does_and_comes_back_with_the_next() {
        let mut hud = RadioHud::default();
        hud.update(Some(&now(1, false)), 0.1);
        hud.update(None, 0.1);
        assert!(hud.card().is_none());
        // The same serial after a stop is a new announcement: the state was forgotten.
        hud.update(Some(&now(1, false)), 0.1);
        assert!(hud.card().is_some());
    }

    #[test]
    fn an_album_less_song_shows_only_the_artist() {
        let card = Card { album: String::new(), ..Card::of(&now(1, false)) };
        assert_eq!(card.credit(), "Artist");
    }

    #[test]
    fn the_card_follows_the_hud_layout_setting() {
        // 1080p, 16:9: 2.25 points per unit. The PC layout moves the left side out by 120 units.
        let wide = screen(1920.0, 1080.0);
        let pc = placement(wide, HudLayout::Pc);
        assert!((pc.left - 15.0).abs() < 1e-3 && (pc.top - 27.0).abs() < 1e-3 && (pc.unit - 2.25).abs() < 1e-6);
        // Classic keeps the centred 4:3 HUD: the card sits further in, at the same size.
        let classic = placement(wide, HudLayout::Classic);
        assert!((classic.left - 285.0).abs() < 1e-3 && classic.unit == pc.unit);
        // Xbox 360 scales the HUD by 0.92 about the centre: smaller text and a margin that moves in.
        let xbox = placement(wide, HudLayout::Xbox360);
        assert!((xbox.unit - 2.25 * 0.92).abs() < 1e-6);
        assert!(xbox.left > pc.left && xbox.top > pc.top);
        assert!((xbox.left - (960.0 - 420.0 * 0.92 * 2.25)).abs() < 1e-3);
    }

    #[test]
    fn the_card_stays_on_screen_in_narrow_and_odd_windows() {
        // 4:3 and narrower: all layouts are the centred, unscaled one, and the card never leaves the window.
        let narrow = placement(screen(300.0, 600.0), HudLayout::Pc);
        assert_eq!((narrow.left, narrow.top), (EDGE, 15.0), "pushed in from the left edge, 228 units above the centre");
        let square = placement(screen(480.0, 480.0), HudLayout::Xbox360);
        assert!(square.left >= EDGE && square.top >= EDGE);
        assert_eq!(placement(screen(1024.0, 768.0), HudLayout::Pc).unit, 1.6);
        let ultrawide = placement(screen(3440.0, 1440.0), HudLayout::Pc);
        assert!(ultrawide.left >= EDGE && ultrawide.left < 400.0, "{}", ultrawide.left);
    }
}
