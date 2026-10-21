//! Custom Race, step four: the car select (`Car_Select.fng` in its quick race mode, the original's `UIQRCarSelect`). Left and
//! right step through the cars of a list, up and down change the list, `3` opens the Showcase, Accept asks for the
//! transmission and starts the race, Back returns to the options. Spec:
//! `docs/specs/custom-race.md` sections 5 to 7.

use std::sync::Arc;

use blackbox_feng::ids::{BUTTON_PRESSED, PAD_ACCEPT, PAD_BACK, PAD_BUTTON0, PAD_DOWN, PAD_LEFT, PAD_RIGHT, PAD_UP};
use blackbox_text::label_hash;
use nfsmw_career::carlot::{self, stats};
use nfsmw_career::settings::Flags;
use nfsmw_career::stable::CarRecord;

use super::car_list::{self, Entry, List, Progress};
use super::state;
use crate::career::CareerHandle;
use crate::frontend::career_screens::car_lot::{ACCELERATION_STRIPS, HANDLING_STRIPS, TOP_SPEED_STRIPS, set_bar};
use crate::frontend::confirm::{Confirm, Look as DialogLook};
use crate::frontend::ids::{self, screen};
use crate::frontend::logic::{Args, Command, CustomStart, Cx, Footer, ScreenLogic};
use crate::frontend::look::{CarLook, Look};
use crate::frontend::quick_play::fill::logo_textures;
use crate::frontend::quick_play::pool::{self, CarPool, PROLOGUE_STAGE};
use crate::frontend::quick_play::text::{self, Unlock};
use crate::settings::Transmission;
use crate::ui::Text;

/// Posted to the package when it opens in the quick race: it shows the list header, the counter and the arrows.
const QUICK_RACE_MODE: u32 = 0xDE51_1657;
/// Posted to leave by Accept and by Back (the package's own leave animations, which end in `EXIT_COMPLETE`), and to show the
/// Showcase's transition.
const LEAVE_ACCEPT: u32 = 0x2E76_EDFB;
const LEAVE_BACK: u32 = 0x93E8_A57C;
const SHOWCASE_TRANSITION: u32 = 0x89D0_649C;
/// The Showcase button is the first face button (`PAD_BUTTON0`, key 3).
const SHOWCASE_BUTTON: u32 = PAD_BUTTON0;

/// The objects of the package, by name hash.
const LIST_LABEL: u32 = 0xAA98_34BC;
const LIST_ICON: u32 = 0xE3B2_71B8;
const MAKER_LOGO: u32 = 0x3E01_AD1D;
const MODEL_LOGO: u32 = 0xB05D_D708;
const NUMBER: u32 = 0x6F25_A248;
const NUMBER_OF: u32 = 0xB203_7BDC;
const LOCKED_GROUP: u32 = 0xD0F7_C7CC;
const LOCKED_TEXT: u32 = 0x2D25_B2C4;
/// The car name strings (the pictures name the car instead), the console Accept button, the heat meter and the lot's lines.
const HIDDEN: [u32; 8] =
    [0xD6D3_2016, 0x79D6_E45C, 0x64F6_D21F, 0xBDE6_E3BA, 0x1939_8802, 0x1930_B057, 0x20D1_13DC, 0x20C8_3C31];
const BLANKED: [u32; 3] = [0xD6D3_2016, 0x79D6_E45C, 0xC5E1_D8A4];
/// The "no cars" panel of an empty list: shown and hidden by these messages, its sentence is the string `0x36C1E04D` (the My
/// Cars list says how to fill it, the others say `0x58BBED2A`) **[decomp]**.
const NO_CARS_ON: u32 = 0xD942_0CD5;
const NO_CARS_OFF: u32 = 0x7C45_83DC;
const NO_CARS_TEXT: u32 = 0x36C1_E04D;
const NO_CARS: u32 = 0x58BB_ED2A;
const NO_MY_CARS: u32 = 0x0DA8_7B01;
const SCRIPT_SHOW: u32 = 0x001C_A7C0;
const SCRIPT_HIDE: u32 = 0x0016_A259;
/// `Select your transmission.`, Manual and Auto.
const TRANSMISSION_QUESTION: u32 = 0x6F54_01D1;
const MANUAL: u32 = 0x317D_3005;
const AUTO: u32 = 0x8CD5_32A0;

