//! The post-processing rows of the Video options (docs/post-processing.md).

use super::options::{Data, Title};
use crate::settings::motion_blur::MOTION_BLUR_STRENGTH_RANGE;
use crate::settings::{Partial, PostAa, PostBloom, PostTonemap, Settings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostSetting {
    Tonemap,
    Bloom,
    Aa,
    MotionBlur,
    MotionBlurStrength,
}

impl PostSetting {
    pub const ALL: [Self; 5] = [Self::Aa, Self::Tonemap, Self::Bloom, Self::MotionBlur, Self::MotionBlurStrength];

    pub fn title(self) -> Title {
        Title::Text(match self {
            Self::Tonemap => "SDR Tone Mapping",
            Self::Bloom => "Bloom",
            Self::Aa => "Anti-Aliasing",
            Self::MotionBlur => "Motion Blur (High Speed)",
            Self::MotionBlurStrength => "Motion Blur Strength",
        })
    }

    pub fn data(self, s: &Settings) -> Data {
        if self == Self::MotionBlurStrength {
            return Data::Text(format!("{}%", (s.motion_blur_strength * 100.0).round() as u32));
        }
        Data::Text(
            match self {
                Self::MotionBlur => match s.motion_blur {
                    true => "On",
                    false => "Off",
                },
                Self::MotionBlurStrength => unreachable!("slider handled above"),
                Self::Tonemap => match s.post_tonemap {
                    PostTonemap::Off => "Off",
                    PostTonemap::Aces => "ACES",
                },
                Self::Bloom => match s.post_bloom {
                    PostBloom::Off => "Off",
                    PostBloom::Low => "Low",
                    PostBloom::Medium => "Medium",
                    PostBloom::High => "High",
                },
                Self::Aa => match s.post_aa {
                    PostAa::Off => "Off",
                    PostAa::Fxaa => "FXAA",
                    PostAa::Smaa => "SMAA",
                    PostAa::Taa => "TAA",
                },
            }
            .to_owned(),
        )
    }

    /// Moves to the next (or previous) value, wrapping, and records the change for the config file.
    pub fn step(self, s: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
        let before = *s;
        match self {
            Self::Tonemap => {
                s.post_tonemap = pick(&[PostTonemap::Off, PostTonemap::Aces], s.post_tonemap, forward);
                changed.post_tonemap = Some(s.post_tonemap);
            }
            Self::Bloom => {
                let all = [PostBloom::Off, PostBloom::Low, PostBloom::Medium, PostBloom::High];
                s.post_bloom = pick(&all, s.post_bloom, forward);
                changed.post_bloom = Some(s.post_bloom);
            }
            Self::Aa => {
                s.post_aa = pick(&[PostAa::Off, PostAa::Fxaa, PostAa::Smaa, PostAa::Taa], s.post_aa, forward);
                changed.post_aa = Some(s.post_aa);
            }
            Self::MotionBlur => {
                s.motion_blur = !s.motion_blur;
                changed.motion_blur = Some(s.motion_blur);
            }
            Self::MotionBlurStrength => {
                let (lo, hi) = MOTION_BLUR_STRENGTH_RANGE;
                let step = match forward {
                    true => 0.1,
                    false => -0.1,
                };
                // Rounded to a tenth, so repeated steps do not drift.
                s.motion_blur_strength = (((s.motion_blur_strength + step) * 10.0).round() / 10.0).clamp(lo, hi);
                changed.motion_blur_strength = Some(s.motion_blur_strength);
            }
        }
        before != *s
    }
}

pub(super) fn pick<T: Copy + PartialEq>(all: &[T], current: T, forward: bool) -> T {
    let at = all.iter().position(|v| *v == current).unwrap_or(0);
    let next = if forward { (at + 1) % all.len() } else { (at + all.len() - 1) % all.len() };
    all[next]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_cycle_both_ways_and_record_only_their_own_change() {
        let mut s = Settings::from(Partial::default());
        let mut changed = Partial::default();
        assert_eq!(PostSetting::Bloom.data(&s), Data::Text("Off".into()));
        for expected in ["Low", "Medium", "High", "Off"] {
            assert!(PostSetting::Bloom.step(&mut s, &mut changed, true));
            assert_eq!(PostSetting::Bloom.data(&s), Data::Text(expected.into()));
        }
        assert!(PostSetting::Bloom.step(&mut s, &mut changed, false));
        assert_eq!(s.post_bloom, PostBloom::High, "left from off wraps to the last");
        assert_eq!(changed, Partial { post_bloom: Some(PostBloom::High), ..Partial::default() });

        let mut changed = Partial::default();
        PostSetting::Tonemap.step(&mut s, &mut changed, true);
        assert_eq!((s.post_tonemap, PostSetting::Tonemap.data(&s)), (PostTonemap::Aces, Data::Text("ACES".into())));
        PostSetting::Aa.step(&mut s, &mut changed, true);
        assert_eq!((s.post_aa, PostSetting::Aa.data(&s)), (PostAa::Fxaa, Data::Text("FXAA".into())));
        assert_eq!(
            changed,
            Partial { post_tonemap: Some(PostTonemap::Aces), post_aa: Some(PostAa::Fxaa), ..Partial::default() }
        );
    }

    #[test]
    fn the_motion_blur_rows_toggle_and_step_the_strength_within_its_range() {
        let mut s = Settings::from(Partial::default());
        let mut changed = Partial::default();
        assert_eq!(PostSetting::MotionBlurStrength.data(&s), Data::Text("100%".into()));
        assert!(PostSetting::MotionBlurStrength.step(&mut s, &mut changed, true));
        assert_eq!(PostSetting::MotionBlurStrength.data(&s), Data::Text("110%".into()));
        for _ in 0..30 {
            PostSetting::MotionBlurStrength.step(&mut s, &mut changed, true);
        }
        assert_eq!(s.motion_blur_strength, 2.0, "stops at the top of the range");
        assert!(PostSetting::MotionBlur.step(&mut s, &mut changed, true));
        assert_eq!(PostSetting::MotionBlur.data(&s), Data::Text("Off".into()));
        assert_eq!(changed.motion_blur, Some(false));
        assert_eq!(changed.motion_blur_strength, Some(2.0));
    }
}
