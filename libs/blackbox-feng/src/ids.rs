//! Ids of the messages the FEng engine itself sends or understands: the hash of the upper-cased name
//! (`fe_hash_upper`). Packages and screens use many more; these are the ones the runtime needs.

pub const BUTTON_PRESSED: u32 = 0x0C40_7210;
pub const BUTTON_RELEASED: u32 = 0x936A_6A7F;
/// Sent to a button when it becomes the current one.
pub const BUTTON_HIGHLIGHT: u32 = 0xABC0_8912;
/// Sent to a button when it stops being the current one.
pub const BUTTON_UNHIGHLIGHT: u32 = 0x55D1_E635;
/// A script event with this id makes its target the current button.
pub const SET_CURRENT_BUTTON: u32 = 0x1B39_09AA;

pub const PAD_UP: u32 = 0x7261_9778;
pub const PAD_DOWN: u32 = 0x911C_0A4B;
pub const PAD_LEFT: u32 = 0x9120_409E;
pub const PAD_RIGHT: u32 = 0xB597_1BF1;
pub const PAD_UPLEFT: u32 = 0x6FFB_6F23;
pub const PAD_UPRIGHT: u32 = 0x6FD8_1B16;
pub const PAD_DOWNLEFT: u32 = 0x7989_1376;
pub const PAD_DOWNRIGHT: u32 = 0xAB1A_49C9;
pub const PAD_ACCEPT: u32 = 0x4064_15E3;
pub const PAD_BACK: u32 = 0x911A_B364;
pub const PAD_START: u32 = 0xB5AF_2461;
pub const PAD_LTRIGGER: u32 = 0x5073_EF13;
pub const PAD_RTRIGGER: u32 = 0xD9FE_EC59;
pub const PAD_LTRIGGER_HELD: u32 = 0x4473_15AF;
pub const PAD_RTRIGGER_HELD: u32 = 0x20AD_4EB5;
pub const PAD_ACCEPT_RELEASED: u32 = 0xC12E_9E27;
pub const PAD_BACK_RELEASED: u32 = 0xC2F8_FCC8;
pub const PAD_START_RELEASED: u32 = 0xEBFC_DA65;
pub const PAD_LTRIGGER_RELEASED: u32 = 0x091D_CD57;
pub const PAD_RTRIGGER_RELEASED: u32 = 0x7A39_195D;
/// `PAD_BUTTON0` .. `PAD_BUTTON9` are consecutive.
pub const PAD_BUTTON0: u32 = 0xC519_BFBF;

/// The released message of `PAD_BUTTON0` .. `PAD_BUTTON9`.
pub const PAD_BUTTON_RELEASED: [u32; 10] = [
    0xD467_1F83,
    0xD871_B0A4,
    0xDC7C_41C5,
    0xE086_D2E6,
    0xE491_6407,
    0xE89B_F528,
    0xECA6_8649,
    0xF0B1_176A,
    0xF4BB_A88B,
    0xF8C6_39AC,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::fe_hash_upper;

    #[test]
    fn ids_are_the_hashes_of_their_names() {
        let names = [
            (BUTTON_PRESSED, "BUTTON_PRESSED"),
            (BUTTON_RELEASED, "BUTTON_RELEASED"),
            (BUTTON_HIGHLIGHT, "BUTTON_HIGHLIGHT"),
            (BUTTON_UNHIGHLIGHT, "BUTTON_UNHIGHLIGHT"),
            (PAD_UP, "PAD_UP"),
            (PAD_DOWN, "PAD_DOWN"),
            (PAD_LEFT, "PAD_LEFT"),
            (PAD_RIGHT, "PAD_RIGHT"),
            (PAD_UPLEFT, "PAD_UPLEFT"),
            (PAD_UPRIGHT, "PAD_UPRIGHT"),
            (PAD_DOWNLEFT, "PAD_DOWNLEFT"),
            (PAD_DOWNRIGHT, "PAD_DOWNRIGHT"),
            (PAD_ACCEPT, "PAD_ACCEPT"),
            (PAD_BACK, "PAD_BACK"),
            (PAD_START, "PAD_START"),
            (PAD_LTRIGGER, "PAD_LTRIGGER"),
            (PAD_RTRIGGER, "PAD_RTRIGGER"),
            (PAD_LTRIGGER_HELD, "PAD_LTRIGGER_HELD"),
            (PAD_RTRIGGER_HELD, "PAD_RTRIGGER_HELD"),
            (PAD_ACCEPT_RELEASED, "PAD_ACCEPT_RELEASED"),
            (PAD_BACK_RELEASED, "PAD_BACK_RELEASED"),
            (PAD_START_RELEASED, "PAD_START_RELEASED"),
            (PAD_LTRIGGER_RELEASED, "PAD_LTRIGGER_RELEASED"),
            (PAD_RTRIGGER_RELEASED, "PAD_RTRIGGER_RELEASED"),
            (PAD_BUTTON0, "PAD_BUTTON0"),
        ];
        for (id, name) in names {
            assert_eq!(id, fe_hash_upper(name), "{name}");
        }
        for (i, id) in PAD_BUTTON_RELEASED.iter().enumerate() {
            assert_eq!(*id, fe_hash_upper(&format!("PAD_BUTTON{i}_RELEASED")));
            assert_eq!(PAD_BUTTON0 + i as u32, fe_hash_upper(&format!("PAD_BUTTON{i}")));
        }
    }
}
