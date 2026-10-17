//! The `traffic_avoidance` setting: a traffic car held up by a car in its lane moves over to the next lane when the gap
//! is clear. A rewrite extension, off by default because the original does not do it
//! (`docs/specs/ai-traffic-obstacles.md` §10.1).

/// Off by default, so that traffic keeps the original's behaviour.
pub const DEFAULT_TRAFFIC_AVOIDANCE: bool = false;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Partial, Settings, env, file};

    #[test]
    fn traffic_avoidance_is_off_by_default_and_comes_from_every_layer() {
        assert!(!Settings::from(Partial::default()).traffic_avoidance);
        const { assert!(!DEFAULT_TRAFFIC_AVOIDANCE) };
        let cli = Partial { traffic_avoidance: Some(true), ..Partial::default() };
        let env = env::read(|n| (n == env::TRAFFIC_AVOIDANCE).then(|| "on".to_owned()));
        let file = file::parse("traffic_avoidance = false\n", "test");
        assert!(Settings::from(cli.or(env).or(file)).traffic_avoidance, "the command line wins");
        assert!(Settings::from(Partial::default().or(env).or(file)).traffic_avoidance, "the environment over the file");
        assert!(!Settings::from(Partial::default().or(file)).traffic_avoidance);
        assert_eq!(file::parse("traffic_avoidance = 3", "test").traffic_avoidance, None);
    }
}
