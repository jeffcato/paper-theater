pub fn header(total: u64) -> [u8; 44] {
    let data = u32::try_from(total - 44).unwrap();
    let mut b = [0; 44];
    b[..4].copy_from_slice(b"RIFF");
    b[4..8].copy_from_slice(&(data + 36).to_le_bytes());
    b[8..12].copy_from_slice(b"WAVE");
    b[12..16].copy_from_slice(b"fmt ");
    b[16..20].copy_from_slice(&16u32.to_le_bytes());
    b[20..22].copy_from_slice(&1u16.to_le_bytes());
    b[22..24].copy_from_slice(&2u16.to_le_bytes());
    b[24..28].copy_from_slice(&48000u32.to_le_bytes());
    b[28..32].copy_from_slice(&192000u32.to_le_bytes());
    b[32..34].copy_from_slice(&4u16.to_le_bytes());
    b[34..36].copy_from_slice(&16u16.to_le_bytes());
    b[36..40].copy_from_slice(b"data");
    b[40..44].copy_from_slice(&data.to_le_bytes());
    b
}
pub fn chapter() -> Vec<u8> {
    let mut b = vec![0; 34_560_044];
    let n = b.len();
    b[..44].copy_from_slice(&header(n as u64));
    b[n - 4..].copy_from_slice(&[1, 0, 2, 0]);
    b
}
