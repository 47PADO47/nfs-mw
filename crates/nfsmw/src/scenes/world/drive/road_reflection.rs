//! The displayed player pose supplies attached body, wheels and damage visibility.

use blackbox_gfx::Instance;

use super::Drive;
use crate::scenes::world::wet_reflections::vehicles;

impl Drive {
    pub(in crate::scenes::world) fn road_reflection_instances(&self, out: &mut Vec<Instance>) {
        vehicles::append(self.pose().transform(), out, |out| self.instances(out));
    }
}

#[cfg(test)]
mod tests;
