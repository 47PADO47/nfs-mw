//! What the interpreter asks of its user: voices and other objects. Spec: `docs/specs/aems.md` §5 and §6.

/// The input types of a player.
pub mod input {
    pub const PITCH: usize = 0;
    pub const TIME: usize = 1;
    pub const VOLUME: usize = 2;
    pub const AZIMUTH: usize = 3;
    pub const REVERB: usize = 5;
    pub const LOW_PASS: usize = 6;
    pub const HIGH_PASS: usize = 7;
    pub const DRY: usize = 8;
    /// The number of input types.
    pub const COUNT: usize = 12;
}

/// One entry of a sample group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleEntry {
    /// 0 bank sound, 1 stream, 2 looped stream, 3 MIDI.
    pub kind: u8,
    pub priority: u8,
    /// The bank sound (`BNKl` entry, counted from 1) for kind 0.
    pub index: u32,
    /// The loop offset of the entry (streams).
    pub loop_offset: u32,
}

/// The values of a player's inputs. Types the player does not have are absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerInputs {
    values: [i32; input::COUNT],
    present: u16,
    changed: u16,
}

impl PlayerInputs {
    pub(crate) fn set(&mut self, kind: usize, value: i32, changed: bool) {
        if kind >= input::COUNT {
            return;
        }
        self.values[kind] = value;
        self.present |= 1 << kind;
        if changed {
            self.changed |= 1 << kind;
        }
    }

    /// The value of input `kind`, if the player has one.
    pub fn get(&self, kind: usize) -> Option<i32> {
        (kind < input::COUNT && self.present & (1 << kind) != 0).then(|| self.values[kind])
    }

    /// Whether input `kind` changed since the last time the voice was told.
    pub fn changed(&self, kind: usize) -> bool {
        kind < input::COUNT && self.changed & (1 << kind) != 0
    }

    /// The volume as a linear gain, 0 to 1 (the input runs 0 to 0x7FFF).
    pub fn gain(&self) -> f32 {
        self.get(input::VOLUME).map_or(1.0, |v| v.clamp(0, 0x7FFF) as f32 / 32767.0)
    }

    /// The pitch multiplier as a playback ratio (0x1000 = 1), 1 when the player has none.
    pub fn pitch_ratio(&self) -> f32 {
        self.get(input::PITCH).map_or(1.0, |v| v.clamp(0, 0xFFFF) as f32 / 4096.0)
    }
}

/// What the host says about a voice it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceState {
    pub playing: bool,
    /// Milliseconds played so far.
    pub elapsed_ms: i32,
    /// Milliseconds left; 0 for a sound that sustains (loops).
    pub remaining_ms: i32,
}

impl VoiceState {
    pub const GONE: VoiceState = VoiceState { playing: false, elapsed_ms: 0, remaining_ms: 0 };
}

/// A call of a class-controller node (it creates, feeds or releases another object).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassCall<'a> {
    /// Which class controller of the module.
    pub controller: usize,
    /// The object was just created with `params`.
    pub created: bool,
    /// The object was released.
    pub released: bool,
    pub params: &'a [i32],
}

/// The user of an [`Instance`](crate::Instance): voices and objects.
pub trait Host {
    /// Starts the voice of player `player` with `entry`; returns whether it started.
    fn play(&mut self, player: usize, entry: &SampleEntry, inputs: &PlayerInputs) -> bool;
    /// Stops the voice of `player`.
    fn stop(&mut self, player: usize);
    /// Silences the voice (volume 0 and pitch 0) without ending it.
    fn pause(&mut self, player: usize);
    /// Takes the voice up again after a pause with these inputs.
    fn resume(&mut self, player: usize, inputs: &PlayerInputs);
    /// Applies the inputs that changed to the playing voice and says how it is.
    fn update(&mut self, player: usize, inputs: &PlayerInputs) -> VoiceState;
    /// A class controller acted; returns the object's reference count (0 when it does not exist).
    fn class_call(&mut self, _call: &ClassCall<'_>) -> i32 {
        0
    }
}

/// A host that starts nothing: every voice fails to start.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullHost;

impl Host for NullHost {
    fn play(&mut self, _: usize, _: &SampleEntry, _: &PlayerInputs) -> bool {
        false
    }
    fn stop(&mut self, _: usize) {}
    fn pause(&mut self, _: usize) {}
    fn resume(&mut self, _: usize, _: &PlayerInputs) {}
    fn update(&mut self, _: usize, _: &PlayerInputs) -> VoiceState {
        VoiceState::GONE
    }
}
