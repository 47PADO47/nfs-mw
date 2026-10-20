//! The icons of each icon menu: what an icon does, its picture and its name (docs/specs/frontend-menus.md, section 2).
//! `icon_menu.rs` binds them to the package's objects and scrolls them.

use nfsmw_career::safehouse::menu::MenuItem;

use super::icon_menu::Kind;
use super::logic::{Category, PauseCtx, QuitTo};

/// What an icon does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    Career,
    Challenge,
    QuickRace,
    MyCars,
    /// The online and LAN entries have no screen here yet, so they are greyed out in the main menu.
    Online,
    LanPlay,
    /// Opens the alias manager's row (`alias_menu.rs`).
    AliasManager,
    Options,
    /// The alias manager's row: make, load, save and delete an alias (`alias_menu.rs`).
    AliasCreate,
    AliasLoad,
    AliasSave,
    AliasDelete,
    /// The career menu's row (`career_screens/career_menu.rs`).
    CareerResume,
    CareerNew,
    CareerLoad,
    /// The Quick Race menu's row (`race_menu.rs`): Custom Race and Split Screen are listed and greyed out.
    QuickPlay,
    CustomRace,
    SplitScreen,
    /// The key list (`controls_screen.rs`).
    Controls,
    /// The sensitivity and camera rows (`widget_menu.rs`).
    ControlOptions,
    EaTrax,
    Credits,
    Category(Category),
    Resume,
    /// The pause menu's items in a race: Resume Race and Restart.
    ResumeRace,
    RestartRace,
    /// Quit to the menu the game was started from; the name and the question follow [`QuitTo`].
    Quit(QuitTo),
    /// The safe house menu's items (`career_screens/safehouse_menu.rs`); Blacklist carries the stage it opens.
    SafeHouseResume,
    SafeHouseBlacklist(u8),
    SafeHouseCarSelect,
    SafeHouseRapSheet,
    SafeHouseSave,
}

/// The share of its slot the Discord mark is drawn at. The mark is a bundled 128-unit image drawn whole
/// (`WHOLE_IMAGE`), with no margin, so at full slot size it reads bigger than the package's pictures, which keep
/// a margin inside the slot. Drawing it at this share brings its glyph to about the package pictures' size
/// (docs/specs/frontend-menus.md, 2.2 Discord category). Only the size changes: the picture rectangle stays whole.
pub const DISCORD_DRAW_SCALE: f32 = 0.65;