/// What a leave in progress goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Leave {
    Accept,
    Back,
    Showcase,
}

pub struct CarSelect {
    pool: Option<Arc<CarPool>>,
    list: List,
    entries: Vec<Entry>,
    at: usize,
    leaving: Option<Leave>,
}

impl Default for CarSelect {
    fn default() -> Self {
        Self { pool: None, list: List::Stock, entries: Vec::new(), at: 0, leaving: None }
    }
}

/// The Blacklist position, the career's win and the developer's switch, which open the cars.
pub(crate) fn progress() -> Progress {
    let (stage, flags, owned, customs) = CareerHandle::shared()
        .with_loaded(|career| {
            let names = |keep: &dyn Fn(&CarRecord) -> bool| -> Vec<String> {
                let cars = career.stable.cars.iter().filter(|car| keep(car));
                cars.filter_map(|car| {
                    carlot::by_frontend_key(&career.data.cars, car.frontend_key).map(|lot| lot.name.clone())
                })
                .collect()
            };
            let owned = names(&|car| !car.is_free() && car.is_owned());
            let customs = names(&CarRecord::is_quick_race);
            (career.stage(), career.settings.flags, owned, customs)
        })
        .unwrap_or((PROLOGUE_STAGE, Flags::EMPTY, Vec::new(), Vec::new()));
    // The profile's own flags (`HasBeatenCareer`, `HasBeatenSpecialChallengeEvent`, `HasBeatenChallengeSeries`,
    // `HasBeenAwardedCastrolGT`) count as much as the series wins this build keeps itself **[decomp]**.
    let mut series = crate::frontend::challenge_series::completion();
    series.special |= flags.contains(Flags::EVENT_19_8_69_DONE);
    series.all |= flags.contains(Flags::CHALLENGE_SERIES_DONE);
    Progress {
        stage,
        career_won: flags.contains(Flags::CAREER_COMPLETE),
        castrol_awarded: flags.contains(Flags::BONUS_CAR_0X34498EB2),
        open_all: super::tracks::unlock_all(),
        owned,
        customs,
        series,
    }
}

impl CarSelect {
    /// Takes the cars once they were read (in the background when the front end started), and shows the car chosen last (the
    /// career-start M3 GTR the first time, in the Preset list **[verified]**).
    fn take_pool(&mut self, cx: &mut Cx) {
        if self.pool.is_some() {
            return;
        }
        let Some(pool) = pool::cars() else { return };
        let progress = progress();
        let remembered = state::with(|s| s.car.name.clone());
        let found = remembered.as_deref().and_then(|name| car_list::find(&pool, &progress, name));
        let (list, at) = found.unwrap_or((List::Preset, 0));
        self.pool = Some(pool);
        self.open_list(cx, list, at);
    }

    fn open_list(&mut self, cx: &mut Cx, list: List, at: usize) {
        let Some(pool) = &self.pool else { return };
        self.list = list;
        self.entries = car_list::entries(list, pool, &progress());
        self.at = at.min(self.entries.len().saturating_sub(1));
        self.choose(cx);
    }

    /// The shown car is the race's car now (`SetSelectedCar`), and the screen draws it.
    fn choose(&mut self, cx: &mut Cx) {
        if let Some(entry) = self.entries.get(self.at) {
            state::with(|s| {
                s.car.list = self.list.index();
                s.car.name = Some(entry.key.clone());
            });
        }
        self.refresh(cx);
    }

