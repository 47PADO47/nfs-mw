//! Zone kinds and the per-frame entry permissions (`docs/specs/career-safehouse.md` sections 2 and 3.1).

use super::context::FrontEndContext;

/// The kind of a menu-gate zone. The three are told apart only by the `ZoneType` string of the zone collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZoneKind {
    /// `safehouse`: opens the safe house menu.
    SafeHouse,
    /// `carlot`: opens Car Select on the car lot platform.
    CarLot,
    /// `customshop`: opens the customization screen.
    CustomShop,
}

/// The screen and platform a zone opens, with the context bits the opener adds to the career bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneOpen {
    /// The front-end platform id (2 safe house, 3 custom shop, 5 car lot).
    pub platform: u8,
    /// The screen package name.
    pub screen: &'static str,
    /// Context bits set besides career (bit 0).
    pub extra_context: FrontEndContext,
}

impl ZoneKind {
    /// The kind of a `ZoneType` string. The compare is character by character and case sensitive (the install
    /// strings are lower case); any other string, including none, is `None`: no message and no screen.
    pub fn from_zone_type(zone_type: &str) -> Option<ZoneKind> {
        match zone_type {
            "safehouse" => Some(ZoneKind::SafeHouse),
            "carlot" => Some(ZoneKind::CarLot),
            "customshop" => Some(ZoneKind::CustomShop),
            _ => None,
        }
    }

    /// The `ZoneType` string of the kind.
    pub const fn zone_type(self) -> &'static str {
        match self {
            ZoneKind::SafeHouse => "safehouse",
            ZoneKind::CarLot => "carlot",
            ZoneKind::CustomShop => "customshop",
        }
    }

    /// The FE image id of the prompt icon.
    pub const fn icon_image(self) -> u32 {
        match self {
            ZoneKind::CarLot => 0x4EAE_E18B,
            ZoneKind::SafeHouse => 0x0ED3_9F69,
            ZoneKind::CustomShop => 0x0CF0_7089,
        }
    }

    /// What opening the zone does. A custom shop needs a current car record (`has_current_car`): without one the
    /// opener returns and nothing opens. The other kinds always open.
    pub fn open(self, has_current_car: bool) -> Option<ZoneOpen> {
        match self {
            ZoneKind::SafeHouse => {
                Some(ZoneOpen { platform: 2, screen: "MainMenu_Sub.fng", extra_context: FrontEndContext(0) })
            }
            ZoneKind::CarLot => {
                Some(ZoneOpen { platform: 5, screen: "Car_Select.fng", extra_context: FrontEndContext::CAR_LOT })
            }
            ZoneKind::CustomShop if has_current_car => {
                Some(ZoneOpen { platform: 3, screen: "CustomizeMain.fng", extra_context: FrontEndContext::CUSTOMIZE })
            }
            ZoneKind::CustomShop => None,
        }
    }
}

/// The state of the player pursuit as the permission rule sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PursuitPhase {
    /// The player car has no pursuit object.
    None,
    /// A pursuit exists and is not cooling down.
    Active,
    /// A pursuit exists and its status is 2 (cool-down).
    CoolingDown,
}

/// The two permissions the manager recomputes every frame in free roam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryPermissions {
    /// Car lots and custom shops (the manager bytes `+0x2CE` and `+0x2D0`, which hold the same expression): no
    /// event underway and no pursuit.
    pub menu_gates: bool,
    /// Safe houses (`+0x2CF`, the script hook `AllowEngageSafehouse`): no event underway and either no pursuit or
    /// one in cool-down.
    pub safe_house: bool,
}

impl EntryPermissions {
    /// The recomputation at the end of every race status update.
    pub const fn compute(event_underway: bool, pursuit: PursuitPhase) -> EntryPermissions {
        let in_pursuit = !matches!(pursuit, PursuitPhase::None);
        let cooling_down = matches!(pursuit, PursuitPhase::CoolingDown);
        EntryPermissions {
            menu_gates: !event_underway && !in_pursuit,
            safe_house: !event_underway && (!in_pursuit || cooling_down),
        }
    }

