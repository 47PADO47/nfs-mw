//! The 32-bit ids that point at things in a map (layout: `docs/formats/mixmap.md`).

/// What an id points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    /// A control of the map (its curve output or its level).
    Control,
    /// A sub-mix or master channel.
    Channel,
    /// An input of a sound object (published by the game).
    Object,
    /// An input of a sound controller (published by the game).
    Controller,
    /// A 3D control.
    Spatial,
    /// An envelope event.
    Event,
    /// Kinds 6 and 7 do not exist.
    Invalid,
}

/// A source id as stored in the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(pub u32);

impl SourceId {
    pub fn kind(self) -> SourceKind {
        match self.0 >> 29 {
            0 => SourceKind::Control,
            1 => SourceKind::Channel,
            2 => SourceKind::Object,
            3 => SourceKind::Controller,
            4 => SourceKind::Spatial,
            5 => SourceKind::Event,
            _ => SourceKind::Invalid,
        }
    }

    /// For channel ids: sub-mix (true) or master (false).
    pub fn is_sub_channel(self) -> bool {
        self.0 & 0x1000_0000 != 0
    }

    /// Bits 24 to 27: the curve shape of a control's input, the envelope of an event, the camera-state count of
    /// a 3D control.
    pub fn nibble(self) -> u8 {
        ((self.0 >> 24) & 0xF) as u8
    }

    pub fn state(self) -> u8 {
        ((self.0 >> 16) & 0xFF) as u8
    }

    /// Bits 11 to 15; meaningless in the file (the loader overwrites it).
    pub fn instance(self) -> u8 {
        ((self.0 >> 11) & 0x1F) as u8
    }

    /// The sound object or controller number (kinds `Object` and `Controller`).
    pub fn object(self) -> u8 {
        ((self.0 >> 4) & 0x7F) as u8
    }

    /// The input index of an object or controller input.
    pub fn input_index(self) -> u8 {
        (self.0 & 0xF) as u8
    }

    /// The index of an element within its state (controls, channels, 3D controls, events).
    pub fn element(self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    /// The id with the instance bits replaced.
    pub fn with_instance(self, instance: u8) -> SourceId {
        SourceId((self.0 & 0xFFFF_07FF) | (u32::from(instance & 0x1F) << 11))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_of_a_controller_input() {
        // kind 3, curve nibble 4, state 0, object 0, input 0 (the first control of the main state).
        let id = SourceId(0x6400_0000);
        assert_eq!(id.kind(), SourceKind::Controller);
        assert_eq!(id.nibble(), 4);
        assert_eq!((id.state(), id.object(), id.input_index()), (0, 0, 0));
        let id = SourceId(0x4002_0010);
        assert_eq!(id.kind(), SourceKind::Object);
        assert_eq!((id.state(), id.object(), id.input_index()), (2, 1, 0));
    }

    #[test]
    fn channel_ids_tell_sub_from_master() {
        assert!(SourceId(0x3000_0003).is_sub_channel());
        assert!(!SourceId(0x2000_0003).is_sub_channel());
        assert_eq!(SourceId(0x3000_0003).kind(), SourceKind::Channel);
    }

    #[test]
    fn the_instance_bits_are_replaced() {
        let id = SourceId(0xA102_3012).with_instance(5);
        assert_eq!(id.instance(), 5);
        assert_eq!(id.state(), 2);
        assert_eq!(id.element(), 0x12);
        assert_eq!(id.kind(), SourceKind::Event);
    }
}
