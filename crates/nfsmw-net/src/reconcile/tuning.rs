//! The tuning values of sync-model.md section 9, with the document's start values as defaults.

/// Every tuning value of the sync model. The defaults are the start values of sync-model.md section 9 for a LAN
/// session; [`Tuning::online`] changes what differs online. They are to be set by the test plan, not argued here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Snapshots per second (20, or 30 as a LAN option). Section 3.2.
    pub snapshot_hz: u32,
    /// Interpolation delay in snapshot intervals. Section 3.2.
    pub interp_delay_intervals: f32,
    /// Most extra delay the jitter allowance adds, in snapshot intervals. Section 3.2.
    pub delay_allowance_intervals: f32,
    /// Extrapolation stops here and the entity is marked lagging, milliseconds. Section 3.4.
    pub extrapolation_limit_ms: f32,
    /// Extrapolated velocity starts to decay here, milliseconds. Section 3.4.
    pub extrapolation_decay_ms: f32,
    /// A predicted position closer than this to the server's is right, metres. Section 2.3.
    pub position_threshold_m: f32,
    /// A predicted orientation closer than this is right, degrees. Section 2.3.
    pub orientation_threshold_deg: f32,
    /// A predicted velocity closer than this is right, metres per second. Section 2.3.
    pub velocity_threshold_mps: f32,
    /// A correction longer than this is a snap, metres. Section 2.3.
    pub snap_distance_m: f32,
    /// Half-life of the drawn correction offset, milliseconds. Section 2.4.
    pub offset_half_life_ms: f32,
    /// Ticks of input repeated in each input packet. Section 2.2.
    pub input_redundancy: usize,
    /// The server's input buffer depth and the client's input lead margin, ticks. Section 2.5.
    pub input_buffer_ticks: u32,
    /// Ticks of missing input after which the server decays throttle. Section 2.5.
    pub missing_input_decay_ticks: u32,
    /// Remote cars nearer than this are drawn at the near-field delay, metres. Section 4.3.
    pub near_field_m: f32,
    /// The near-field display delay, milliseconds. Section 4.3.
    pub near_field_delay_ms: f32,
    /// The clock's rate factor stays within `1 ± this`. Section 5.2.
    pub rate_factor_limit: f64,
    /// A clock error above this many ticks steps the clock. Section 5.2.
    pub clock_step_ticks: f64,
    /// Ticks of prediction kept. Section 2.1.
    pub prediction_ring: usize,
}

impl Default for Tuning {
    fn default() -> Self {
        Self::lan()
    }
}

impl Tuning {
    /// The start values for a LAN session.
    pub const fn lan() -> Self {
        Self {
            snapshot_hz: 20,
            interp_delay_intervals: 2.0,
            delay_allowance_intervals: 1.0,
            extrapolation_limit_ms: 250.0,
            extrapolation_decay_ms: 100.0,
            position_threshold_m: 0.02,
            orientation_threshold_deg: 0.2,
            velocity_threshold_mps: 0.05,
            snap_distance_m: 2.0,
            offset_half_life_ms: 50.0,
            input_redundancy: 8,
            input_buffer_ticks: 1,
            missing_input_decay_ticks: 6,
            near_field_m: 15.0,
            near_field_delay_ms: 50.0,
            rate_factor_limit: 0.02,
            clock_step_ticks: 6.0,
            prediction_ring: 128,
        }
    }

    /// The start values online: a two-tick input buffer.
    pub const fn online() -> Self {
        Self { input_buffer_ticks: 2, ..Self::lan() }
    }

    /// Physics ticks between two snapshots (3 at 20 Hz), at least 1.
    pub fn snapshot_interval_ticks(&self) -> u32 {
        (nfsmw_sim::TICK_HZ / self.snapshot_hz.max(1)).max(1)
    }
}
