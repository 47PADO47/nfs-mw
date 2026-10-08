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

    /// Sets the x and y of the position, keeping the depth.
    pub fn set_position_xy(&mut self, o: ObjectRef, x: f32, y: f32) {
        if let Some(s) = self.state_mut(o) {
            let z = s.data.position().z;
            s.data.set_position(Vec3::new(x, y, z));
        }
    }

    /// Sets the width and height of an image or the scale of a string, keeping the depth.
    pub fn set_size_xy(&mut self, o: ObjectRef, width: f32, height: f32) {
        if let Some(s) = self.state_mut(o) {
            s.data.set_f32(word::SIZE, width);
            s.data.set_f32(word::SIZE + 1, height);
        }
    }

    /// Sets the texture rectangle `[u0, v0, u1, v1]` of an image.
    pub fn set_uv(&mut self, o: ObjectRef, uv: [f32; 4]) {
        if let Some(s) = self.state_mut(o) {
            for (i, v) in uv.into_iter().enumerate() {
                s.data.set_f32(word::UV + i, v);
            }
        }
    }

    /// Sets the colour and alpha (red, green, blue, alpha) of the object.
    pub fn set_colour_rgba(&mut self, o: ObjectRef, rgba: [u8; 4]) {
        self.set_colour(o, [rgba[0], rgba[1], rgba[2]]);
        self.set_alpha(o, rgba[3]);
    }

    /// The position of the object relative to its parent.
    pub fn position(&self, o: ObjectRef) -> Option<Vec3> {
        Some(self.object(o)?.data.position())
    }

    /// The size of the object.
    pub fn size(&self, o: ObjectRef) -> Option<Vec3> {
        Some(self.object(o)?.data.size())
    }

    /// The first object whose name hashes from `name` (any case).
    pub fn find_name(&self, package: super::PackageId, name: &str) -> Option<ObjectRef> {
        self.find(package, crate::hash::fe_hash_upper(name))
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
