//! Process memory counters are cumulative/gauges, not GPU allocation or hard-fault measurements.
#![allow(unsafe_code, reason = "Windows fills owned process-memory and system-memory structures")]
use windows::Win32::System::{ProcessStatus::*, SystemInformation::*, Threading::*};

pub(super) fn sample() -> Vec<(&'static str, f64)> {
    let mut values = Vec::new();
    let mut process =
        PROCESS_MEMORY_COUNTERS_EX { cb: size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32, ..Default::default() };
    // SAFETY: synchronous API writes only within this correctly sized, live structure.
    if unsafe {
        GetProcessMemoryInfo(GetCurrentProcess(), (&mut process as *mut PROCESS_MEMORY_COUNTERS_EX).cast(), process.cb)
    }
    .is_ok()
    {
        values.extend([
            ("working_set_bytes", process.WorkingSetSize as f64),
            ("private_commit_bytes", process.PrivateUsage as f64),
            ("peak_working_set_bytes", process.PeakWorkingSetSize as f64),
            ("page_fault_count_total", f64::from(process.PageFaultCount)),
        ]);
    }
    let mut system = MEMORYSTATUSEX { dwLength: size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    if unsafe { GlobalMemoryStatusEx(&mut system) }.is_ok() {
        values.extend([
            ("system_available_physical_bytes", system.ullAvailPhys as f64),
            ("system_total_physical_bytes", system.ullTotalPhys as f64),
            ("process_available_commit_bytes", system.ullAvailPageFile as f64),
            ("process_commit_limit_bytes", system.ullTotalPageFile as f64),
        ]);
    }
    let mut io = IO_COUNTERS::default();
    if unsafe { GetProcessIoCounters(GetCurrentProcess(), &mut io) }.is_ok() {
        values.extend([
            ("io_read_bytes_total", io.ReadTransferCount as f64),
            ("io_write_bytes_total", io.WriteTransferCount as f64),
            ("io_read_operations_total", io.ReadOperationCount as f64),
        ]);
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_process_has_memory_and_missing_counters_are_not_zero_filled() {
        let values = sample();
        assert!(values.iter().any(|(key, value)| *key == "working_set_bytes" && *value > 0.0));
        assert!(values.iter().any(|(key, value)| *key == "private_commit_bytes" && *value > 0.0));
        assert!(values.iter().all(|(_, value)| value.is_finite() && *value >= 0.0));
    }
}
