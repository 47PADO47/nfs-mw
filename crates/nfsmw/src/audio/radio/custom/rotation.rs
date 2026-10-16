//! Which source the next track comes from: the EA Trax songs or the custom tracks (`docs/specs/custom-music.md`,
//! section 2.3).

use crate::settings::CustomMode;

/// Where a track comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// An EA Trax song.
    Trax,
    /// A custom track.
    Custom,
}

/// The source of the next track. `has_trax` and `has_custom` say which lists have tracks in this context; `last` is
/// the source of the track picked before. `None` when neither has any.
///
/// Replace: custom tracks whenever there are any. Mix: the sources alternate, EA Trax first; when only one side has
/// tracks, that side plays alone.
pub fn choose(mode: CustomMode, has_trax: bool, has_custom: bool, last: Option<Source>) -> Option<Source> {
    match (has_trax, has_custom) {
        (false, false) => None,
        (true, false) => Some(Source::Trax),
        (false, true) => Some(Source::Custom),
        (true, true) => Some(match mode {
            CustomMode::Replace => Source::Custom,
            CustomMode::Mix => match last {
                Some(Source::Trax) => Source::Custom,
                _ => Source::Trax,
            },
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Source::{Custom, Trax};

    #[test]
    fn with_no_custom_tracks_the_ea_trax_list_plays_as_before() {
        for mode in [CustomMode::Replace, CustomMode::Mix] {
            assert_eq!(choose(mode, true, false, None), Some(Trax));
            assert_eq!(choose(mode, true, false, Some(Custom)), Some(Trax));
            assert_eq!(choose(mode, false, false, None), None);
        }
    }

    #[test]
    fn replace_takes_the_custom_tracks_in_place_of_the_songs() {
        assert_eq!(choose(CustomMode::Replace, true, true, None), Some(Custom));
        assert_eq!(choose(CustomMode::Replace, true, true, Some(Custom)), Some(Custom));
        assert_eq!(choose(CustomMode::Replace, false, true, None), Some(Custom));
    }

    #[test]
    fn mix_alternates_starting_with_the_ea_trax_song() {
        let mut last = None;
        let mut sequence = Vec::new();
        for _ in 0..5 {
            let next = choose(CustomMode::Mix, true, true, last);
            sequence.push(next);
            last = next;
        }
        assert_eq!(sequence, [Some(Trax), Some(Custom), Some(Trax), Some(Custom), Some(Trax)]);
    }

    #[test]
    fn mix_with_one_empty_side_plays_the_other_alone() {
        assert_eq!(choose(CustomMode::Mix, true, false, Some(Trax)), Some(Trax));
        assert_eq!(choose(CustomMode::Mix, false, true, Some(Custom)), Some(Custom));
    }
}
