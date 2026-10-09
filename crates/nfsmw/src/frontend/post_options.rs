//! The post-processing rows of the Video options (docs/post-processing.md).

use super::options::{Data, Title};
use crate::settings::{Partial, PostAa, PostBloom, PostTonemap, Settings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostSetting {
    Tonemap,
    Bloom,
    Aa,
}

impl PostSetting {
    pub const ALL: [Self; 3] = [Self::Tonemap, Self::Bloom, Self::Aa];

    pub fn title(self) -> Title {
        Title::Text(match self {
            Self::Tonemap => "Tone Mapping",
            Self::Bloom => "Bloom",
            Self::Aa => "Anti-Aliasing",
        })
    }

    pub fn data(self, s: &Settings) -> Data {
        Data::Text(
            match self {
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
                s.post_aa = pick(&[PostAa::Off, PostAa::Fxaa], s.post_aa, forward);
                changed.post_aa = Some(s.post_aa);
            }
        }
        before != *s
    }
}

fn pick<T: Copy + PartialEq>(all: &[T], current: T, forward: bool) -> T {
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
        PostSetting::Aa.step(&mut s, &mut changed, false);
        assert_eq!((s.post_aa, PostSetting::Aa.data(&s)), (PostAa::Fxaa, Data::Text("FXAA".into())));
        assert_eq!(
            changed,
            Partial { post_tonemap: Some(PostTonemap::Aces), post_aa: Some(PostAa::Fxaa), ..Partial::default() }
        );
    }
}
