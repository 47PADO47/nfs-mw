//! The host side of the runtime: setting values by object (data binding) and reading them back.

use glam::{Quat, Vec3};

use super::{ObjectRef, Runtime};
use crate::package::word;

impl Runtime {
    fn state_mut(&mut self, o: ObjectRef) -> Option<&mut super::ObjState> {
        self.running_mut(o.package)?.objects.get_mut(o.index)
    }

    /// Shows or hides an object (and its children) without touching its scripts.
    pub fn set_hidden(&mut self, o: ObjectRef, hidden: bool) {
        if let Some(s) = self.state_mut(o) {
            s.hidden = hidden;
        }
    }

    /// Replaces the text of a string object (instead of its stored text and the language table).
    pub fn set_text(&mut self, o: ObjectRef, text: impl Into<String>) {
        if let Some(s) = self.state_mut(o) {
            s.text = Some(text.into());
        }
    }

    /// Gives a string object another language label; clears a text set by the host.
    pub fn set_label(&mut self, o: ObjectRef, label: u32) {
        if let Some(s) = self.state_mut(o) {
            s.label = label;
            s.text = None;
        }
    }

    /// Sets the rotation of the object about the z axis, in radians (a needle).
    pub fn set_rotation_z(&mut self, o: ObjectRef, radians: f32) {
        if let Some(s) = self.state_mut(o) {
            s.data.set_rotation(Quat::from_rotation_z(radians));
        }
    }

    pub fn set_position(&mut self, o: ObjectRef, position: Vec3) {
        if let Some(s) = self.state_mut(o) {
            s.data.set_position(position);
        }
    }

    /// Sets the alpha (0..255) of the object.
    pub fn set_alpha(&mut self, o: ObjectRef, alpha: u8) {
        if let Some(s) = self.state_mut(o) {
            s.data.set_alpha(alpha as i32);
        }
    }

    /// Sets the colour (red, green, blue) of the object.
    pub fn set_colour(&mut self, o: ObjectRef, rgb: [u8; 3]) {
        if let Some(s) = self.state_mut(o) {
            s.data.set_i32(word::COLOUR, rgb[2] as i32);
            s.data.set_i32(word::COLOUR + 1, rgb[1] as i32);
            s.data.set_i32(word::COLOUR + 2, rgb[0] as i32);
        }
    }

    /// Swaps the texture of an image: `texture` is the key (hash of the upper-cased name) in the texture packs.
    pub fn set_texture(&mut self, o: ObjectRef, texture: u32) {
        if let Some(s) = self.state_mut(o) {
            s.texture = Some(texture);
        }
    }

    /// Switches an object to its script with this id, from time 0.
    pub fn run_script(&mut self, o: ObjectRef, script: u32) {
        self.set_script(o.package, o.index, script);
    }

    /// The id of the script an object is playing.
    pub fn script_of(&self, o: ObjectRef) -> Option<u32> {
        self.current_script_id(o.package, o.index)
    }
}
