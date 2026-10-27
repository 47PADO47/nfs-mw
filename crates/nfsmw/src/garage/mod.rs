//! Session-local garage selections, shared by the overlay and driving scenes.

mod appearance;
mod catalog_snapshot;
pub use catalog_snapshot::CatalogSnapshot;
mod color_parts;
mod controls;
mod exhaust;
pub use exhaust::ExhaustSelection;
mod backfire;
pub use backfire::BackfireEvent;
#[cfg(test)]
mod interaction_tests;
mod menu;
pub(crate) mod menu_rows;
#[cfg(test)]
mod menu_tests;
pub(crate) mod my_cars;
mod parts;
mod performance;
#[cfg(test)]
mod performance_tests;
pub(crate) mod presets;
pub(crate) mod pricing;
#[cfg(test)]
mod tests;
mod tuning;
mod view;

use bevy_ecs::resource::Resource;

use crate::input::{MouseCapture, UiFocus};

pub use menu::MenuEvent;
pub(crate) use menu::can_confirm;
pub use view::show;

/// Installed front-end platform displayed by a normal garage page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Cars,
    Customization,
    Backroom,
}

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
    /// Original fixed hero-car restriction, derived from the packed native CarType identifier.
    pub hero: bool,
    pub physics_name: String,
    pub performance: nfsmw_data::car::physics::PerformanceCatalog,
    pub performance_details: nfsmw_data::car::physics::performance::PerformanceDetails,
    /// Original frontend stock/max endpoints, normalized across the installed vehicle population.
    pub performance_ratings: Option<nfsmw_data::car::physics::ratings::RatingEndpoints>,
    pub parts: Option<CatalogSnapshot>,
    /// An installed vehicle that cannot be driven remains listed with a reason.
    pub unavailable: Option<String>,
}

impl Car {
    /// The name to ask the physics for: a mod car's folder, so its `[performance]` applies, else the car type.
    pub fn physics_key(&self) -> &str {
        match nfsmw_data::car::mods::is_mod_car(&self.folder) {
            true => &self.folder,
            false => &self.physics_name,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub car: String,
    /// None restores the installed paint.
    pub color: Option<[u8; 3]>,
    pub performance: nfsmw_data::car::physics::PerformancePackage,
    pub tuning: nfsmw_data::car::physics::tuning::TuningSelection,
    pub parts: nfsmw_data::car::customization::Selection,
    pub gauges: GaugeSelection,
    pub exhaust: ExhaustSelection,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GaugeSelection {
    pub skin: u8,
    pub backing: Option<[u8; 3]>,
    pub needle: Option<[u8; 3]>,
    pub digits: Option<[u8; 3]>,
    #[serde(default, with = "color_parts")]
    pub color_parts: [Option<u32>; 3],
}

impl GaugeSelection {
    pub fn colors(&self) -> [Option<[u8; 3]>; 3] {
        [self.backing, self.needle, self.digits]
    }

    pub fn validate(&self, catalog: &nfsmw_data::car::customization::Catalog) -> anyhow::Result<()> {
        anyhow::ensure!(self.skin == 0 || catalog.gauge_skins.contains(&self.skin), "Gauge skin is unavailable");
        for (index, hash) in self.color_parts.into_iter().enumerate() {
            catalog.validate_palette_color(3 + index, hash, self.colors()[index])?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum OptionsTab {
    #[default]
    Performance,
    Tuning,
    Appearance,
    Parts,
}

#[derive(Clone, Default)]
pub struct Model {
    pub open_requested: bool,
    /// Career shop limits the session to the selected owned vehicle.
    pub shop: bool,
    pub lock_car: bool,
    pub backroom: bool,
    /// Latest validated checkout quote; an absent quote blocks a shop purchase.
    pub shop_total: Option<u32>,
    pub shop_refund: Option<u32>,
    /// The validated cart lines, from installed prices and trade-in rules.
    pub shop_items: Vec<nfsmw_career::economy::shop::Item>,
    pub shop_cash: Option<u32>,
    pub shop_markers: Option<u32>,
    pub shop_marker_counts: [u32; 17],
    pub pricing: Option<pricing::Pricing>,
    pub shop_heat: Option<(f32, f32)>,
    pub gauge_max_rpm: f32,
    pub gauge_has_turbo: bool,
    /// Persisted reward entitlement used by the original Ford GT performance-detail variant.
    pub castrol_unlocked: bool,
    /// The highest unlocked ordinary package for each category in My Cars; Junkman remains unchanged.
    pub maximum_regular: Option<nfsmw_data::car::physics::PerformancePackage>,
    pub custom_slots: Vec<(String, String)>,
    pub custom_slot_cars: std::collections::HashMap<String, String>,
    pub custom_slot: Option<String>,
    pub custom_slots_enabled: bool,
    pub custom_slot_limit: usize,
    pub pending_delete: bool,
    pub shop_error: Option<String>,
    pub selection_locked: bool,
    /// Focus for device-independent menu navigation.
    pub menu_row: usize,
    pub(super) menu_scroll: bool,
    pub preview: bool,
    /// Cosmetic flame preview while the F6 world is frozen; never stored in a car preset.
    pub exhaust_preview: bool,
    pub backfire_preview: Option<BackfireEvent>,
    pub(super) exhaust_style_dirty: bool,
    /// Only a real garage session owns a config destination; standalone UI models remain side-effect free.
    pub(crate) exhaust_style_path: Option<std::path::PathBuf>,
    pub options_tab: OptionsTab,
    pub category: Category,
    pub cars: Vec<Car>,
    pub applied: Selection,
    pub draft: Selection,
    pub filter: String,
    pub parts_filter: [String; 9],
    pub message: Option<Result<String, String>>,
    pub preset_picker: crate::preset_store::Picker,
}

impl Model {
    pub fn selected(&self) -> Option<&Car> {
        self.cars.iter().find(|car| car.folder == self.draft.car)
    }

    pub fn select(&mut self, car: String) {
        let style = self.draft.exhaust.style();
        self.draft = Selection { car, ..Selection::default() };
        self.draft.exhaust.set_style(style);
        self.message = None;
    }

    pub fn reset_draft(&mut self) {
        self.pending_delete = false;
        self.menu_row = 0;
        self.menu_scroll = true;
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
    /// Normal garage presentation; F6 retains the developer overlay.
    pub menu: bool,
    pub menu_requested: bool,
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
