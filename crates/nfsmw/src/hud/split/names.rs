//! Object names of the split-screen packages, by FEng name hash (`nfsmw dump-screen HUD_Player1.fng`; the names that
//! the single-player HUD already uses are the same objects). Several names are shared by the four packages, others by two.

/// The nitrous group (the icon and the backing) and the bar next to it, a multi image whose mask turns.
pub(super) const NITROUS_GROUP: u32 = 0x87C3_8E97;
pub(super) const NITROUS_BAR: u32 = 0xEDFB_6D37;
pub(super) const NITROUS_ICON: u32 = 0x27DD_F583;
/// The rpm bar: an image whose width follows the rpm, with its left edge fixed.
pub(super) const RPM_BAR: &str = "3rdPersonNeedle";
/// The `RaceInformationGroup`: the place, the clock and their backings (a race package), the gear and the shift backing
/// too (a drag package).
pub(super) const INFO_GROUP: u32 = 0xB1C5_ED0D;
/// The clock and its icon at the top of the half, and the glow that flashes behind the icon when time is added (not bound).
pub(super) const CLOCK: &str = "CurrentLapTime";
pub(super) const CLOCK_ICON: u32 = 0x6DD7_54EC;
pub(super) const TIME_GLOW: u32 = 0xCAEC_9D04;
/// The generic message line and the countdown step; the lines' groups differ between the packages.
pub(super) const GENERIC_MESSAGE: u32 = 0x32A7_A521;
pub(super) const COUNTDOWN: &str = "321_GO_GROUP";
/// The wrong-way icon (no state feeds it).
pub(super) const WRONG_WAY: u32 = 0xC82F_A200;
/// The tollbooth milestone group of the race information (not bound), and the drag package's engine heat light.
pub(super) const MILESTONE: u32 = 0x1A09_1045;
pub(super) const ENGINE_HEAT: u32 = 0x88A5_6517;
/// The redline script of the rpm bar, and the shift light's.
pub(super) const SCRIPT_REDLINE: u32 = 0x61D3_0442;
/// The rule between the halves: a `BasePoly` 1200 units wide in the first player's package (the guid is the same in
/// both packages that have it).
pub(super) const DIVIDER_GUID: u32 = 0xF6DD2;
/// The resource of the speed-trap icons on the minimap layer of the first player's race package (not bound).
pub(super) const SPEED_TRAP_ICON: &str = "Minimap_Icon_Speedtrap";
/// The second player's arrow on the shared minimap.
pub(super) const MAP_ARROW_2: &str = "PLAYERCARINDICATOR2";
