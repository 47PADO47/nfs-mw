//! CPU checks: the metrics uniform, the table sizes the shaders assume, and the preset constants in the WGSL header.

use crate::area::{AREA_SIZE, area_texture_cached};
use crate::search::{SEARCH_SIZE, search_texture};
use crate::targets::rt_metrics;

#[test]
fn rt_metrics_hold_the_inverse_size_and_the_size() {
    let small: [f32; 4] = [1.0 / 160.0, 1.0 / 90.0, 160.0, 90.0];
    assert_eq!(rt_metrics(160, 90), small);
    let large: [f32; 4] = [1.0 / 1920.0, 1.0 / 1080.0, 1920.0, 1080.0];
    assert_eq!(rt_metrics(1920, 1080), large);
}

#[test]
fn texture_tables_have_the_sizes_the_shaders_assume() {
    assert_eq!(AREA_SIZE, (160, 560));
    assert_eq!(area_texture_cached().len(), 160 * 560 * 2, "RG8 area table");
    assert_eq!(SEARCH_SIZE, (64, 16));
    assert_eq!(search_texture().len(), 64 * 16, "R8 search table");
}

#[test]
fn the_shader_header_keeps_the_high_preset_and_the_table_sizes() {
    let common = include_str!("shaders/common.wgsl");
    for line in [
        "const SMAA_THRESHOLD: f32 = 0.1;",
        "const SMAA_MAX_SEARCH_STEPS: f32 = 16.0;",
        "const SMAA_MAX_SEARCH_STEPS_DIAG: f32 = 8.0;",
        "const SMAA_CORNER_ROUNDING_NORM: f32 = 0.25;",
        "vec2<f32>(1.0 / 160.0, 1.0 / 560.0)",
        "vec2<f32>(64.0, 16.0)",
    ] {
        assert!(common.contains(line), "common.wgsl lacks `{line}`");
    }
}
