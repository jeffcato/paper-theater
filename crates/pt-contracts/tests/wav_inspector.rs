#[path = "support/wav_cases.rs"]
mod wav;
use pt_contracts::*;
use std::io::{Cursor, Read, Seek, SeekFrom};
#[test]
fn native_and_delivery_admission_preserves_known_formats() {
    for c in wav::cases() {
        let mut cursor = Cursor::new(c.bytes.as_slice());
        let native = inspect_pcm16_wav(&mut cursor, c.bytes.len() as u64);
        assert_eq!(native.is_ok(), c.native, "{}", c.id);
        if let Ok(info) = native {
            info.validate().unwrap();
            assert_eq!(cursor.position(), 0);
            assert_eq!(info.frame_count, 1);
        }
        assert_eq!(
            inspect_delivery_wav(&mut Cursor::new(c.bytes.as_slice()), c.bytes.len() as u64)
                .is_ok(),
            c.delivery,
            "{}",
            c.id
        );
    }
}
struct Counted<R> {
    inner: R,
    bytes: u64,
}
impl<R: Read> Read for Counted<R> {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(b)?;
        self.bytes += n as u64;
        Ok(n)
    }
}
impl<R: Seek> Seek for Counted<R> {
    fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(p)
    }
}
#[test]
fn exact_and_over_limit_admission_is_bounded_without_a_master_sized_buffer() {
    #[allow(dead_code)]
    #[path = "support/acceptance_fixture.rs"]
    mod fixture;
    use std::io::Write;
    let f = fixture::Fixture::new();
    let path = f.root.join("sparse.wav");
    for (size, channels, rate, valid) in [
        (MAX_NATIVE_PCM16_WAV_BYTES, 1, 24000, true),
        (MAX_NATIVE_PCM16_WAV_BYTES + 2, 1, 24000, false),
        (MAX_PCM16_WAV_BYTES, 2, 48000, true),
        (MAX_PCM16_WAV_BYTES + 4, 2, 48000, false),
    ] {
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(&wav::header(size, channels, rate)).unwrap();
        file.set_len(size).unwrap();
        drop(file);
        let mut r = Counted {
            inner: std::fs::File::open(&path).unwrap(),
            bytes: 0,
        };
        let info = inspect_pcm16_wav(&mut r, size);
        assert_eq!(info.is_ok(), valid, "size {size}");
        assert!(r.bytes < 16384, "inspector read PCM payload");
        if let Ok(i) = info {
            assert_eq!(i.frame_count, (size - 44) / (channels as u64 * 2));
        }
    }
    let mut bytes = wav::wave(2, 48000, 1);
    bytes.truncate(44);
    assert!(inspect_pcm16_wav(&mut Cursor::new(&bytes), 48).is_err());
}
