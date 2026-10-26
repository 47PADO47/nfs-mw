//! Writes the picked car and race into the objects of `Quick_Race_Brief.fng` (`docs/specs/quick-play.md` section 6).

use blackbox_feng::fe_hash_upper;
use blackbox_text::label_hash;
use nfsmw_career::carlot::stats;

use super::pool::{CarPool, PoolCar};
use super::text::{self, Unlock};
use crate::frontend::career_screens::car_lot::{Strip, set_bar};
use crate::frontend::logic::Cx;
use crate::frontend::quick_race::Track;
use crate::settings::SpeedUnits;
use crate::ui::Text;

/// The objects the screen writes, by name hash.
const MODE_TEXT_GROUP: u32 = 0xB515_4998;
pub(crate) const TRACK_NAME: u32 = 0xB515_4999;
pub(crate) const LENGTH: u32 = 0xB515_499A;
pub(crate) const LAPS: u32 = 0xB515_499B;
pub(crate) const OPPONENTS: u32 = 0xB515_499C;

/// The objects by GUID (`nfsmw dump-screen Quick_Race_Brief.fng`): the mode icon, the two logos, the group of the unlock line
/// and its string.
pub(crate) const MODE_ICON: u32 = 0x000F_5E9E;
pub(crate) const MANUFACTURER_LOGO: u32 = 0x000E_DDCD;
pub(crate) const MODEL_LOGO: u32 = 0x000E_DDDD;
#[cfg(test)]
pub(crate) const UNLOCK_GROUP: u32 = 0x000E_DCE4;
/// The unlock group's name hash, and its two scripts: the one that shows it (`0x1CA7C0`, which `Car_Select` runs for a locked
/// car) and the one that hides it (`0x16A259`, the package's own start state).
const UNLOCK_GROUP_NAME: u32 = 0xFE8F_DBF7;
const SCRIPT_SHOW: u32 = 0x001C_A7C0;
const SCRIPT_HIDE: u32 = 0x0016_A259;
const UNLOCK_TEXT: u32 = 0x000F_0F0D;

/// The right-hand column of options (Difficulty, Catch Up, Traffic with their values, and the `Track:` caption), which Quick
/// Play has no use for: the package shows it for a custom race.
pub(crate) const OPTION_COLUMN: [u32; 7] =
    [0x000E_DEA1, 0x000E_DEAD, 0x000E_DEA4, 0x000E_DEB3, 0x000E_972C, 0x000E_96F9, 0x000E_96DF];

/// The strips of the three bars: TOP SPEED group `0xF5C77`, ACCELERATION group `0xF5C7E`, HANDLING group `0xF5C85`. Their
/// geometry is that of `Car_Select.fng`'s bars, only the GUIDs differ.
pub(crate) const TOP_SPEED: [Strip; 2] = [
    Strip { guid: 0x000F_5C78, centre_x: -20.0, width: 80.0, uv_width: 2.5 },
    Strip { guid: 0x000F_5C79, centre_x: 20.0, width: 160.0, uv_width: 5.0 },
];
pub(crate) const ACCELERATION: [Strip; 2] = [
    Strip { guid: 0x000F_5C7F, centre_x: -40.0, width: 40.0, uv_width: 1.25 },
    Strip { guid: 0x000F_5C80, centre_x: 20.0, width: 160.0, uv_width: 5.0 },
];
pub(crate) const HANDLING: [Strip; 2] = [
    Strip { guid: 0x000F_5C86, centre_x: 0.0, width: 120.0, uv_width: 3.75 },
    Strip { guid: 0x000F_5C87, centre_x: 20.0, width: 160.0, uv_width: 5.0 },
];
/// The `LED_DELTA` ticks of the bars, which compare with another car; the brief compares with none.
pub(crate) const DELTA_TICKS: [u32; 3] = [0x000F_5C7A, 0x000F_5C81, 0x000F_5C88];

/// Hides what Quick Play never shows. Done once when the screen opens and again when it comes back.
pub fn hide_unused(cx: &mut Cx) {
    for guid in OPTION_COLUMN.into_iter().chain(DELTA_TICKS) {
        set_hidden(cx, guid, true);
    }
}