    fn step_car(&mut self, cx: &mut Cx, by: isize) {
        if self.entries.len() < 2 {
            return;
        }
        self.at = (self.at as isize + by).rem_euclid(self.entries.len() as isize) as usize;
        self.choose(cx);
    }

    fn step_list(&mut self, cx: &mut Cx, forward: bool) {
        self.open_list(cx, self.list.step(forward), 0);
    }

    fn entry(&self) -> Option<&Entry> {
        self.entries.get(self.at)
    }

    /// Writes the list header, the counter, the logos, the bars and the unlock line.
    fn refresh(&mut self, cx: &mut Cx) {
        for hidden in HIDDEN {
            cx.hide(hidden, true);
        }
        cx.label_group(LIST_LABEL, self.list.label());
        if let Some(o) = cx.object(LIST_ICON) {
            cx.rt.set_texture(o, self.list.icon());
        }
        let number = if self.entries.is_empty() { 0 } else { self.at + 1 };
        cx.text_group(NUMBER, &number.to_string());
        cx.text_group(NUMBER_OF, &self.entries.len().to_string());
        // Strings the package's own scripts show again: the car's name (the pictures name it) and the console's Showcase hint.
        for name in BLANKED {
            cx.text_object(name, "");
        }
        let (Some(pool), Some(entry)) = (self.pool.clone(), self.entry().cloned()) else {
            return self.show_empty(cx);
        };
        cx.rt.post_to_package(cx.package, NO_CARS_OFF);
        let (maker, model) = logo_textures(cx.assets, &entry.car);
        for (name, texture) in [(MAKER_LOGO, maker), (MODEL_LOGO, model)] {
            if let Some(o) = cx.object(name) {
                cx.rt.set_hidden(o, false);
                cx.rt.set_texture(o, texture);
            }
        }
        let [top, handling, acceleration] = match entry.car.stats {
            Some(own) => stats::bar_fractions(&own, &pool.population),
            None => [0.0; 3],
        };
        set_bar(cx, &TOP_SPEED_STRIPS, top);
        set_bar(cx, &ACCELERATION_STRIPS, acceleration);
        set_bar(cx, &HANDLING_STRIPS, handling);
        self.show_unlock(cx, &entry);
    }

    /// A list with no car: the package's "no cars" panel with the list's own sentence, no logos and empty bars.
    fn show_empty(&self, cx: &mut Cx) {
        cx.rt.post_to_package(cx.package, NO_CARS_ON);
        let label = if self.list == List::MyCars { NO_MY_CARS } else { NO_CARS };
        if let Some(o) = cx.object(NO_CARS_TEXT) {
            cx.label(o, label);
        }
        for name in [MAKER_LOGO, MODEL_LOGO] {
            cx.hide(name, true);
        }
        for strips in [&TOP_SPEED_STRIPS, &ACCELERATION_STRIPS, &HANDLING_STRIPS] {
            set_bar(cx, strips, 0.0);
        }
        cx.script(LOCKED_GROUP, SCRIPT_HIDE);
    }

    /// The locked notice: shown with its sentence for a locked car, hidden otherwise (the group's own scripts).
    fn show_unlock(&self, cx: &mut Cx, entry: &Entry) {
        let sentence = match (entry.locked, entry.unlock) {
            (false, _) | (true, Unlock::None) => None,
            (true, Unlock::Label(label)) => Text::Game(label).resolve(cx.assets),
            (true, Unlock::Rival(member)) => {
                let template = Text::Game(text::RIVAL_LABEL).resolve(cx.assets);
                let alias = Text::Game(label_hash(&text::alias_key(member))).resolve(cx.assets);
                template.zip(alias).map(|(template, alias)| text::fill_sentence(&template, &alias, member))
            }
        };
        let script = if entry.locked { SCRIPT_SHOW } else { SCRIPT_HIDE };
        cx.script(LOCKED_GROUP, script);
        cx.text_object(LOCKED_TEXT, &sentence.unwrap_or_default());
    }

