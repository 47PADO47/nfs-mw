//! The HD content rows of the Video options: HD fonts, the HD cursor and HD movies (`settings/hd_content.rs`). Each
//! corrects a bug that shows with replacement content and changes nothing for stock content, so each is on by default.

use crate::frontend::ids::{LABEL_OFF, LABEL_ON};
use crate::settings::{Partial, Settings};

use super::{Data, Title};

/// One HD content row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HdSetting {
    Fonts,
    Cursor,
    Movies,
}

impl HdSetting {
    pub fn title(self) -> Title {
        Title::Text(match self {
            Self::Fonts => "HD Fonts",
            Self::Cursor => "HD Cursor",
            Self::Movies => "HD Movies",
        })
    }

    fn on(self, s: &Settings) -> bool {
        match self {
            Self::Fonts => s.hd_content.fonts,
            Self::Cursor => s.hd_content.cursor,
            Self::Movies => s.hd_content.fmv,
        }
    }

    pub fn data(self, s: &Settings) -> Data {
        Data::Label(match self.on(s) {
            true => LABEL_ON,
            false => LABEL_OFF,
        })
    }

    /// Flips the option and records the new value for the config file. Left and right both flip it.
    pub fn step(self, s: &mut Settings, changed: &mut Partial, _forward: bool) -> bool {
        let value = !self.on(s);
        match self {
            Self::Fonts => {
                s.hd_content.fonts = value;
                changed.hd_content.fonts = Some(value);
            }
            Self::Cursor => {
                s.hd_content.cursor = value;
                changed.hd_content.cursor = Some(value);
            }
            Self::Movies => {
                s.hd_content.fmv = value;
                changed.hd_content.fmv = Some(value);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_hd_row_flips_its_own_option_and_records_it() {
        let (mut s, mut changed) = (Settings::from(Partial::default()), Partial::default());
        assert_eq!(HdSetting::Movies.data(&s), Data::Label(LABEL_ON));
        assert!(HdSetting::Movies.step(&mut s, &mut changed, true));
        assert_eq!((s.hd_content.fmv, s.hd_content.fonts, s.hd_content.cursor), (false, true, true));
        assert_eq!(changed.hd_content.fmv, Some(false));
        assert_eq!(changed.hd_content.fonts, None);
        assert_eq!(HdSetting::Movies.data(&s), Data::Label(LABEL_OFF));
        HdSetting::Fonts.step(&mut s, &mut changed, false);
        assert_eq!((s.hd_content.fonts, changed.hd_content.fonts), (false, Some(false)));
        HdSetting::Cursor.step(&mut s, &mut changed, true);
        assert_eq!(changed.hd_content.cursor, Some(false));
    }
}
