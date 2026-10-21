use super::*;

#[test]
fn health_destruction_preserves_ordinary_zone_tiers_and_retired_collision_stimuli() {
    let mut events = engine();
    events.hit(hit());
    events.tick(0.0);
    let levels = *events.source_levels();
    assert_eq!(levels[1], 2);
    assert_eq!(levels.iter().filter(|&&level| level != 0).count(), 1);
    events.note_health_destroyed();
    assert_eq!(events.broken_lights(), u32::MAX);
    assert_eq!(*events.source_levels(), levels, "ordinary health destruction is not explicit Destroy");
    events.hit(hit());
    assert_eq!(
        ids(&events.tick(0.0)),
        [100, 104, 105],
        "retired contacts still dispatch native collision/impact stimuli"
    );
    assert_eq!(*events.source_levels(), levels);
    events.repair_lights();
    assert_eq!(events.broken_lights(), 0, "logical lamp repair clears the health override");
    assert_eq!(*events.source_levels(), levels, "lamp-only repair is not full damage repair");
    events.destroy();
    assert_eq!(events.source_levels(), &[6; 10], "explicit breaker/flip Destroy still fills all zones");
}
