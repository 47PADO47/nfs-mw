//! Session-local garage selections, shared by the overlay and driving scenes.

mod controls;
#[cfg(test)]
mod interaction_tests;
mod parts;
mod performance;
#[cfg(test)]
mod performance_tests;
mod presets;
#[cfg(test)]
mod tests;
mod tuning;
mod view;

use bevy_ecs::resource::Resource;

use crate::input::{MouseCapture, UiFocus};

pub use view::show;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Category {
    #[default]
    Racing,
    Police,
}

impl Category {
    /// CarTypeInfos usage values documented in docs/formats/cardata.md.
    pub fn from_usage(usage: i32) -> Option<Self> {
        match usage {
            0 => Some(Self::Racing),
            1 => Some(Self::Police),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Racing => "Racing cars",
            Self::Police => "Police",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Car {
    pub folder: String,
    pub name: String,
    pub manufacturer: String,
    pub category: Category,
    pub original_color: Option<[u8; 3]>,
    pub paintable: bool,
    pub physics_name: String,
    pub performance: nfsmw_data::car::physics::PerformanceCatalog,
    pub parts: Option<nfsmw_data::car::customization::Catalog>,
    /// An installed vehicle that cannot be driven remains listed with a reason.
    pub unavailable: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub car: String,
    /// None restores the installed paint.
    pub color: Option<[u8; 3]>,
    pub performance: nfsmw_data::car::physics::PerformancePackage,
    pub tuning: nfsmw_data::car::physics::tuning::TuningSelection,
    pub parts: nfsmw_data::car::customization::Selection,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum OptionsTab {
    #[default]
    Performance,
    Tuning,
    Appearance,
    Parts,
}

#[derive(Default)]
pub struct Model {
    pub preview: bool,
    pub options_tab: OptionsTab,
    pub category: Category,
    pub cars: Vec<Car>,
    pub applied: Selection,
    pub draft: Selection,
    pub filter: String,
    pub parts_filter: [String; 5],
    pub message: Option<Result<String, String>>,
    pub preset_picker: crate::preset_store::Picker,
}

impl Model {
    pub fn selected(&self) -> Option<&Car> {
        self.cars.iter().find(|car| car.folder == self.draft.car)
    }

    pub fn select(&mut self, car: String) {
        self.draft = Selection { car, ..Selection::default() };
        self.message = None;
    }

    pub fn reset_draft(&mut self) {
        self.draft = self.applied.clone();
        if self.draft.car.is_empty()
            && let Some(car) = self.cars.iter().find(|car| car.category == self.category && car.unavailable.is_none())
        {
            self.draft.car = car.folder.clone();
        }
        self.message = None;
        if let Some(car) = self.selected() {
            self.category = car.category;
        }
    }
}

/// Input ownership belongs to the app, not a particular world scene.
#[derive(Resource, Default)]
pub struct Overlay {
    pub open: bool,
    /// A startup request can wait while the front end creates its driving scene.
    pub start_requested: bool,
    pub start_tab: OptionsTab,
    /// Also blocks the opening/closing frame from reaching the front end.
    pub had_input: bool,
    restore_capture: bool,
}

impl Overlay {
    pub fn set_open(&mut self, open: bool, capture: &mut MouseCapture, focus: &mut UiFocus) {
        if open == self.open {
            return;
        }
        self.had_input = true;
        self.open = open;
        focus.0 = open;
        if open {
            self.restore_capture = capture.0;
            capture.0 = false;
            return;
        }
        capture.0 = self.restore_capture;
    }
}

pub fn start(app: &mut bevy_app::App, tab: OptionsTab) {
    let world = app.world_mut();
    world.resource_mut::<Overlay>().start_tab = tab;
    let mut host = world.non_send_mut::<crate::app::Host>();
    let Some(model) = host.scene.garage() else {
        world.resource_mut::<Overlay>().start_requested = true;
        return;
    };
    model.reset_draft();
    model.options_tab = tab;
    model.preview = tab == OptionsTab::Parts;
    world.resource_mut::<Overlay>().open = true;
    world.resource_mut::<UiFocus>().0 = true;
    world.resource_mut::<MouseCapture>().0 = false;
}
