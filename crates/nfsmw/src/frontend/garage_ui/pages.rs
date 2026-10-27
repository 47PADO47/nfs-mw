//! The original garage's package routes, retaining each specialized screen.

use crate::garage::Model;
use crate::garage::menu_rows::Row;
use nfsmw_data::car::customization::Category as Part;

mod cameras;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Parts,
    Performance,
    Visual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Manager,
    AddCar,
    Main,
    Categories {
        section: Section,
        backroom: bool,
    },
    Choices {
        row: Row,
        backroom: bool,
    },
    RimBrands {
        backroom: bool,
    },
    Rims {
        brand: u32,
        radius: u32,
        backroom: bool,
    },
    Spoilers {
        group: u8,
        backroom: bool,
    },
    VinylGroups {
        backroom: bool,
    },
    Vinyls {
        group: u8,
        backroom: bool,
    },
    Paint {
        color: usize,
        backroom: bool,
    },
    Numbers,
    /// The text of a mod car's licence plate (`plate.rs`).
    Plate,
    /// The design behind the glyphs of a mod car's licence plate (`plate_design.rs`).
    PlateDesign,
    DecalZones {
        backroom: bool,
    },
    DecalSlots {
        zone: usize,
        backroom: bool,
    },
    GaugeColors {
        backroom: bool,
    },
    Tuning,
    Cart {
        backroom: bool,
        quick: bool,
    },
    Checkout {
        backroom: bool,
        quick: bool,
    },
    MaxPerformance,
    TuningDiscard,
    TuningError,
    InsufficientFunds,
    Delete,
    Leave,
    Showcase,
}

impl Page {
    pub fn root(model: &Model) -> Self {
        match model.custom_slots_enabled {
            true => Self::Manager,
            false => Self::Main,
        }
    }

