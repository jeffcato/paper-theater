#[derive(Clone)]
pub struct Case {
    pub id: &'static str,
    pub bytes: Vec<u8>,
    pub native: bool,
    pub delivery: bool,
}
pub fn header(size: u64, channels: u16, rate: u32) -> Vec<u8> {
    let mut b = vec![];
    b.extend(b"RIFF");
    b.extend(((size - 8) as u32).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(channels.to_le_bytes());
    b.extend(rate.to_le_bytes());
    b.extend((rate * channels as u32 * 2).to_le_bytes());
    b.extend((channels * 2).to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(((size - 44) as u32).to_le_bytes());
    b
}
pub fn wave(channels: u16, rate: u32, frames: usize) -> Vec<u8> {
    assert!(frames <= 64);
    let mut b = header(44 + (frames * channels as usize * 2) as u64, channels, rate);
    for i in 0..frames * channels as usize {
        b.extend((if i % 2 == 0 { 1i16 } else { -1i16 }).to_le_bytes());
    }
    b
}
fn extent(b: &mut [u8]) {
    let n = b.len() as u32 - 8;
    b[4..8].copy_from_slice(&n.to_le_bytes());
}
pub fn cases() -> Vec<Case> {
    let base = wave(2, 48000, 1);
    let mut out = vec![];
    let mut add = |id, bytes, native, delivery| {
        out.push(Case {
            id,
            bytes,
            native,
            delivery,
        })
    };
    add("stereo-48k-pcm16", base.clone(), true, true);
    add("native-mono-24k", wave(1, 24000, 1), true, false);
    add("native-mono-48k", wave(1, 48000, 1), true, false);
    add("native-stereo-24k", wave(2, 24000, 1), true, false);
    let mut b = base.clone();
    b.splice(36..36, [0, 0]);
    b[16..20].copy_from_slice(&18u32.to_le_bytes());
    extent(&mut b);
    add("fmt18-zero-extension", b.clone(), true, true);
    b[36] = 1;
    add("fmt18-invalid-extension", b, false, false);
    let mut b = base.clone();
    b.splice(12..12, [b'J', b'U', b'N', b'K', 1, 0, 0, 0, 8, 7]);
    extent(&mut b);
    add("odd-ancillary-padding", b, true, true);
    let mut b = base.clone();
    b.extend(b"JUNK\0\0\0\0");
    extent(&mut b);
    add("ancillary-after-data", b, true, true);
    for (id, offset, value) in [
        ("bad-riff", 0, b'X'),
        ("bad-wave", 8, b'X'),
        ("wrong-riff-extent", 4, 0),
        ("wrong-byte-rate", 28, 1),
        ("wrong-block-align", 32, 1),
        ("wrong-bits", 34, 8),
        ("unsupported-encoding", 20, 3),
        ("extensible-encoding", 20, 254),
    ] {
        let mut b = base.clone();
        b[offset] = value;
        add(id, b, false, false);
    }
    add("mime-only-garbage", vec![b'X'; 48], false, false);
    add("unsupported-channels", wave(3, 48000, 1), false, false);
    add("unsupported-rate", wave(2, 4000, 1), false, false);
    let mut b = wave(2, 48000, 2);
    b[20..22].copy_from_slice(&3u16.to_le_bytes());
    b[34..36].copy_from_slice(&32u16.to_le_bytes());
    b[32..34].copy_from_slice(&8u16.to_le_bytes());
    b[28..32].copy_from_slice(&384000u32.to_le_bytes());
    add("float32-wave", b, false, false);
    let mut b = base.clone();
    b.pop();
    extent(&mut b);
    add("truncated-data", b, false, false);
    let mut b = base.clone();
    b[40..44].copy_from_slice(&8u32.to_le_bytes());
    add("data-extent-beyond-file", b, false, false);
    let mut b = base.clone();
    b.push(0);
    add("trailing-outside-riff", b, false, false);
    let mut b = base.clone();
    b.extend(&base[36..]);
    extent(&mut b);
    add("duplicate-data", b, false, false);
    let mut b = base.clone();
    b.extend(&base[12..36]);
    extent(&mut b);
    add("duplicate-fmt-after-data", b, false, false);
    let mut b = base[..12].to_vec();
    b.extend(&base[36..]);
    b.extend(&base[12..36]);
    extent(&mut b);
    add("data-before-fmt", b, false, false);
    add("empty-data", wave(2, 48000, 0), false, false);
    let mut b = base.clone();
    b[40..44].copy_from_slice(&3u32.to_le_bytes());
    add("misaligned-data", b, false, false);
    let mut b = base.clone();
    b.splice(12..12, [b'J', b'U', b'N', b'K', 255, 255, 255, 255]);
    extent(&mut b);
    add("oversized-chunk", b, false, false);
    let mut b = base.clone();
    b.extend([0, 0, 0, 0]);
    extent(&mut b);
    add("incomplete-chunk-header", b, false, false);
    for (id, count, valid) in [
        ("chunk-limit", 1022, true),
        ("too-many-chunks", 1023, false),
    ] {
        let mut b = base[..12].to_vec();
        for _ in 0..count {
            b.extend(b"JUNK\0\0\0\0");
        }
        b.extend(&base[12..]);
        extent(&mut b);
        add(id, b, valid, valid);
    }
    out
}