    /// True when the Lua graph may show the prompt of a zone of this kind.
    pub const fn allows(&self, kind: ZoneKind) -> bool {
        match kind {
            ZoneKind::SafeHouse => self.safe_house,
            ZoneKind::CarLot | ZoneKind::CustomShop => self.menu_gates,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_types_are_matched_exactly() {
        assert_eq!(ZoneKind::from_zone_type("safehouse"), Some(ZoneKind::SafeHouse));
        assert_eq!(ZoneKind::from_zone_type("carlot"), Some(ZoneKind::CarLot));
        assert_eq!(ZoneKind::from_zone_type("customshop"), Some(ZoneKind::CustomShop));
        assert_eq!(ZoneKind::from_zone_type("SafeHouse"), None, "case sensitive");
        assert_eq!(ZoneKind::from_zone_type(""), None);
        assert_eq!(ZoneKind::from_zone_type("safehouse "), None);
        assert_eq!(ZoneKind::from_zone_type("speedtrap"), None);
        for kind in [ZoneKind::SafeHouse, ZoneKind::CarLot, ZoneKind::CustomShop] {
            assert_eq!(ZoneKind::from_zone_type(kind.zone_type()), Some(kind));
        }
    }

    #[test]
    fn each_kind_opens_its_screen_on_its_platform() {
        let safe = ZoneKind::SafeHouse.open(false).unwrap();
        assert_eq!((safe.platform, safe.screen, safe.extra_context.0), (2, "MainMenu_Sub.fng", 0));
        let lot = ZoneKind::CarLot.open(false).unwrap();
        assert_eq!((lot.platform, lot.screen, lot.extra_context), (5, "Car_Select.fng", FrontEndContext::CAR_LOT));
        let shop = ZoneKind::CustomShop.open(true).unwrap();
        assert_eq!(
            (shop.platform, shop.screen, shop.extra_context),
            (3, "CustomizeMain.fng", FrontEndContext::CUSTOMIZE)
        );
    }

    #[test]
    fn a_custom_shop_needs_a_current_car() {
        assert_eq!(ZoneKind::CustomShop.open(false), None);
    }

    #[test]
    fn the_icons_are_the_documented_image_ids() {
        assert_eq!(ZoneKind::CarLot.icon_image(), 0x4EAEE18B);
        assert_eq!(ZoneKind::SafeHouse.icon_image(), 0x0ED39F69);
        assert_eq!(ZoneKind::CustomShop.icon_image(), 0x0CF07089);
    }

    #[test]
    fn permissions_follow_event_and_pursuit() {
        use PursuitPhase::*;
        let p = |event, pursuit| EntryPermissions::compute(event, pursuit);
        assert_eq!(p(false, None), EntryPermissions { menu_gates: true, safe_house: true });
        assert_eq!(p(false, Active), EntryPermissions { menu_gates: false, safe_house: false });
        assert_eq!(p(false, CoolingDown), EntryPermissions { menu_gates: false, safe_house: true });
        for pursuit in [None, Active, CoolingDown] {
            assert_eq!(
                p(true, pursuit),
                EntryPermissions { menu_gates: false, safe_house: false },
                "an event blocks all"
            );
        }
    }

    #[test]
    fn prompts_follow_the_kind() {
        let cooling = EntryPermissions::compute(false, PursuitPhase::CoolingDown);
        assert!(cooling.allows(ZoneKind::SafeHouse));
        assert!(!cooling.allows(ZoneKind::CarLot) && !cooling.allows(ZoneKind::CustomShop));
        let free = EntryPermissions::compute(false, PursuitPhase::None);
        assert!(free.allows(ZoneKind::CarLot) && free.allows(ZoneKind::CustomShop) && free.allows(ZoneKind::SafeHouse));
    }
}
