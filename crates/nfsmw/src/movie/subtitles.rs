//! The subtitles of a movie: which file belongs to it, whether its lines show at all, and the text of each line.
//! Spec: `docs/specs/subtitles.md` (the gate in section 4.1, the tutorials in 4.3) and `docs/formats/text.md#subtitles`.

use blackbox_text::SubtitleTrack;
use blackbox_text::subtitles::parse;
use egui::{Align2, Color32, RichText};
use game_install::GameDir;

/// The label of a clear record: a cue with it shows no text. It is in no string table.
const CLEAR: u32 = 0x001A_20BA;
/// The five tutorial movies. They show their subtitles whatever the language, matched by name.
const TUTORIALS: [&str; 5] =
    ["bounty_tutorial", "drag_tutorial", "pursuit_tutorial", "speedtrap_tutorial", "tollbooth_tutorial"];

/// The movie's name without its file suffix, lower case: `bounty_tutorial` for `bounty_tutorial_english_ntsc.vp6`
/// (or `_german_pal.vp6`; `docs/specs/localization.md` section 1.3). This is also the name of its subtitle file under
/// `SUBTITLES/`.
pub(super) fn movie_stem(file: &str) -> String {
    let lower = file.to_ascii_lowercase();
    let name = lower.strip_suffix(".vp6").unwrap_or(&lower);
    let Some(name) = name.strip_suffix("_ntsc").or_else(|| name.strip_suffix("_pal")) else { return name.to_owned() };
    let Some((stem, language)) = name.rsplit_once('_') else { return name.to_owned() };
    match nfsmw_data::languages::LANGUAGES.iter().any(|l| l.stem.eq_ignore_ascii_case(language)) {
        true => stem.to_owned(),
        false => name.to_owned(),
    }
}

fn is_tutorial(stem: &str) -> bool {
    TUTORIALS.contains(&stem)
}

/// Whether the subtitles of the movie `stem` (see [`movie_stem`]) load and show. The tutorials always do. Any other
/// movie shows them only when the game language is not English (`docs/specs/subtitles.md`, section 4.1).
pub fn wanted(stem: &str, language_is_english: bool) -> bool {
    if is_tutorial(stem) {
        return true;
    }
    !language_is_english
}

/// The track of the movie file `file` (as in the install's `MOVIES/`), or `None` when it has no subtitles to show: the
/// gate is shut, or there is no subtitle file for it.
pub(super) fn load(dir: &GameDir, file: &str) -> Option<SubtitleTrack> {
    let stem = movie_stem(file);
    if !wanted(&stem, crate::i18n::current().is_english()) {
        return None;
    }
    let bytes = dir.read(&format!("SUBTITLES/{stem}")).ok()?;
    let cues = parse(&bytes);
    let lookup = crate::i18n::game;
    if is_tutorial(&stem) {
        return Some(SubtitleTrack::with_tutorial_lines(&cues, CLEAR, lookup));
    }
    Some(SubtitleTrack::new(&cues, CLEAR, lookup))
}


/// The lines shown `elapsed` seconds into the movie: none without a track.
pub(super) fn lines_at(track: Option<&SubtitleTrack>, elapsed: f64) -> (Option<&str>, Option<&str>) {
    track.map_or((None, None), |track| track.lines_at(elapsed as f32))
}

/// Draws the lines centred near the bottom of the window, white on a dark box. Nothing is drawn when no line is up.
pub(crate) fn show(ctx: &egui::Context, (first, second): (Option<&str>, Option<&str>)) {
    let lines: Vec<&str> = [first, second].into_iter().flatten().collect();
    if lines.is_empty() {
        return;
    }
    egui::Area::new(egui::Id::new("movie_subtitles"))
        .anchor(Align2::CENTER_BOTTOM, [0.0, -40.0])
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(Color32::from_black_alpha(170))
                .corner_radius(4.0)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.set_max_width(ctx.content_rect().width() * 0.8);
                        for line in lines {
                            ui.label(RichText::new(line).size(22.0).color(Color32::WHITE));
                        }
                    });
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stem_drops_the_suffix_and_the_case() {
        assert_eq!(movie_stem("bounty_tutorial_english_ntsc.vp6"), "bounty_tutorial");
        assert_eq!(movie_stem("BLACKLIST_01_english_ntsc.VP6"), "blacklist_01");
        assert_eq!(movie_stem("ealogo_english_ntsc.vp6"), "ealogo");
    }

    #[test]
    fn english_shows_only_the_tutorials() {
        for tutorial in TUTORIALS {
            assert!(wanted(tutorial, true), "{tutorial} always shows");
        }
        assert!(!wanted("blacklist_01", true));
        assert!(!wanted("storyfmv_bus12", true));
        assert!(!wanted("psa", true));
    }

    #[test]
    fn other_languages_show_every_movies_subtitles() {
        assert!(wanted("blacklist_01", false));
        assert!(wanted("storyfmv_bus12", false));
        assert!(wanted("bounty_tutorial", false));
    }

    #[test]
    fn a_tutorial_is_matched_by_its_whole_name() {
        assert!(!wanted("bounty", true));
        assert!(!wanted("bounty_tutorial_2", true));
    }

    #[test]
    fn no_track_gives_no_lines_at_any_time() {
        assert_eq!(lines_at(None, 0.0), (None, None));
        assert_eq!(lines_at(None, 99.0), (None, None));
    }

    #[test]
    fn a_track_gives_its_lines_at_their_times() {
        let cues =
            [blackbox_text::Cue { start_tenths: 54, label: 7 }, blackbox_text::Cue { start_tenths: 146, label: CLEAR }];
        let track = SubtitleTrack::with_tutorial_lines(&cues, CLEAR, |label| (label == 7).then(|| "line".to_owned()));
        assert_eq!(lines_at(Some(&track), 5.3), (None, None), "before the first cue");
        assert_eq!(lines_at(Some(&track), 5.4), (Some("line"), None));
        assert_eq!(lines_at(Some(&track), 14.5), (Some("line"), None));
        assert_eq!(lines_at(Some(&track), 14.6), (None, None), "cleared");
    }
}
