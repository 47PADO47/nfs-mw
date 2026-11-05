//! What the pursuit trace reads of a cop's driving: its action and its speed limit.

use super::CopState;

impl CopState {
    /// The action of the last think: `close` (close pursuit), `turnaround` (the road action braking to turn round) or
    /// `road` (the road action).
    pub(in crate::scenes::world::ai) fn trace_action(&self) -> &'static str {
        if self.close.active() {
            return "close";
        }
        match self.turning {
            true => "turnaround",
            false => "road",
        }
    }

    /// The road action's speed governor limit, m/s.
    pub(in crate::scenes::world::ai) fn speed_limit(&self) -> f32 {
        self.governor.speed_limit
    }
}
