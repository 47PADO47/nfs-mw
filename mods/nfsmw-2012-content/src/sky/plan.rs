//! Which of the 2012 game's cloud domes the sky variant uses, how they are named, and the three weathers built from the game's
//! timelines (`docs/formats/chameleon-environment.md`).

use anyhow::{Result, bail};
use chameleon_environment::Timeline;

/// A dome picture of `ENVIRONMENT.BNDL` the variant keeps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomeSpec {
    /// The name in the profile, and of the picture's file (upper case, `.DDS`).
    pub name: &'static str,
    /// The key of the texture (resource class 1).
    pub key: u32,
    /// Where along the picture the sun is painted, for the domes that have one **[read from the pictures]**: 0.5 is the middle.
    pub glow: Option<f32>,
}

/// The cloud domes the timelines name, with the hours they hold in the standard timeline.
pub const DOMES: [DomeSpec; 6] = [
    DomeSpec { name: "night", key: 0x3F34, glow: None },
    DomeSpec { name: "morning", key: 0x3F3C, glow: Some(0.5) },
    DomeSpec { name: "noon", key: 0x3F46, glow: None },
    DomeSpec { name: "afternoon", key: 0x3F40, glow: Some(0.5) },
    DomeSpec { name: "evening", key: 0x3F3E, glow: None },
    DomeSpec { name: "fair", key: 0x3F3A, glow: None },
];

/// The timeline most weather uses (23 of the 25 have the same dome at every hour): the first one of the bundle.
pub const STANDARD_TIMELINE: u32 = 0x69CBB;
/// The one timeline that holds a different dome (the bright cumulus) from noon to 20 h.
pub const FAIR_TIMELINE: u32 = 0x124583;
/// A cloud cover up to which the fair weather is drawn, and up to which the standard one is.
pub const FAIR_UP_TO: f32 = 0.30;
pub const CLOUDY_UP_TO: f32 = 0.70;
/// The overcast weather has no timeline of its own in the data: it is the standard one with the cloud cover multiplied and the light
/// dimmed **[our choice]**.
pub const OVERCAST_GAIN: f32 = 1.8;
pub const OVERCAST_DIM: f32 = 0.7;

/// The name of the dome with texture `key`.
pub fn dome_name(key: u32) -> Option<&'static str> {
    DOMES.iter().find(|dome| dome.key == key).map(|dome| dome.name)
}

/// One key of a weather: the hour, the dome's name and the tint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeySpec {
    pub hour: f32,
    pub dome: &'static str,
    pub tint: [f32; 3],
}

/// A weather of the profile.
#[derive(Debug, Clone, PartialEq)]
pub struct WeatherSpec {
    pub max_cover: f32,
    pub gain: f32,
    pub keys: Vec<KeySpec>,
}

fn keys(timeline: &Timeline, dim: f32) -> Result<Vec<KeySpec>> {
    let mut out = Vec::new();
    for key in &timeline.keys {
        let Some(dome) = dome_name(key.clouds.texture) else {
            bail!("timeline {:X} names a dome ({:X}) this converter does not know", timeline.id, key.clouds.texture);
        };
        // Hours rise in a timeline; two keys of one hour would not (the profile refuses them).
        if out.last().is_some_and(|last: &KeySpec| last.hour >= key.hour) {
            continue;
        }
        out.push(KeySpec { hour: key.hour, dome, tint: key.clouds.tint().map(|value| value * dim) });
    }
    Ok(out)
}

/// The fair, cloudy and overcast weathers from the bundle's timelines.
pub fn weathers(timelines: &[Timeline]) -> Result<Vec<WeatherSpec>> {
    let find = |id: u32| timelines.iter().find(|timeline| timeline.id == id);
    let (Some(standard), Some(fair)) = (find(STANDARD_TIMELINE), find(FAIR_TIMELINE)) else {
        bail!("the install's ENVIRONMENT.BNDL lacks the timelines {STANDARD_TIMELINE:X} and {FAIR_TIMELINE:X}");
    };
    Ok(vec![
        WeatherSpec { max_cover: FAIR_UP_TO, gain: 1.0, keys: keys(fair, 1.0)? },
        WeatherSpec { max_cover: CLOUDY_UP_TO, gain: 1.0, keys: keys(standard, 1.0)? },
        WeatherSpec { max_cover: 1.0, gain: OVERCAST_GAIN, keys: keys(standard, OVERCAST_DIM)? },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use chameleon_environment::{Clouds, Key};

    fn timeline(id: u32, plan: &[(f32, u32)]) -> Timeline {
        let clouds = |texture| Clouds { texture, lit: [1.0; 4], ambient: [0.5; 4], params: [0.0; 7] };
        Timeline {
            id,
            keys: plan.iter().map(|&(hour, texture)| Key { hour, keyframe: 1, clouds: clouds(texture) }).collect(),
        }
    }

    #[test]
    fn the_domes_are_named_once_and_their_keys_differ() {
        for (at, dome) in DOMES.iter().enumerate() {
            assert!(DOMES[at + 1..].iter().all(|other| other.key != dome.key && other.name != dome.name));
            assert!(dome.name.bytes().all(|byte| byte.is_ascii_lowercase()));
        }
        assert_eq!(dome_name(0x3F46), Some("noon"));
        assert_eq!(dome_name(0x1234), None);
    }

    #[test]
    fn three_weathers_are_built_and_the_overcast_one_is_dimmer_with_more_cover() {
        let day = [(0.0, 0x3F34), (5.0, 0x3F3C), (12.0, 0x3F46)];
        let timelines = [timeline(STANDARD_TIMELINE, &day), timeline(FAIR_TIMELINE, &[(0.0, 0x3F34), (12.0, 0x3F3A)])];
        let weathers = weathers(&timelines).unwrap();
        assert_eq!(weathers.len(), 3);
        assert_eq!(weathers[0].keys[1].dome, "fair");
        assert_eq!((weathers[1].gain, weathers[1].keys[1].dome), (1.0, "morning"));
        assert_eq!(weathers[2].gain, OVERCAST_GAIN);
        assert!(weathers[2].keys[0].tint[0] < weathers[1].keys[0].tint[0]);
        assert_eq!(weathers[1].keys[0].tint, [0.75; 3], "the mean of the lit (1) and ambient (0.5) colours");
    }

    #[test]
    fn an_unknown_dome_or_a_missing_timeline_is_an_error() {
        let odd = timeline(STANDARD_TIMELINE, &[(0.0, 0x9999)]);
        assert!(weathers(&[odd, timeline(FAIR_TIMELINE, &[(0.0, 0x3F34)])]).is_err());
        assert!(weathers(&[]).is_err());
    }

    #[test]
    fn keys_that_do_not_rise_are_dropped() {
        let doubled = timeline(1, &[(0.0, 0x3F34), (5.0, 0x3F3C), (5.0, 0x3F3C), (6.0, 0x3F3C)]);
        let hours: Vec<f32> = keys(&doubled, 1.0).unwrap().iter().map(|key| key.hour).collect();
        assert_eq!(hours, [0.0, 5.0, 6.0]);
    }
}