    fn leave(&mut self, cx: &mut Cx, way: Leave) {
        let message = match way {
            Leave::Accept => LEAVE_ACCEPT,
            Leave::Back => LEAVE_BACK,
            Leave::Showcase => SHOWCASE_TRANSITION,
        };
        self.leaving = Some(way);
        cx.rt.post_to_package(cx.package, message);
    }

    /// Accept on an open car; a locked one is refused.
    fn accept(&mut self, cx: &mut Cx) {
        if self.entry().is_some_and(|entry| !entry.locked) {
            self.leave(cx, Leave::Accept);
        }
    }

    fn showcase(&mut self, cx: &mut Cx) {
        if self.entry().is_some_and(|entry| !entry.locked) {
            self.leave(cx, Leave::Showcase);
        }
    }

    /// The package has left: go where the answer was.
    fn finish(&mut self, cx: &mut Cx) {
        match self.leaving.take() {
            Some(Leave::Back) => cx.send(Command::Switch(screen::TRACK_OPTIONS.into(), Args::default())),
            Some(Leave::Showcase) => {
                cx.send(Command::Switch(screen::SHOWCASE.into(), Args { custom: true, ..Args::default() }))
            }
            Some(Leave::Accept) => self.start(cx),
            None => {}
        }
    }

    /// The transmission question, for every kind of race (the decompiled car select skips it for a drag race; the request says
    /// the question comes after the car, so a drag race asks too **[ours]**).
    fn start(&mut self, cx: &mut Cx) {
        let Some(entry) = self.entry().cloned() else { return };
        let Some((mode, race)) = state::with(|s| s.event().map(|race| (s.mode, race.to_owned()))) else { return };
        let options = state::with(|s| s.settings().race_options(mode));
        let go = |transmission| {
            Command::StartCustomRace(Box::new(CustomStart {
                race: race.clone(),
                car: entry.car.folder.clone(),
                preset: entry.preset.clone(),
                options,
                transmission,
            }))
        };
        let manual = cx.settings.transmission == Transmission::Manual;
        let question = Confirm::choice(
            Text::Game(TRANSMISSION_QUESTION),
            (Text::Game(MANUAL), go(Transmission::Manual)),
            (Text::Game(AUTO), go(Transmission::Automatic)),
        )
        .looking(DialogLook::Selection)
        .focusing_first(manual);
        cx.send(Command::Confirm(question));
    }
}

impl ScreenLogic for CarSelect {
    fn start(&mut self, cx: &mut Cx) {
        cx.rt.post_to_package(cx.package, QUICK_RACE_MODE);
        self.take_pool(cx);
        self.refresh(cx);
    }

    fn resume(&mut self, cx: &mut Cx) {
        // The question was answered No: the screen is as it was, and its package takes input again.
        self.leaving = None;
        cx.rt.set_input_enabled(cx.package, true);
        self.refresh(cx);
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        // The cars are read in the background, so the first frames may have none.
        if self.pool.is_none() {
            self.take_pool(cx);
        }
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        if self.leaving.is_some() {
            if message == ids::EXIT_COMPLETE {
                self.finish(cx);
            }
            return;
        }
        match message {
            PAD_LEFT => self.step_car(cx, -1),
            PAD_RIGHT => self.step_car(cx, 1),
            PAD_UP => self.step_list(cx, false),
            PAD_DOWN => self.step_list(cx, true),
            PAD_ACCEPT | BUTTON_PRESSED => self.accept(cx),
            PAD_BACK => self.leave(cx, Leave::Back),
            SHOWCASE_BUTTON => self.showcase(cx),
            _ => {}
        }
    }

    fn footer(&self) -> Footer {
        match self.entry() {
            Some(entry) if !entry.locked => Footer::CarSelect,
            _ => Footer::CarSelectLocked,
        }
    }

    fn look(&self) -> Option<Look> {
        let entry = self.entry()?;
        Some(Look::Car(CarLook::named(&entry.car.folder, entry.preset.as_deref())))
    }
}