/// Writes the pick: the race (kind, name, length, laps, opponents), the car (logos, bars) and the unlock line.
pub fn show(cx: &mut Cx, car: &PoolCar, track: &Track, pool: &CarPool, stage: u8) {
    show_race(cx, track);
    show_car(cx, car, pool);
    show_unlock(cx, text::unlock_of(car, stage));
}

fn show_race(cx: &mut Cx, track: &Track) {
    let kind = Text::Game(text::mode_label(track.race_type)).resolve().unwrap_or_else(|| track.kind.to_owned());
    cx.text_group(MODE_TEXT_GROUP, &kind);
    set_texture(cx, MODE_ICON, text::mode_icon(track.race_type));
    let event = crate::career::event_id(&track.name);
    let name = Text::Game(label_hash(&text::track_label_key(&event))).resolve().unwrap_or(event);
    cx.text_object(TRACK_NAME, &name);
    let metric = cx.settings.speed_units == SpeedUnits::Metric;
    let unit = Text::Game(text::unit_label(metric)).resolve().unwrap_or_default();
    cx.text_object(LENGTH, &text::length_text(track.brief_length(), metric, &unit));
    cx.text_object(LAPS, &track.laps.to_string());
    cx.text_object(OPPONENTS, &track.opponents.to_string());
}

/// The textures of a car's manufacturer logo and model-name picture; the generic logos stand in for a missing one.
pub(crate) fn logo_textures(assets: &crate::ui::UiAssets, car: &PoolCar) -> (u32, u32) {
    let logo = |name: String, generic: &str| {
        let hash = fe_hash_upper(&name);
        match assets.texture(hash) {
            Some(_) => hash,
            None => fe_hash_upper(generic),
        }
    };
    (logo(text::manufacturer_texture(car), "GENERIC_LOGO_128"), logo(text::model_texture(car), "GENERIC_LOGO_256"))
}

fn show_car(cx: &mut Cx, car: &PoolCar, pool: &CarPool) {
    let (maker, model) = logo_textures(cx.assets, car);
    crate::frontend::car_title::show(cx, &car.folder);
    set_texture(cx, MANUFACTURER_LOGO, maker);
    set_texture(cx, MODEL_LOGO, model);
    let [top, handling, acceleration] = match car.stats {
        Some(own) => stats::bar_fractions(&own, &pool.population),
        None => [0.0; 3],
    };
    set_bar(cx, &TOP_SPEED, top);
    set_bar(cx, &ACCELERATION, acceleration);
    set_bar(cx, &HANDLING, handling);
}

/// The unlock line: shown with its sentence for a car the profile lacks, hidden otherwise (by the group's scripts, as the car
/// lot's locked notice is).
fn show_unlock(cx: &mut Cx, unlock: Unlock) {
    let sentence = match unlock {
        Unlock::None => None,
        Unlock::Label(label) => Text::Game(label).resolve(),
        Unlock::Rival(member) => rival_sentence(member),
    };
    let script = match sentence {
        Some(_) => SCRIPT_SHOW,
        None => SCRIPT_HIDE,
    };
    cx.script(UNLOCK_GROUP_NAME, script);
    let Some(sentence) = sentence else { return };
    let Some(object) = cx.guid(UNLOCK_TEXT) else { return };
    cx.rt.set_text(object, sentence);
}

/// `Unlock this car by beating <alias>, Blacklist member <n>.`; `None` when the language table lacks the alias or the sentence.
fn rival_sentence(member: u8) -> Option<String> {
    let template = Text::Game(text::RIVAL_LABEL).resolve()?;
    let alias = Text::Game(label_hash(&text::alias_key(member))).resolve()?;
    Some(text::fill_sentence(&template, &alias, member))
}

fn set_hidden(cx: &mut Cx, guid: u32, hidden: bool) {
    let Some(object) = cx.guid(guid) else { return };
    cx.rt.set_hidden(object, hidden);
}

fn set_texture(cx: &mut Cx, guid: u32, texture: u32) {
    let Some(object) = cx.guid(guid) else { return };
    cx.rt.set_texture(object, texture);
}
