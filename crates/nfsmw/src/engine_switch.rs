//! The player's engine switch (`bb.engine`, `docs/modding/api-engine.md`): a mod switches the engine off, and starts it with
//! a sound and a delay. The state is the scene's; the drive reads it to take the throttle away, the HUD to show no revs
//! and the sound to keep the engine silent and play the start sound.
//!
//! An engine that no mod has stopped is running and nothing here changes: the switch is idle until a mod calls it.

use blackbox_modding::host::{EnginePhase, EngineState};

/// What the scene tells the mods about the player's engine in a frame: the engine and the pedals, and the car's id.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineReadout {
    pub state: EngineState,
    /// The id of the player's car (`POR911CARR12`).
    pub car: String,
}

/// The sound a start plays: a WAV in the mods' `files/` layers and its gain.
#[derive(Debug, Clone, PartialEq)]
pub struct StartSound {
    pub file: String,
    pub gain: f32,
}

/// What a mod asked of the switch.
#[derive(Debug, Clone, PartialEq)]
pub enum EngineCommand {
    /// Switch the engine off.
    Stop,
    /// Start it: play `sound` now, and run after `delay` seconds.
    Start { sound: Option<StartSound>, delay: f32 },
}

/// What the car's sound needs of the switch in one frame.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EngineAudio {
    /// The engine is not running: it makes no sound.
    pub stopped: bool,
    /// The start sound to play now, once.
    pub start: Option<StartSound>,
}

/// Where the engine is, and the start sound that has not been played yet.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EngineSwitch {
    phase: EnginePhase,
    /// Seconds until a starting engine runs.
    left: f32,
    sound: Option<StartSound>,
}

impl EngineSwitch {
    pub fn phase(&self) -> EnginePhase {
        self.phase
    }

    /// Whether the engine drives the car.
    pub fn is_running(&self) -> bool {
        self.phase == EnginePhase::Running
    }

    /// Make a mod's request. A start of an engine that already runs or starts does nothing.
    pub fn apply(&mut self, command: EngineCommand) {
        match command {
            EngineCommand::Stop => {
                self.phase = EnginePhase::Off;
                self.left = 0.0;
                self.sound = None;
            }
            EngineCommand::Start { sound, delay } => {
                if self.phase != EnginePhase::Off {
                    return;
                }
                self.sound = sound;
                if delay <= 0.0 {
                    self.phase = EnginePhase::Running;
                    return;
                }
                self.phase = EnginePhase::Starting;
                self.left = delay;
            }
        }
    }

    /// Advance `dt` seconds: a starting engine runs when its delay is over.
    pub fn step(&mut self, dt: f32) {
        if self.phase != EnginePhase::Starting {
            return;
        }
        self.left -= dt.max(0.0);
        if self.left <= 0.0 {
            self.phase = EnginePhase::Running;
        }
    }

    /// The start sound, once: the sound plays it and it is gone.
    fn take_sound(&mut self) -> Option<StartSound> {
        self.sound.take()
    }

    /// The frame's audio feed: whether the engine is silent, and the start sound once.
    pub fn audio(&mut self) -> EngineAudio {
        EngineAudio { stopped: !self.is_running(), start: self.take_sound() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sound() -> Option<StartSound> {
        Some(StartSound { file: "SOUND/MODENGINE/CAR/IGNITION.WAV".to_owned(), gain: 0.8 })
    }

    #[test]
    fn an_untouched_switch_is_a_running_engine() {
        let mut switch = EngineSwitch::default();
        assert!(switch.is_running());
        switch.step(1.0);
        assert!(switch.is_running());
        assert_eq!(switch.take_sound(), None);
    }

    #[test]
    fn stop_switches_off_and_drops_a_waiting_sound() {
        let mut switch = EngineSwitch::default();
        switch.apply(EngineCommand::Stop);
        assert_eq!(switch.phase(), EnginePhase::Off);
        assert!(!switch.is_running());
        switch.apply(EngineCommand::Start { sound: sound(), delay: 1.0 });
        switch.apply(EngineCommand::Stop);
        assert_eq!(switch.take_sound(), None, "a stop cancels a start that has not played");
        switch.step(5.0);
        assert_eq!(switch.phase(), EnginePhase::Off, "an engine that is off stays off with time");
    }

    #[test]
    fn a_start_with_a_delay_runs_when_the_delay_is_over_and_plays_its_sound_once() {
        let mut switch = EngineSwitch::default();
        switch.apply(EngineCommand::Stop);
        switch.apply(EngineCommand::Start { sound: sound(), delay: 0.5 });
        assert_eq!(switch.phase(), EnginePhase::Starting);
        assert!(!switch.is_running());
        assert_eq!(switch.take_sound(), sound());
        assert_eq!(switch.take_sound(), None, "once");
        switch.step(0.25);
        assert_eq!(switch.phase(), EnginePhase::Starting);
        switch.step(0.3);
        assert!(switch.is_running());
    }

    #[test]
    fn a_start_with_no_delay_runs_at_once() {
        let mut switch = EngineSwitch::default();
        switch.apply(EngineCommand::Stop);
        switch.apply(EngineCommand::Start { sound: None, delay: 0.0 });
        assert!(switch.is_running());
    }

    #[test]
    fn a_start_of_a_running_or_starting_engine_changes_nothing() {
        let mut switch = EngineSwitch::default();
        switch.apply(EngineCommand::Start { sound: sound(), delay: 1.0 });
        assert!(switch.is_running());
        assert_eq!(switch.take_sound(), None, "a running engine plays no start sound");
        switch.apply(EngineCommand::Stop);
        switch.apply(EngineCommand::Start { sound: sound(), delay: 1.0 });
        switch.apply(EngineCommand::Start { sound: None, delay: 0.0 });
        assert_eq!(switch.phase(), EnginePhase::Starting);
        assert_eq!(switch.take_sound(), sound(), "the second start did not replace the first");
    }
}