    pub fn package(self) -> &'static str {
        match self {
            Self::Manager => "MyCarsManager.fng",
            Self::AddCar => "Car_Select.fng",
            Self::Main => "CustomizeMain.fng",
            Self::RimBrands { backroom: false } => "CustomizeGenericTop.fng",
            Self::RimBrands { backroom: true } => "CustomizeGenericTop_BACKROOM.fng",
            Self::Rims { backroom: false, .. } => "Rims.fng",
            Self::Rims { backroom: true, .. } => "Rims_BACKROOM.fng",
            Self::Spoilers { backroom: false, .. } => "Spoilers.fng",
            Self::Spoilers { backroom: true, .. } => "Spoilers_BACKROOM.fng",
            Self::Vinyls { backroom: false, .. } => "CustomizeParts.fng",
            Self::Vinyls { backroom: true, .. } => "CustomizeParts_BACKROOM.fng",
            Self::Categories { backroom: false, .. } => "CustomizeCategory.fng",
            Self::Categories { backroom: true, .. } => "CustomizeCategory_BACKROOM.fng",
            Self::DecalZones { backroom: false }
            | Self::DecalSlots { backroom: false, .. }
            | Self::VinylGroups { backroom: false } => "CustomizeGenericTop.fng",
            Self::DecalZones { backroom: true }
            | Self::DecalSlots { backroom: true, .. }
            | Self::VinylGroups { backroom: true } => "CustomizeGenericTop_BACKROOM.fng",
            Self::Numbers => "Numbers.fng",
            Self::Choices { row: Row::Part(Part::Wheels), backroom: false } => "Rims.fng",
            Self::Choices { row: Row::Part(Part::Wheels), backroom: true } => "Rims_BACKROOM.fng",
            Self::Choices { row: Row::Part(Part::Spoiler), backroom: false } => "Spoilers.fng",
            Self::Choices { row: Row::Part(Part::Spoiler), backroom: true } => "Spoilers_BACKROOM.fng",
            Self::Choices { row: Row::Performance(_) | Row::Junkman(_), backroom: false } => "CustomizePerformance.fng",
            Self::Choices { row: Row::Performance(_) | Row::Junkman(_), backroom: true } => {
                "CustomizePerformance_BACKROOM.fng"
            }
            Self::Choices { row: Row::Decal(_, _) | Row::Number(_), backroom: false } => "Decals.fng",
            Self::Choices { row: Row::Decal(_, _) | Row::Number(_), backroom: true } => "Decals_BACKROOM.fng",
            Self::Choices { row: Row::Gauge, backroom: false } => "CustomHUD.fng",
            Self::Choices { row: Row::Gauge, backroom: true } => "CustomHUD_BACKROOM.fng",
            Self::GaugeColors { backroom: false } => "CustomHUDColor.fng",
            Self::GaugeColors { backroom: true } => "CustomHUDColor_BACKROOM.fng",
            Self::Choices { backroom: false, .. } => "CustomizeParts.fng",
            Self::Choices { backroom: true, .. } => "CustomizeParts_BACKROOM.fng",
            Self::Paint { backroom: false, .. } => "Paint.fng",
            Self::Paint { backroom: true, .. } => "Paint_BACKROOM.fng",
            Self::Tuning => "Pause_Performance_Tuning.fng",
            Self::Cart { backroom: true, .. } => "ShoppingCart_BACKROOM.fng",
            Self::Cart { quick: true, .. } => "ShoppingCart_QR.fng",
            Self::Cart { .. } => "ShoppingCart.fng",
            Self::Delete
            | Self::Leave
            | Self::Plate
            | Self::PlateDesign
            | Self::Checkout { .. }
            | Self::MaxPerformance
            | Self::InsufficientFunds => "Dialog.fng",
            Self::TuningDiscard | Self::TuningError => "InGameDialog.fng",
            Self::Showcase => "Showcase.fng",
        }
    }

    pub(super) fn backroom(self) -> bool {
        matches!(
            self,
            Self::Categories { backroom: true, .. }
                | Self::Choices { backroom: true, .. }
                | Self::Paint { backroom: true, .. }
                | Self::DecalZones { backroom: true }
                | Self::DecalSlots { backroom: true, .. }
                | Self::GaugeColors { backroom: true }
                | Self::Cart { backroom: true, .. }
        ) || matches!(self, Self::RimBrands { backroom: true } | Self::Rims { backroom: true, .. })
            || matches!(
                self,
                Self::Spoilers { backroom: true, .. }
                    | Self::VinylGroups { backroom: true }
                    | Self::Vinyls { backroom: true, .. }
            )
    }

    pub(super) fn title(self) -> String {
        match self {
            Self::Manager => "My Cars".into(),
            Self::AddCar => "Add New Car".into(),
            Self::Main => "Customize Car".into(),
            Self::Categories { section, .. } => format!("{section:?}"),
            Self::Choices { row, .. } => match row {
                Row::Part(Part::Body) => "Body Kits".into(),
                Row::Part(Part::Hood) => "Hoods".into(),
                Row::Part(Part::Roof) => "Roof Scoops".into(),
                Row::Performance(category) | Row::Junkman(category) => format!("{} Packages", category.label()),
                Row::Gauge => "Custom Gauges".into(),
                Row::Decal(_, _) => "Decals".into(),
                _ => row.label(),
            },
            Self::RimBrands { .. } | Self::Rims { .. } => "Rims".into(),
            Self::Spoilers { .. } => "Spoilers".into(),
            Self::VinylGroups { .. } | Self::Vinyls { .. } => "Vinyls".into(),
            Self::Paint { color: 0, .. } => "Paint".into(),
            Self::Paint { color: 1, .. } => "Rim Paint".into(),
            Self::Paint { color: 2..=4, .. } => "Vinyl Color".into(),
            Self::Paint { .. } => "Gauge Color".into(),
            Self::DecalZones { .. } => "Decal Location".into(),
            Self::DecalSlots { zone, .. } => nfsmw_data::car::customization::DecalZone::ALL[zone].label().into(),
            Self::GaugeColors { .. } => "Gauge Color".into(),
            Self::Numbers => "Numbers".into(),
            Self::Plate => "License Plate".into(),
            Self::PlateDesign => "Plate Design".into(),
            Self::Tuning => "Performance Tuning".into(),
            Self::Cart { .. } => "Shopping Cart".into(),
            Self::Checkout { .. } => "Checkout".into(),
            Self::MaxPerformance => "Max Performance".into(),
            Self::Delete => "Delete Car".into(),
            Self::Leave => "Leave Garage".into(),
            Self::TuningDiscard => "Performance Tuning".into(),
            Self::TuningError => "Attention".into(),
            Self::InsufficientFunds => "Attention".into(),
            Self::Showcase => "Showcase".into(),
        }
    }

    /// Installed language labels for the original category captions.
    pub(super) fn title_label(self, model: &Model) -> Option<u32> {
        use nfsmw_data::car::physics::PerformanceCategory as Hardware;
        Some(match self {
            Self::Manager => 0x3179_44de,
            Self::Main if model.backroom => 0x92fc_dbf0,
            Self::Main => 0x1f24_2e03,
            Self::Categories { section: Section::Parts, .. } => 0x055d_ce1a,
            Self::Categories { section: Section::Performance, .. } => 0xbaef_8282,
            Self::Categories { section: Section::Visual, .. } => 0xbfa7_d7c4,
            Self::Choices { row: Row::Part(Part::Body), .. } => 0x6134_c218,
            Self::Choices { row: Row::Part(Part::Hood), .. } => 0x04d4_a88d,
            Self::Choices { row: Row::Part(Part::Roof), .. } => 0x61e8_f83c,
            Self::Choices { row: Row::Part(Part::WindowTint), .. } => 0xd327_29a6,
            Self::Choices { row: Row::Gauge, .. } | Self::GaugeColors { .. } => 0x7898_0a6b,
            Self::Choices { row: Row::Performance(category) | Row::Junkman(category), .. } => match category {
                Hardware::Engine => 0x9853_d9a6,
                Hardware::Transmission => 0x29aa_74ba,
                Hardware::Chassis => 0x6e10_1aa7,
                Hardware::Nitrous => 0x4b73_7a18,
                Hardware::Tires => 0x05aa_9137,
                Hardware::Brakes => 0x9199_7ee8,
                Hardware::Induction if model.selected().is_some_and(|car| car.performance_details.turbo) => 0x05b1_255c,
                Hardware::Induction => 0xbb68_12bb,
            },
            Self::Spoilers { .. } | Self::Choices { row: Row::Part(Part::Spoiler), .. } => 0x94e7_3021,
            Self::RimBrands { .. } => 0xf868_eb0b,
            Self::Rims { .. } => 0xe167_f7c8,
            Self::VinylGroups { .. } | Self::Vinyls { .. } => 0xbfa5_2c55,
            Self::Paint { color: 0, .. } => 0x055d_a70c,
            Self::Paint { color: 1, .. } => 0xe126_ff53,
            Self::Paint { color: 2..=4, .. } => 0xd8ee_1a80,
            Self::DecalZones { .. } | Self::Choices { row: Row::Decal(_, _), .. } => 0x9559_80bc,
            Self::Numbers => 0x6857_e5ac,
            Self::Cart { .. } => 0x3997_28be,
            _ => return None,
        })
    }

    pub fn view(self) -> crate::garage::View {
        if matches!(self, Self::Manager | Self::AddCar) {
            return crate::garage::View::Cars;
        }
        if self.backroom() {
            return crate::garage::View::Backroom;
        }
        crate::garage::View::Customization
    }

    pub fn camera(self) -> &'static str {
        if self.backroom() {
            return cameras::backroom(self);
        }
        match self {
            Self::Manager => "mycarsmanager",
            Self::AddCar => "career_car_select",
            Self::Main => "customize_customizemain",
            Self::Categories { section: Section::Parts, .. } => "customize_customizecategory_2049",
            Self::Categories { section: Section::Performance, .. } => "customize_customizecategory_2050",
            Self::Categories { section: Section::Visual, .. } => "customize_customizecategory_2051",
            Self::Choices { row: Row::Part(Part::Body), .. } => "customize_customizeparts_257",
            Self::Choices { row: Row::Part(Part::Hood), .. } => "customize_customizeparts_260",
            Self::Choices { row: Row::Part(Part::Roof), .. } => "customize_customizeparts_261",
            Self::Choices { row: Row::Part(Part::Spoiler), .. } => "customize_spoilers_258",
            Self::Choices { row: Row::Performance(category) | Row::Junkman(category), .. } => {
                cameras::PERFORMANCE[category as usize]
            }
            Self::Spoilers { .. } => "customize_spoilers_258",
            Self::VinylGroups { .. } => "customize_customizegenerictop_770",
            Self::Vinyls { group: 0, .. } => "customize_customizeparts_1026",
            Self::Vinyls { group: 1, .. } => "customize_customizeparts_1027",
            Self::Vinyls { group: 2, .. } => "customize_customizeparts_1028",
            Self::Vinyls { group: 3, .. } => "customize_customizeparts_1029",
            Self::Vinyls { group: 4, .. } => "customize_customizeparts_1030",
            Self::Vinyls { group: 5, .. } => "customize_customizeparts_1031",
            Self::Vinyls { group: 6, .. } => "customize_customizeparts_1032",
            Self::Vinyls { group: 7, .. } => "customize_customizeparts_1033",
            Self::RimBrands { .. } => "customize_customizegenerictop_259",
            Self::Rims { .. } => "customize_rims",
            Self::Paint { color: 0, .. } => "customize_paint_769",
            Self::Paint { color: 1, .. } => "customize_paint_771",
            Self::Paint { .. } => "customize_paint_770",
            Self::Choices { row: Row::Part(Part::WindowTint), .. } => "customize_customizeparts_772",
            Self::Choices { row: Row::Gauge, .. } => "customize_customhud_775",
            Self::GaugeColors { .. } => "customize_customhudcolor",
            Self::DecalZones { .. } => "customize_customizegenerictop_773",
            Self::DecalSlots { zone, .. } => cameras::decal_slots(zone, false),
            Self::Choices { row: Row::Decal(zone, _), .. } => cameras::decals(zone, false),
            Self::Cart { quick: true, .. } => "customize_shoppingcart_qr",
            Self::Cart { .. } => "customize_shoppingcart",
            Self::Numbers => "customize_numbers_774",
            Self::Showcase => "showcase",
            _ => "customize_customizemain",
        }
    }
}
