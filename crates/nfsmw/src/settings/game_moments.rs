//! The Game Moment Camera (`[game_moments]`): the original's gameplay switch for the short cinematic cuts of a chase, and
//! the rewrite's choice of which kinds play. Spec: `docs/specs/game-moment-camera.md`.

use serde::{Deserialize, Serialize};

/// One kind of game moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MomentKind {
    /// The cut of a pursuit breaker's collapse.
    Breakers,
    /// The look at the cop that starts a pursuit.
    Pursuit,
    /// The jump camera.
    Jumps,
    /// The camera of a crash through a roadblock.
    Crashes,
}

impl MomentKind {
    #[cfg(test)]
    pub const ALL: [Self; 4] = [Self::Breakers, Self::Pursuit, Self::Jumps, Self::Crashes];

    /// The options row's title key.
    pub fn title_key(self) -> &'static str {
        match self {
            Self::Breakers => "options-game-moment-breakers",
            Self::Pursuit => "options-game-moment-pursuit",
            Self::Jumps => "options-game-moment-jumps",
            Self::Crashes => "options-game-moment-crashes",
        }
    }
}

/// The switch and the kinds. The original has the switch alone (`Game Moment Camera`, on by default), which covers
/// every kind; the kinds are **[ours]** and all on by default, so the default plays what the original plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GameMoments {
    /// The original's Game Moment Camera row.
    pub camera: bool,
    pub breakers: bool,
    pub pursuit: bool,
    pub jumps: bool,
    pub crashes: bool,
}

impl Default for GameMoments {
    fn default() -> Self {
        Self { camera: true, breakers: true, pursuit: true, jumps: true, crashes: true }
    }
}

impl GameMoments {
    /// Whether moments of this kind play: the switch is on and so is the kind.
    pub fn allows(self, kind: MomentKind) -> bool {
        self.camera && self.kind(kind)
    }

    /// The kind's own switch, whatever the main one is.
    pub fn kind(self, kind: MomentKind) -> bool {
        match kind {
            MomentKind::Breakers => self.breakers,
            MomentKind::Pursuit => self.pursuit,
            MomentKind::Jumps => self.jumps,
            MomentKind::Crashes => self.crashes,
        }
    }

    pub fn kind_mut(&mut self, kind: MomentKind) -> &mut bool {
        match kind {
            MomentKind::Breakers => &mut self.breakers,
            MomentKind::Pursuit => &mut self.pursuit,
            MomentKind::Jumps => &mut self.jumps,
            MomentKind::Crashes => &mut self.crashes,
        }
    }

    pub(crate) fn read(value: &toml::Value, origin: &str) -> Option<Self> {
        match value.clone().try_into::<Self>() {
            Ok(value) => Some(value),
            Err(error) => {
                log::warn!("{origin}: invalid game moments: {error}");
                None
            }
        }
    }

    pub(crate) fn merge(self, old: Option<&toml::Value>) -> anyhow::Result<toml::Value> {
        let mut table = old.and_then(toml::Value::as_table).cloned().unwrap_or_default();
        let toml::Value::Table(update) = toml::Value::try_from(self)? else { unreachable!() };
        table.extend(update);
        Ok(toml::Value::Table(table))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_plays_every_kind_as_the_original_does() {
        let moments = GameMoments::default();
        assert!(MomentKind::ALL.into_iter().all(|kind| moments.allows(kind)));
    }

    #[test]
    fn the_switch_turns_every_kind_off_and_keeps_the_kinds() {
        let moments = GameMoments { camera: false, ..GameMoments::default() };
        assert!(MomentKind::ALL.into_iter().all(|kind| !moments.allows(kind) && moments.kind(kind)));
        let mut moments = GameMoments::default();
        *moments.kind_mut(MomentKind::Jumps) = false;
        assert!(!moments.allows(MomentKind::Jumps) && moments.allows(MomentKind::Pursuit));
    }

    #[test]
    fn a_partial_table_keeps_the_defaults_and_round_trips() {
        let value: toml::Value = toml::from_str("jumps = false").unwrap();
        let read = GameMoments::read(&value, "test").unwrap();
        assert_eq!(read, GameMoments { jumps: false, ..GameMoments::default() });
        let merged = read.merge(None).unwrap();
        assert_eq!(GameMoments::read(&merged, "test"), Some(read));
    }
}
