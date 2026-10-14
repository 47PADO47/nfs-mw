//! Sharing authored event output across independently clocked mixer instances.
use super::Mixer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventOutput {
    pub position: i32,
    pub level: i32,
}

impl Default for EventOutput {
    fn default() -> Self {
        Self { position: crate::shape::UNITY, level: 0 }
    }
}

impl Mixer {
    pub fn event_output(&self, state: u8, instance: u8, event: u8) -> Option<EventOutput> {
        let index = *self.event_index.get(&(state, instance, event))?;
        Some(EventOutput { position: self.graph.event_q[index], level: self.graph.event_db[index] })
    }

    /// Override an existing event's output and skip its local clock. `None`
    /// resumes local evaluation. Reject unknown events or invalid Q15 positions.
    pub fn override_event(&mut self, state: u8, instance: u8, event: u8, output: Option<EventOutput>) -> bool {
        let Some(&index) = self.event_index.get(&(state, instance, event)) else { return false };
        if let Some(output) = output {
            if !(0..=crate::shape::UNITY).contains(&output.position) {
                return false;
            }
            self.event_overrides.insert(index, output);
            return true;
        }
        self.event_overrides.remove(&index);
        true
    }
}
