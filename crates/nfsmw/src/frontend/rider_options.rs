//! The rider rows of the Gameplay options: the riding style of each class of bike (`riding_style_road`, `riding_style_motard`,
//! `riding_style_offroad`) and the fingers on the brake and clutch levers (`brake_fingers`, `clutch_fingers`). RIDE 6's riding style
//! page offers the same choices (`docs/specs/ride6-rider-styles.md`, section 8; `ride6-rider-hands.md`).

use crate::settings::{BikeClass, BrakeFingers, ClutchFingers, Partial, RidingStyle, Settings};

use super::{Data, Title, cycle};

/// One rider row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RiderSetting {
    /// The riding style of one class of bike.
    Style(BikeClass),
    BrakeFingers,
    ClutchFingers,
}

impl RiderSetting {
    /// The rows in the order the page lists them.
    pub const ALL: [Self; 5] = [
        Self::Style(BikeClass::Road),
        Self::Style(BikeClass::Motard),
        Self::Style(BikeClass::Offroad),
        Self::BrakeFingers,
        Self::ClutchFingers,
    ];

    pub fn title(self) -> Title {
        Title::Key(match self {
            Self::Style(BikeClass::Road) => "options-riding-style-road",
            Self::Style(BikeClass::Motard) => "options-riding-style-motard",
            Self::Style(BikeClass::Offroad) => "options-riding-style-offroad",
            Self::BrakeFingers => "options-brake-fingers",
            Self::ClutchFingers => "options-clutch-fingers",
        })
    }

    pub fn data(self, s: &Settings) -> Data {
        Data::Text(match self {
            Self::Style(class) => s.riding_style_of(class).label(),
            Self::BrakeFingers => s.brake_fingers.label(),
            Self::ClutchFingers => s.clutch_fingers.label(),
        })
    }

    /// Moves to the next or the previous choice and records it for the config file.
    pub fn step(self, s: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
        match self {
            Self::Style(class) => {
                let at = RidingStyle::ALL.iter().position(|style| *style == s.riding_style_of(class)).unwrap_or(0);
                let style = RidingStyle::ALL[cycle(at, RidingStyle::ALL.len(), forward)];
                *s.riding_style_mut(class) = style;
                changed.set_riding_style(class, style);
            }
            Self::BrakeFingers => {
                let at = BrakeFingers::ALL.iter().position(|fingers| *fingers == s.brake_fingers).unwrap_or(0);
                s.brake_fingers = BrakeFingers::ALL[cycle(at, BrakeFingers::ALL.len(), forward)];
                changed.brake_fingers = Some(s.brake_fingers);
            }
            Self::ClutchFingers => {
                let at = ClutchFingers::ALL.iter().position(|fingers| *fingers == s.clutch_fingers).unwrap_or(0);
                s.clutch_fingers = ClutchFingers::ALL[cycle(at, ClutchFingers::ALL.len(), forward)];
                changed.clutch_fingers = Some(s.clutch_fingers);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_class_cycles_its_own_riding_style_and_records_it() {
        let (mut s, mut changed) = (Settings::from(Partial::default()), Partial::default());
        let motard = RiderSetting::Style(BikeClass::Motard);
        assert_eq!(motard.data(&s), Data::Text("Balanced".into()));
        assert!(motard.step(&mut s, &mut changed, true));
        assert_eq!(
            [s.riding_style_road, s.riding_style_motard, s.riding_style_offroad],
            [RidingStyle::Balanced, RidingStyle::ShouldersOut, RidingStyle::Balanced]
        );
        assert_eq!(changed.riding_style_motard, Some(RidingStyle::ShouldersOut));
        assert_eq!((changed.riding_style_road, changed.riding_style_offroad), (None, None));
        motard.step(&mut s, &mut changed, true);
        motard.step(&mut s, &mut changed, true);
        assert_eq!(s.riding_style_motard, RidingStyle::Balanced, "it wraps round");
        motard.step(&mut s, &mut changed, false);
        assert_eq!(s.riding_style_motard, RidingStyle::OldSchool, "and backwards");
        RiderSetting::Style(BikeClass::Offroad).step(&mut s, &mut changed, true);
        assert_eq!(RiderSetting::Style(BikeClass::Offroad).data(&s), Data::Text("Shoulders Out".into()));
    }

    #[test]
    fn the_fingers_step_through_the_values_the_original_offers() {
        let (mut s, mut changed) = (Settings::from(Partial::default()), Partial::default());
        assert_eq!(RiderSetting::BrakeFingers.data(&s), Data::Text("2".into()));
        let mut seen = Vec::new();
        for _ in 0..4 {
            RiderSetting::BrakeFingers.step(&mut s, &mut changed, true);
            seen.push(s.brake_fingers.count());
        }
        assert_eq!(seen, [4, 1, 2, 4], "1, 2 and 4, round and round");
        assert_eq!(changed.brake_fingers, Some(BrakeFingers::Four));
        RiderSetting::ClutchFingers.step(&mut s, &mut changed, true);
        assert_eq!((s.clutch_fingers, changed.clutch_fingers), (ClutchFingers::Four, Some(ClutchFingers::Four)));
        RiderSetting::ClutchFingers.step(&mut s, &mut changed, false);
        assert_eq!(s.clutch_fingers, ClutchFingers::Two);
    }

    #[test]
    fn every_row_has_its_own_title() {
        let keys: Vec<Title> = RiderSetting::ALL.iter().map(|setting| setting.title()).collect();
        assert!(keys.iter().all(|title| matches!(title, Title::Key(_))));
        let mut distinct = keys.clone();
        distinct.dedup();
        assert_eq!(distinct.len(), keys.len());
    }
}
