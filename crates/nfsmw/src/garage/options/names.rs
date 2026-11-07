//! The words the options use for slots and levels.

use nfsmw_data::car::physics::PerformanceCategory;

/// A slot id as a label: `brake_discs` is `Brake discs`.
pub(super) fn humanize(id: &str) -> String {
    let words = id.trim_start_matches("perf_").replace(['_', '-'], " ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The console word of a performance category.
pub(super) fn level_key(category: PerformanceCategory) -> &'static str {
    match category {
        PerformanceCategory::Nitrous => "nitrous",
        other => other.field(),
    }
}

/// The performance categories in the order the garage lists them.
pub(super) const LEVEL_ORDER: [PerformanceCategory; 7] = [
    PerformanceCategory::Engine,
    PerformanceCategory::Transmission,
    PerformanceCategory::Induction,
    PerformanceCategory::Nitrous,
    PerformanceCategory::Chassis,
    PerformanceCategory::Tires,
    PerformanceCategory::Brakes,
];
