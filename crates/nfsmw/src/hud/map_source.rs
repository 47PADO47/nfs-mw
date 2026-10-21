//! Which minimap tiles the HUD wants: the career stage's map (free roam), a map variant picked by a pursuit race's
//! region, or the tiles of the running race's event. Rules: `docs/specs/race-minimap.md` section 1.

use nfsmw_career::regions::{MapVariant, region_index};

use super::minimap::{usable_variant, variant_for_stage, variant_stem};

/// What decides the minimap's map file. The scene sets it in [`super::HudState`]; the HUD plugin turns it into a file.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum MapSource {
    /// No race: the career stage's map (the full map outside career mode).
    #[default]
    Stage,
    /// A pursuit race: the map its `Region` names.
    Variant(MapVariant),
    /// A race that is not a pursuit race: the tiles baked for its event id (`MINI_MAP_<event id>`).
    Event(String),
}

impl MapSource {
    /// The source of a race with this event id. A pursuit race (`PursuitRace`) picks a variant by its `Region` text
    /// (`college` the first unlock map, `coastal` the second, anything else the full map); any other race uses its own
    /// event's tiles.
    pub fn of_race(event: &str, pursuit_race: bool, region: Option<&str>) -> Self {
        if !pursuit_race {
            return MapSource::Event(event.to_owned());
        }
        MapSource::Variant(region_variant(region))
    }

    /// The event whose tiles are wanted, if any.
    pub fn event(&self) -> Option<&str> {
        match self {
            MapSource::Event(event) => Some(event),
            _ => None,
        }
    }
}

/// The variant a `Region` text picks: the race region index 0 is the first unlock map, 1 the second, else the full map.
fn region_variant(region: Option<&str>) -> MapVariant {
    match region.and_then(region_index) {
        Some(0) => MapVariant::Unlock1,
        Some(1) => MapVariant::Unlock2,
        _ => MapVariant::Full,
    }
}

/// The file stem the minimap shows. `loaded` tells which variants have their tiles loaded; `event_stem` is the stem of
/// the wanted event's tiles once they are loaded (`None` while they load, or when the event has no file): then the
/// stage's map stays.
pub fn resolve_stem<'a>(
    source: &MapSource,
    stage: Option<u8>,
    loaded: impl Fn(MapVariant) -> bool,
    event_stem: Option<&'a str>,
) -> &'a str {
    let usable = |wanted: MapVariant| usable_variant(wanted, &loaded);
    if let (MapSource::Event(_), Some(stem)) = (source, event_stem) {
        return stem;
    }
    match source {
        MapSource::Variant(variant) => variant_stem(usable(*variant)),
        _ => variant_stem(usable(variant_for_stage(stage))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(_: MapVariant) -> bool {
        true
    }

    #[test]
    fn a_race_that_is_not_a_pursuit_uses_its_event() {
        assert_eq!(MapSource::of_race("10.2.1", false, Some("college")), MapSource::Event("10.2.1".into()));
        assert_eq!(MapSource::of_race("10.4.2.r", false, None).event(), Some("10.4.2.r"));
    }

    #[test]
    fn a_pursuit_race_picks_its_region_variant() {
        let of = |region| MapSource::of_race("19.8.1", true, region);
        assert_eq!(of(Some("college")), MapSource::Variant(MapVariant::Unlock1));
        assert_eq!(of(Some("coastal")), MapSource::Variant(MapVariant::Unlock2));
        assert_eq!(of(Some("city")), MapSource::Variant(MapVariant::Full));
        assert_eq!(of(None), MapSource::Variant(MapVariant::Full));
        assert_eq!(of(Some("nowhere")).event(), None);
    }

    #[test]
    fn without_a_race_the_stage_decides() {
        assert_eq!(resolve_stem(&MapSource::Stage, Some(14), all, None), "MINI_MAP_UNLOCK_1");
        assert_eq!(resolve_stem(&MapSource::Stage, Some(10), all, None), "MINI_MAP_UNLOCK_2");
        assert_eq!(resolve_stem(&MapSource::Stage, None, all, None), "MINI_MAP");
        assert_eq!(resolve_stem(&MapSource::Stage, Some(14), |v| v == MapVariant::Full, None), "MINI_MAP");
    }

    #[test]
    fn an_event_with_loaded_tiles_wins_over_the_stage() {
        let source = MapSource::Event("10.2.1".into());
        assert_eq!(resolve_stem(&source, Some(14), all, Some("MINI_MAP_10_2_1")), "MINI_MAP_10_2_1");
    }

    #[test]
    fn an_event_whose_tiles_are_not_ready_keeps_the_stage_map() {
        let source = MapSource::Event("16.8.1".into());
        assert_eq!(resolve_stem(&source, Some(14), all, None), "MINI_MAP_UNLOCK_1");
        assert_eq!(resolve_stem(&source, None, all, None), "MINI_MAP");
    }

    #[test]
    fn a_pursuit_race_ignores_the_stage_and_event_tiles() {
        let source = MapSource::Variant(MapVariant::Unlock2);
        assert_eq!(resolve_stem(&source, Some(1), all, Some("MINI_MAP_X")), "MINI_MAP_UNLOCK_2");
        assert_eq!(resolve_stem(&source, Some(1), |v| v == MapVariant::Full, None), "MINI_MAP");
    }
}
