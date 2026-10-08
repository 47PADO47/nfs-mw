//! Volume groups and the amplitude to decibel conversion.

use kira::Decibels;

/// Which mixer group a sound plays in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(dead_code, reason = "the music wiring uses the music group")]
pub enum Group {
    Sfx,
    Music,
    Engine,
}

/// Linear volumes, 0 (silent) to 1 (full).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volumes {
    pub master: f32,
    pub music: f32,
    pub sfx: f32,
    pub engine: f32,
}

impl Default for Volumes {
    fn default() -> Self {
        Self { master: 0.8, music: 0.6, sfx: 0.9, engine: 0.9 }
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
}
