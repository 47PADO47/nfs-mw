//! The graphics-cost rows of the Video options (docs/low-end.md).

use super::options::{Data, Title};
use crate::settings::{CarShading, Partial, Settings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicsSetting {
    CarShading,
}

impl GraphicsSetting {
    pub const ALL: [Self; 1] = [Self::CarShading];

    pub fn title(self) -> Title {
        Title::Text(match self {
            Self::CarShading => "Car Shading",
        })
    }

    pub fn data(self, s: &Settings) -> Data {
        Data::Text(
            match self {
                Self::CarShading => match s.car_shading {
                    CarShading::Simple => "Simple",
                    CarShading::Glossy => "Glossy",
                },
            }
            .to_owned(),
        )
    }

    /// Moves to the next (or previous) value, wrapping, and records the change for the config file.
    pub fn step(self, s: &mut Settings, changed: &mut Partial, _forward: bool) -> bool {
        let before = *s;
        match self {
            Self::CarShading => {
                s.car_shading = match s.car_shading {
                    CarShading::Simple => CarShading::Glossy,
                    CarShading::Glossy => CarShading::Simple,
                };
                changed.car_shading = Some(s.car_shading);
            }
        }
        before != *s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn car_shading_toggles_and_records_only_its_change() {
        let mut s = Settings::from(Partial::default());
        let mut changed = Partial::default();
        assert_eq!(GraphicsSetting::CarShading.data(&s), Data::Text("Glossy".into()));
        assert!(GraphicsSetting::CarShading.step(&mut s, &mut changed, true));
        assert_eq!(GraphicsSetting::CarShading.data(&s), Data::Text("Simple".into()));
        assert_eq!(changed, Partial { car_shading: Some(CarShading::Simple), ..Partial::default() });
        assert!(GraphicsSetting::CarShading.step(&mut s, &mut changed, false));
        assert_eq!(s.car_shading, CarShading::Glossy);
    }
}