/// How large an icon's picture is drawn, as a share of its slot.
pub fn draw_scale(icon: &Icon) -> f32 {
    match icon.texture {
        crate::ui::discord_icon::TEXTURE => DISCORD_DRAW_SCALE,
        _ => 1.0,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Icon {
    pub id: Id,
    /// The texture key of the icon, and the language labels of its name. Texture 0 draws no picture.
    pub texture: u32,
    pub name: u32,
    pub enabled: bool,
}

const fn icon(id: Id, texture: u32, name: u32, enabled: bool) -> Icon {
    Icon { id, texture, name, enabled }
}

/// The icons of a menu: `(texture, name label)` as the original screens give them.
pub fn icons(kind: Kind) -> Vec<Icon> {
    match kind {
        // Career and Quick Race both start free roam here. Challenge Series, My Cars, Online and LAN are greyed out.
        // Alias Manager (the original's profile manager, which it adds only when the memory card is enabled) opens
        // the save and load rows (`alias_options.rs`).
        // Online and LAN are not in this build's original main menu; the user asked for them, so they are listed
        // without a screen, with the install's own pictures (`Main_Icon_Online`, `Main_Icon_LAN`;
        // docs/specs/frontend-menus.md, 2.1). The main menu has eleven objects, so the row windows over them
        // (`icon_window.rs`).
        Kind::Main => vec![
            icon(Id::Career, 0x0370_4F3D, 0x5815_A2B5, true),
            icon(Id::Challenge, 0x9A96_2438, 0xCC8C_B746, false),
            icon(Id::QuickRace, 0x4E6F_BB02, 0x5402_0A7A, true),
            icon(Id::MyCars, 0xB0C4_6023, 0x1AFD_5BE6, false),
            icon(Id::Online, 0x2056_3D30, 0x74FB_90A8, false),
            icon(Id::LanPlay, 0xD13A_2E66, 0x4D78_216D, false),
            icon(Id::AliasManager, 0x6B30_3856, 0xBCB1_8F38, true),
            icon(Id::Options, 0x3058_FE37, 0x19A8_C0AF, true),
        ],
        // The original's option list (docs/specs/frontend-menus.md, 2.2): EA Trax opens the jukebox (`jukebox.rs`),
        // Credits the credits roll (`credits.rs`). Both are left out in the pause menu. Online is left out as well:
        // the original shows it only in online mode, which this build never is.
        // Controls opens the key list; Control Options (the sensitivity and camera rows) sits beside it. Discord
        // Rich Presence has its own icon (a bundled mark, `ui/discord_icon.rs`) and its toggles.
        Kind::Categories { pause, mods } => {
            let mut list = vec![
                icon(Id::Category(Category::Audio), 0xF37A_F144, 0xE76C_D783, true),
                icon(Id::Category(Category::Video), 0x8A00_6328, 0xE8E2_4508, true),
                icon(Id::Category(Category::Gameplay), 0x4DF9_8FB2, 0xD0CF_6EE1, true),
                icon(Id::Category(Category::Player), 0xD708_EFEF, 0xF760_EABE, true),
            ];
            list.push(icon(Id::Controls, 0xA04A_7B26, 0x04DC_6DB5, true));
            list.push(icon(Id::ControlOptions, crate::ui::input_icons::ATLAS, 0, true));
            list.push(icon(Id::Category(Category::Discord), crate::ui::discord_icon::TEXTURE, 0, true));
            // The Mods page is listed when a mod is listed (`mod_options.rs`) or has rows (`bb.menus`). Its picture is the mod manager's.
            if mods {
                list.push(icon(Id::Category(Category::Mods), crate::ui::mod_icon::TEXTURE, 0, true));
            }
            if !pause {
                list.push(icon(Id::EaTrax, 0xC52C_CBF6, 0xDCFB_6B36, true));
                list.push(icon(Id::Credits, 0x5100_9E20, 0x0905_101F, true));
            }
            list
        }
        Kind::Pause => pause_icons(PauseCtx::default()),
        // The alias manager: Create, Load, Save, Delete, each with its own picture
        // (docs/specs/career-parity-menus.md, section 3).
        Kind::AliasManager => vec![
            icon(Id::AliasCreate, 0x4379_8644, 0x5542_3473, true),
            icon(Id::AliasLoad, 0x2287_E063, 0x0018_ECFF, true),
            icon(Id::AliasSave, 0x228B_7E32, 0x001C_8ACE, true),
            icon(Id::AliasDelete, 0x0D90_35CE, 0x56B0_0632, true),
        ],
        // The Quick Race menu: Quick Play and Custom Race, and the Split Screen picture of the install.
        // Custom Race and Split Screen are not built yet, so they are greyed out (section 4).
        Kind::QuickRace => vec![
            icon(Id::QuickPlay, 0xE631_3967, 0xB5E8_F82F, true),
            icon(Id::CustomRace, 0x2A49_B5E2, 0x25BB_D4C3, false),
            icon(Id::SplitScreen, 0xF365_B5F5, 0x841D_518A, false),
        ],
        // The career menu and the safe house row depend on the career: see `career_icons` and `safehouse_icons`.
        Kind::Career | Kind::SafeHouse => Vec::new(),
    }
}

/// The picture of Resume Race, Resume Free Roam and Resume Career's pause counterpart (`INGAME_ICON_RESUME_RACE`).
const RESUME_TEXTURE: u32 = 0x12BB_5EA2;
const QUIT_TEXTURE: u32 = 0x4C9E_34E6;

/// The pause menu's items: resume, restart (a race only), options, quit. The original puts quit before options; the
/// rewrite keeps its own order, with the new items first (docs/specs/career-parity-menus.md, section 5).
pub fn pause_icons(ctx: PauseCtx) -> Vec<Icon> {
    let mut list = Vec::new();
    match ctx.race {
        true => {
            list.push(icon(Id::ResumeRace, RESUME_TEXTURE, 0xDED3_57E7, true));
            list.push(icon(Id::RestartRace, 0xB295_A6B6, 0xF893_AFA1, true));
        }
        false => list.push(icon(Id::Resume, RESUME_TEXTURE, 0x01BD_185C, true)),
    }
    list.push(icon(Id::Options, 0x520D_E4E3, 0x2B5A_03A8, true));
    let quit = match ctx.quit {
        QuitTo::Main => 0xE950_B7AF,
        QuitTo::QuickRace => 0x4349_998B,
        QuitTo::Career => 0x690E_9B7C,
    };
    list.push(icon(Id::Quit(ctx.quit), QUIT_TEXTURE, quit, true));
    list
}

/// The icons of the career menu: Resume Career when the profile's career has started and was not won,
/// Start New Career (Start Career for a profile whose career has not started), and Load. With no profile loaded the
/// row is Start New Career and Load (docs/specs/career-parity-menus.md, section 6).
pub fn career_icons(started: bool, won: bool) -> Vec<Icon> {
    const NEW_CAREER: u32 = 0xE735_3BE7;
    let mut list = Vec::new();
    if started && !won {
        list.push(icon(Id::CareerResume, 0xC1C0_89CE, 0xE072_DB21, true));
    }
    list.push(icon(Id::CareerNew, NEW_CAREER, 0x17E1_8F87, true));
    list.push(icon(Id::CareerLoad, 0x2287_E063, 0x0018_ECFF, true));
    list
}

/// The icons of the safe house menu, one per item in menu order (`career-safehouse-2.md`). The pictures are the
/// original's own (`MAIN_ICON_TOP_15`, `MAIN_ICON_CAR_SELECT`, `MAIN_ICON_RAP_SHEET`, `GENERIC_ICON_SAVE`, and the pause
/// menu's resume picture; docs/provenance/career-parity-menus.md). Before the world exists (`prelude`) Car Select is
/// greyed out.
pub fn safehouse_icons(items: &[MenuItem], prelude: bool) -> Vec<Icon> {
    items
        .iter()
        .map(|item| match *item {
            MenuItem::ResumeFreeRoam => icon(Id::SafeHouseResume, 0x12BB_5EA2, 0x01BD_185C, true),
            MenuItem::Blacklist(stage) => icon(Id::SafeHouseBlacklist(stage), 0x2C14_AC23, 0x80B9_FF9B, true),
            MenuItem::CarSelect => icon(Id::SafeHouseCarSelect, 0xC6A1_A6E0, 0x00D5_F627, !prelude),
            MenuItem::RapSheet => icon(Id::SafeHouseRapSheet, 0x2FD8_B206, 0xAC22_F27E, true),
            MenuItem::Save => icon(Id::SafeHouseSave, 0x228B_7E32, 0x001C_8ACE, true),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menus_have_the_original_icons() {
        let main = icons(Kind::Main);
        assert_eq!(main.len(), 8, "the original's main menu, Alias Manager included, plus LAN and Online");
        assert!(main[0].enabled && !main[1].enabled, "career works, the challenge series do not exist yet");
        let alias = main.iter().find(|i| i.id == Id::AliasManager).expect("Alias Manager");
        assert!(alias.enabled, "Alias Manager opens the save and load rows");
        assert!(
            main.iter()
                .filter(|i| ![Id::Career, Id::QuickRace, Id::Options, Id::AliasManager].contains(&i.id))
                .all(|i| !i.enabled)
        );
        let categories = icons(Kind::Categories { pause: false, mods: false });
        assert_eq!(
            categories.len(),
            9,
            "the offline option list, Control Options and Discord, which the windowed row scrolls over"
        );
        let controls = categories.iter().find(|i| i.id == Id::Controls).expect("the Controls icon");
        assert_eq!(controls.texture, 0xA04A_7B26, "OMController's picture (docs/specs/frontend-menus.md, 2.2)");
        let options = categories.iter().position(|i| i.id == Id::ControlOptions).expect("Control Options");
        assert_eq!(categories[options - 1].id, Id::Controls, "Control Options follows Controls");
        let player = categories.iter().find(|i| i.id == Id::Category(Category::Player)).expect("the Player category");
        assert_eq!((player.texture, player.name), (0xD708_EFEF, 0xF760_EABE), "the original's Player icon");
        assert!(player.enabled);
        let pause = icons(Kind::Categories { pause: true, mods: false });
        assert_eq!(pause.len(), 7, "Player is available; EA Trax and Credits remain absent in pause");
        assert_eq!(pause[3].id, Id::Category(Category::Player));
        assert!(pause[3].enabled);
        assert_eq!(icons(Kind::Pause)[0].name, 0x01BD_185C, "Resume Free Roam");
    }

    #[test]
    fn discord_has_its_own_category_icon_with_the_bundled_mark() {
        for pause in [false, true] {
            let categories = icons(Kind::Categories { pause, mods: false });
            let discord =
                categories.iter().find(|i| i.id == Id::Category(Category::Discord)).expect("the Discord category icon");
            assert_eq!(discord.texture, crate::ui::discord_icon::TEXTURE);
            assert!(discord.enabled);
        }
    }

    #[test]
    fn the_mods_page_is_listed_only_when_a_mod_has_rows() {
        for pause in [false, true] {
            let without = icons(Kind::Categories { pause, mods: false });
            assert!(without.iter().all(|i| i.id != Id::Category(Category::Mods)));
            let with = icons(Kind::Categories { pause, mods: true });
            let mods = with.iter().find(|i| i.id == Id::Category(Category::Mods)).expect("the Mods category icon");
            assert!(mods.enabled);
            assert_eq!(mods.texture, crate::ui::mod_icon::TEXTURE, "the mod manager's own picture");
            assert_eq!(with.len(), without.len() + 1);
        }
    }

    #[test]
    fn the_discord_mark_is_drawn_smaller_than_its_slot() {
        let categories = icons(Kind::Categories { pause: false, mods: false });
        let discord = categories.iter().find(|i| i.id == Id::Category(Category::Discord)).expect("Discord");
        assert!(draw_scale(discord) < 1.0, "the mark fills the slot unless it is drawn smaller");
        assert!((draw_scale(discord) - DISCORD_DRAW_SCALE).abs() < f32::EPSILON);
        let controls = categories.iter().find(|i| i.id == Id::Controls).expect("Controls");
        assert_eq!(draw_scale(controls), 1.0, "the package pictures keep their full slot");
    }

    #[test]
    fn ea_trax_and_credits_have_their_pictures_and_work() {
        let categories = icons(Kind::Categories { pause: false, mods: false });
        let ea_trax = categories.iter().find(|i| i.id == Id::EaTrax).expect("EA Trax");
        assert_eq!((ea_trax.texture, ea_trax.name), (0xC52C_CBF6, 0xDCFB_6B36), "the original's EA Trax picture");
        assert!(ea_trax.enabled);
        let credits = categories.iter().find(|i| i.id == Id::Credits).expect("Credits");
        assert_eq!((credits.texture, credits.name), (0x5100_9E20, 0x0905_101F), "the original's Credits picture");
        assert!(credits.enabled);
    }

    #[test]
    fn lan_and_online_are_listed_dimmed() {
        let main = icons(Kind::Main);
        let online = main.iter().find(|i| i.id == Id::Online).expect("Online");
        assert_eq!((online.texture, online.name), (0x2056_3D30, 0x74FB_90A8), "Main_Icon_Online, the label Online");
        assert!(!online.enabled);
        let lan = main.iter().find(|i| i.id == Id::LanPlay).expect("LAN");
        assert_eq!((lan.texture, lan.name), (0xD13A_2E66, 0x4D78_216D), "Main_Icon_LAN, the label lan");
        assert!(!lan.enabled);
    }
}
