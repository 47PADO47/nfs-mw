//! The Showcase (`Showcase.fng`, the original's `Showcase`): the picked car on the stage with nothing else on the screen. The
//! arrow keys turn the camera round it and tilt it, `9` and `0` zoom (the backdrop scene does that, `scene.rs`
//! `showcase_camera`); Accept or Back returns to the car select with the same car. Spec: `docs/specs/custom-race.md`
//! section 6.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK};

use super::car_list;
use super::car_select::progress;
use super::state;
use crate::frontend::career_screens::car_lot_look::{showcase_logos, showcased};
use crate::frontend::ids::screen;
use crate::frontend::logic::{Args, Command, Cx, Footer, ScreenLogic};
use crate::frontend::look::{CarLook, Look};
use crate::frontend::quick_play::fill::logo_textures;
use crate::frontend::quick_play::pool;

/// The manufacturer logo and the model-name picture, by name hash (the same objects as the car select's).
const MAKER_LOGO: u32 = 0x3E01_AD1D;
const MODEL_LOGO: u32 = 0xB05D_D708;
/// The group of the controller's hints (`Rotate`, `Zoom Out`, `Zoom In`), which the footer replaces with the keys.
const CONTROLLER_HINTS: u32 = 0xD86A_ACF8;

pub struct Showcase {
    /// The car shown, once the cars were read.
    car: Option<(String, Option<String>)>,
    /// The player whose car it is (split screen; 0 otherwise).
    player: u8,
    /// Opened by the car lot: the car is the lot's and the screen returns to the lot with these arguments.
    lot: Option<Args>,
    /// The lot car's look, with the record the lot knows it by.
    lot_look: Option<CarLook>,
}

impl Showcase {
    pub fn new(args: Args) -> Self {
        Self { car: None, player: args.player, lot: args.safehouse.then_some(args), lot_look: None }
    }

    /// The car the car select left chosen: its folder and preset, and its logos on the package.
    fn take_car(&mut self, cx: &mut Cx) {
        if self.lot.is_some() {
            return self.take_lot_car(cx);
        }
        let Some(pool) = pool::cars() else { return };
        let Some(name) = state::with(|s| s.pick(usize::from(self.player)).name.clone()) else { return };
        let progress = progress();
        let Some((list, at)) = car_list::find(&pool, &progress, &name) else { return };
        let Some(entry) = car_list::entries(list, &pool, &progress).get(at).cloned() else { return };
        let (maker, model) = logo_textures(cx.assets, &entry.car);
        for (hash, texture) in [(MAKER_LOGO, maker), (MODEL_LOGO, model)] {
            if let Some(o) = cx.object(hash) {
                cx.rt.set_texture(o, texture);
            }
        }
        self.car = Some((entry.car.folder, entry.preset));
    }
}

impl Showcase {
    /// The car the car lot sent: its look, and its logos on the package.
    fn take_lot_car(&mut self, cx: &mut Cx) {
        let Some(car) = showcased() else { return };
        if let Some((maker, model)) = showcase_logos(cx.assets, &car.name) {
            for (hash, texture) in [(MAKER_LOGO, maker), (MODEL_LOGO, model)] {
                if let Some(o) = cx.object(hash) {
                    cx.rt.set_texture(o, texture);
                }
            }
        }
        self.car = Some((car.look.car.clone(), car.look.preset.clone()));
        self.lot_look = Some(car.look);
    }
}

impl ScreenLogic for Showcase {
    fn start(&mut self, cx: &mut Cx) {
        cx.hide(CONTROLLER_HINTS, true);
        self.take_car(cx);
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        if self.car.is_none() {
            self.take_car(cx);
        }
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        if matches!(message, PAD_ACCEPT | PAD_BACK) {
            if let Some(args) = self.lot {
                return cx.send(Command::Switch(screen::CAR_SELECT.into(), args));
            }
            cx.send(Command::Switch(
                screen::CAR_SELECT.into(),
                Args { custom: true, player: self.player, ..Args::default() },
            ));
        }
    }

    fn footer(&self) -> Footer {
        Footer::Orbit
    }

    fn look(&self) -> Option<Look> {
        if let Some(look) = &self.lot_look {
            return Some(Look::LotShowcase(look.clone()));
        }
        let (car, preset) = self.car.as_ref()?;
        Some(Look::Showcase(CarLook::named(car, preset.as_deref())))
    }
}
