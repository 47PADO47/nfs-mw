//! Task Manager's busiest-engine convention; never sum overlapping GPU engines.
// Reference: Microsoft, GPUs in the Task Manager (see docs/graphics-lab.md).
pub(super) fn cpu_percent(ticks: u64, seconds: f64) -> Option<f32> {
    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }
    Some((ticks as f64 / 100_000.0 / seconds).clamp(0.0, 100.0) as f32)
}
pub(super) fn peak<'a>(pid: u32, rows: impl Iterator<Item = (&'a str, f64)>) -> Option<(f32, &'static str)> {
    let prefix = format!("pid_{pid}_luid_");
    rows.filter(|(name, value)| name.starts_with(&prefix) && value.is_finite() && *value >= 0.0)
        .map(|(name, value)| (value.min(100.0) as f32, engine(name)))
        .max_by(|a, b| a.0.total_cmp(&b.0))
}
fn engine(name: &str) -> &'static str {
    let kind = name.split("_engtype_").nth(1).unwrap_or("");
    match kind.split('#').next().unwrap_or("") {
        "3D" => "3D",
        "Compute" | "Compute_0" | "Compute_1" => "Compute",
        "Copy" => "Copy",
        "OpticalFlow" => "Optical flow",
        "VideoDecode" => "Video decode",
        "VideoEncode" => "Video encode",
        _ => "Other",
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selects_only_this_pid_and_does_not_sum_parallel_engines() {
        let rows = [
            ("pid_12_luid_a_phys_0_eng_0_engtype_3D", 45.0),
            ("pid_12_luid_a_phys_0_eng_1_engtype_Compute", 92.0),
            ("pid_123_luid_a_phys_0_eng_0_engtype_3D", 100.0),
            ("pid_12_luid_a_phys_0_eng_2_engtype_Copy", f64::NAN),
        ];
        assert_eq!(peak(12, rows.into_iter()), Some((92.0, "Compute")));
        assert_eq!(peak(99, rows.into_iter()), None);
        assert_eq!(peak(12, [(rows[0].0, -1.0)].into_iter()), None);
        assert_eq!(peak(12, [(rows[0].0, 0.0)].into_iter()), Some((0.0, "3D")));
    }
    #[test]
    fn thread_time_is_normalized_to_wall_time_and_invalid_intervals_stay_missing() {
        assert_eq!(cpu_percent(5_000_000, 1.0), Some(50.0));
        assert_eq!(cpu_percent(5_000_000, 0.5), Some(100.0));
        assert_eq!(cpu_percent(0, 0.0), None);
    }
}
