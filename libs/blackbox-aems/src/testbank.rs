//! A tiny module bank built in memory, for the examples and the tests.

fn put32(v: &mut [u8], at: usize, value: u32) {
    v[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// A bank of one module with a class-data node of three words (play control, sample select, volume) and one player
/// of two inputs (pitch, volume) over a group of two bank sounds (7 and 8). The class is named `TEST`.
///
/// The module's code copies the three words into the player and runs it.
#[doc(hidden)]
pub fn minimal_bank() -> Vec<u8> {
    const MODULE: usize = 0x5C;
    let code: &[u8] = &[
        0x56, // push esi
        0x8B, 0x74, 0x24, 0x08, // mov esi, [esp+8]
        0x56, 0xE8, 1, 0, 0, 0, 0x83, 0xC4, 0x04, // class data: eax = word 0
        0x89, 0x46, 0x38, // player play control
        0x8B, 0x46, 0x18, 0x89, 0x46, 0x34, // word 1 to the sample select
        0x8B, 0x46, 0x1C, 0x89, 0x46, 0x50, // word 2 to the volume input
        0x83, 0xC6, 0x20, // esi to the player
        0x56, 0xE8, 27, 0, 0, 0, 0x83, 0xC4, 0x04, // the player
        0x5E, 0xC3, // pop esi; ret
    ];
    let code_at = MODULE + 0x3C + 4;
    let data_at = (code_at + code.len()).next_multiple_of(4);
    let data_size = 0x18 + 0x20 + 0x3C;
    let group_at = data_at + data_size;
    let table_at = group_at + 4 + 24;
    let name_at = table_at + 4 + 12;
    let total = (name_at + 4 + 5).next_multiple_of(4);

    let mut f = vec![0u8; total];
    f[..4].copy_from_slice(b"ABKC");
    f[4] = 1;
    f[0x0A] = 1;
    put32(&mut f, 0x14, total as u32);
    put32(&mut f, 0x18, total as u32);
    put32(&mut f, 0x1C, MODULE as u32);
    put32(&mut f, 0x20, total as u32);
    put32(&mut f, 0x38, table_at as u32);

    put32(&mut f, MODULE, 1);
    f[MODULE + 0x1E] = 4;
    f[MODULE + 0x24] = 1;
    f[MODULE + 0x26] = 1;
    put32(&mut f, MODULE + 0x28, code_at as u32);
    put32(&mut f, MODULE + 0x2C, data_at as u32);
    put32(&mut f, MODULE + 0x30, data_size as u32);
    put32(&mut f, MODULE + 0x34, data_size as u32);
    put32(&mut f, MODULE + 0x3C, 0x38);
    f[code_at..code_at + code.len()].copy_from_slice(code);

    // Data: 0x18 header, the class-data node (three words), the player node.
    let class = data_at + 0x18;
    f[class + 0x10] = 3;
    let player = class + 0x20;
    put32(&mut f, player + 4, group_at as u32);
    f[player + 0xE] = 2;
    f[player + 0xF] = 1;
    f[player + 0x10] = 0xFF;
    f[player + 0x1C] = 0;
    put32(&mut f, player + 0x1C + 8, 4096);
    f[player + 0x28] = 2;

    put32(&mut f, group_at, 2);
    put32(&mut f, group_at + 4 + 4, 7);
    put32(&mut f, group_at + 4 + 12 + 4, 8);

    put32(&mut f, table_at, 1);
    put32(&mut f, table_at + 4, (MODULE + 4) as u32);
    put32(&mut f, table_at + 8, name_at as u32);
    f[table_at + 12] = 1;
    f[name_at + 4..name_at + 8].copy_from_slice(b"TEST");
    f
}
