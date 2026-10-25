//! The in-game HUD: the original HUD package (`HUD_SingleRace.fng`) run by `blackbox-feng`, driven by a plain
//! [`HudState`] and drawn by the shared presenter ([`crate::ui::present::blackbox`]).
//!
//! Game code only knows [`HudState`]: a scene returns one from `Scene::hud_state`. Nothing outside this module
//! names FEng or the presenter, so the presentation can change (docs/decisions/0002-ui-presentation.md).

mod bind;
#[cfg(test)]
mod bind_tests;
mod blip_style;
#[cfg(test)]
mod blip_style_tests;
mod blip_styles;
mod blips;
#[cfg(test)]
mod blips_tests;
mod busted_overlay;
mod cards;
mod cop_blip;
#[cfg(test)]
mod cop_blip_tests;
mod elements;
mod event_tiles;
mod frame_cache;
mod gauge_colors;
mod gauges;
mod hiding_mark;
#[cfg(test)]
mod install_tests;
mod interact_prompt;
#[cfg(test)]
mod interact_prompt_tests;
mod leaderboard;
#[cfg(test)]
mod leaderboard_tests;
mod leaders;
#[cfg(test)]
mod leaders_tests;
/// The Custom map's keys, applied to the settings.
mod map_keys;
/// The roads the Custom map draws, built once per network.
mod map_roads;
mod map_source;
mod message;
#[cfg(test)]
mod message_tests;
pub(crate) mod minimap;
#[cfg(test)]
mod minimap_tests;
#[cfg(test)]
mod mirror_install_tests;
/// The notification manager: the frame's HUD events as prioritised notices.
mod notify;
mod plugin;
pub(crate) mod police;
#[cfg(test)]
mod police_install_tests;
mod pursuit;
mod pursuit_meter;
pub use police::RadarHud;
mod pursuit_end;
#[cfg(test)]
mod pursuit_tests;
mod race;
mod race_card;
#[cfg(test)]
mod race_install_tests;
mod radar;
mod radio;
/// The FEng roots of each HUD element, for the Original theme's adapters (`ui/theme/original`).
pub(crate) mod roots;
pub(crate) mod skin;
mod speedbreaker;
pub(crate) mod split;
mod state;
mod trax;
/// The HUD layout's viewport math. The Custom radio card (`ui/theme/custom/radio_card`) reuses its `parameters`.
pub(crate) mod viewport;
#[cfg(test)]
mod viewport_tests;
mod withdrawn;

pub use blip_style::{MapBlip, TICK_SECONDS};
pub use blip_styles::{
    AUTO_SHOP, AUTO_SHOP_TINT, CAR_LOT, CAR_LOT_TINT, CHECKPOINT, COP, GPS, HELICOPTER, PURSUIT_BREAKER, RACE_CIRCUIT,
    RACE_DRAG, RACE_KNOCKOUT, RACE_SPEEDTRAP, RACE_SPRINT, RACE_TOLLBOOTH, RACER, ROADBLOCK, SAFE_HOUSE,
    SAFE_HOUSE_TINT,
};
pub use cards::{cards_routed, set_cards_routed};
pub use cop_blip::{colour as cop_colour, step_at as cop_step_at};
pub use interact_prompt::{InteractPopup, Prompt, show as show_interact_prompt};
pub use leaders::{
    Entry as LeaderEntry, LABEL_BUSTED, LABEL_KMH, LABEL_KNOCKED_OUT, LABEL_MPH, LeaderRow, Leaders, game_word,
};
#[cfg(test)]
pub use map_roads::MapRoad;
pub use map_roads::{MapRoads, MapRoadsHandle, RaceRoute, RaceRouteHandle, RoadClass};
pub use map_source::MapSource;
#[cfg(test)]
pub use message::current as current_mod_message;
pub use message::{
    Tone as ModMessageTone, advance as advance_mod_message, hide as hide_mod_message, is_up as mod_message_up,
    line as mod_message_line, set as set_mod_message, show as show_mod_message,
};
pub use notify::{Category as NoticeCategory, ChipKind, NoticeId};
pub use plugin::HudPlugin;
pub use pursuit::PursuitHud;
pub use pursuit_end::{PursuitEndCard, SHOW_SECS as PURSUIT_END_SECS, show as show_pursuit_end};
pub use race::{Count, Message, RaceHud};
pub use race_card::{RaceCard, ResultCard, show as show_race_card};
pub use radio::{Card, RadioHud};
pub use state::{
    ConditionWarning, EntryAction, EntryPrompt, HEAVY_DAMAGE, HelicopterIntel, HudCamera, HudContext, HudEvent,
    HudState, HudTelemetry, LastKnown, MapPosition, NavMarker, NavMarkerKind, NavRoute, PoliceIntel, PursuitPhase,
    RaceInfo, RaceIntro, RaceStage, RivalGap, TireCondition, TurnDirection, VehicleCondition, checkpoint_gap,
    cosmetic_level, entry_action, known_roadblocks, next_turn, rival_of, shows_rival_gap, sighted,
};
pub use state::{EntranceMark, MarkPoint, WorldMarks};
