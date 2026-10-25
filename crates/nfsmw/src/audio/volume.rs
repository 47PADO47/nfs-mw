//! Volume groups and the amplitude to decibel conversion.

use kira::Decibels;

/// Which mixer group a sound plays in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Sfx,
    Music,
    Engine,
    /// The HUD's own cues (`hud/notify`): `[custom_hud] hud_volume`.
    Hud,
}

/// Linear volumes, 0 (silent) to 1 (full).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volumes {
    pub master: f32,
    pub music: f32,
    pub sfx: f32,
    pub engine: f32,
    pub hud: f32,
}

impl Default for Volumes {
    fn default() -> Self {
        Self { master: 0.8, music: 0.6, sfx: 0.9, engine: 0.9, hud: crate::settings::hud::DEFAULT_HUD_VOLUME }
    }
}

impl Volumes {
    /// The level of `group` alone, before the master volume.
    #[cfg(test)]
    pub fn level(&self, group: Group) -> f32 {
        match group {
            Group::Sfx => self.sfx,
            Group::Music => self.music,
            Group::Engine => self.engine,
            Group::Hud => self.hud,
        }
    }

    /// What a sound of `group` plays at: the group's level times the master volume, as the mixer's tracks nest.
    #[cfg(test)]
    pub fn effective(&self, group: Group) -> f32 {
        self.master * self.level(group)
    }
}

/// A linear amplitude as decibels (silence at and below 0.001, which is -60 dB).
pub fn decibels(amplitude: f32) -> Decibels {
    if amplitude.is_nan() || amplitude <= 0.001 {
        return Decibels::SILENCE;
    }
    Decibels(20.0 * amplitude.min(4.0).log10())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amplitude_to_decibels() {
        assert_eq!(decibels(1.0), Decibels(0.0));
        assert!((decibels(0.5).0 + 6.0206).abs() < 1e-3);
        assert_eq!(decibels(0.0), Decibels::SILENCE);
        assert_eq!(decibels(f32::NAN), Decibels::SILENCE);
        assert_eq!(decibels(100.0), decibels(4.0));
    }

    #[test]
    fn the_hud_group_is_its_level_times_the_master() {
        let volumes = Volumes { master: 0.5, hud: 0.8, ..Volumes::default() };
        assert_eq!(volumes.level(Group::Hud), 0.8);
        assert!((volumes.effective(Group::Hud) - 0.4).abs() < 1e-6);
        let silent = Volumes { master: 0.0, ..volumes };
        assert_eq!(silent.effective(Group::Hud), 0.0, "the master silences the HUD too");
        let muted = Volumes { hud: 0.0, ..volumes };
        assert_eq!(muted.effective(Group::Hud), 0.0);
        assert_eq!(muted.effective(Group::Sfx), volumes.effective(Group::Sfx), "the HUD level moves the HUD only");
    }
}
