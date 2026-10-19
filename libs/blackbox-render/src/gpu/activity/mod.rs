//! OS activity samples are collected off the render thread; no graphics queue is stalled for telemetry.
#[cfg(any(windows, test))]
mod sample;
#[cfg(windows)]
mod windows;

#[derive(Default)]
pub(super) struct Monitor {
    #[cfg(windows)]
    reader: Option<windows::Reader>,
}
impl Monitor {
    pub(super) fn new(windowed: bool) -> Self {
        #[cfg(windows)]
        return Self { reader: windowed.then(windows::Reader::new).flatten() };
        #[cfg(not(windows))]
        {
            let _ = windowed;
            Self::default()
        }
    }
    pub(super) fn sample(&self) -> blackbox_gfx::WorkloadActivity {
        #[cfg(windows)]
        return self.reader.as_ref().map_or_else(Default::default, windows::Reader::sample);
        #[cfg(not(windows))]
        blackbox_gfx::WorkloadActivity::default()
    }
}
