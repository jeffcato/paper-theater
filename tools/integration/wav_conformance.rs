#[path = "../../crates/pt-contracts/tests/support/wav_cases.rs"]
mod wav;
use pt_contracts::*;
use std::io::Cursor;
// Read-only R11 conformance scaffold: production-side test only; contracts never depend on runtime.
#[allow(dead_code)]
mod store {
    #[derive(Debug)]
    pub struct Error {
        pub code: String,
        pub message: String,
    }
    impl Error {
        pub fn new(code: &str, message: impl Into<String>) -> Self {
            Self {
                code: code.into(),
                message: message.into(),
            }
        }
    }
    impl From<std::io::Error> for Error {
        fn from(e: std::io::Error) -> Self {
            Self::new("io", e.to_string())
        }
    }
    pub type Result<T> = std::result::Result<T, Error>;
    pub fn fail<T>(code: &str, message: &str) -> Result<T> {
        Err(Error::new(code, message))
    }
    pub fn valid_hash(s: &str) -> bool {
        pt_contracts::valid_digest(s)
    }
}
#[allow(dead_code)]
#[path = "../../crates/pt-production/src/runtime/media.rs"]
mod r11;
#[test]
fn shared_native_inspection_matches_r11_and_delivery_is_strict_at_every_size() {
    for c in wav::cases() {
        let original = r11::validate_artifact_bytes(&c.bytes, "audio/wav");
        let mut cursor = Cursor::new(c.bytes.as_slice());
        let native = inspect_pcm16_wav(&mut cursor, c.bytes.len() as u64);
        assert_eq!(native.is_ok(), c.native, "{}", c.id);
        assert_eq!(native.is_ok(), original.is_ok(), "R11 drift: {}", c.id);
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
