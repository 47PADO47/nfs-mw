//! CRC-32 (IEEE 802.3, the zlib and PNG checksum: reflected polynomial `EDB88320`, initial and final value all ones),
//! with a table built at compile time.

const POLYNOMIAL: u32 = 0xEDB8_8320;

const TABLE: [u32; 256] = table();

const fn table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut index = 0;
    while index < 256 {
        let mut crc = index as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = match crc & 1 {
                0 => crc >> 1,
                _ => POLYNOMIAL ^ (crc >> 1),
            };
            bit += 1;
        }
        table[index] = crc;
        index += 1;
    }
    table
}

/// The CRC-32 of `bytes`.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc = TABLE[((crc ^ u32::from(*byte)) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}
