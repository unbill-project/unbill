use std::fmt;
use std::str::FromStr;

/// A cryptographically random invitation token — 32 bytes, lowercase hex-encoded (64 chars).
///
/// Held in `UnbillService` memory only. Never persisted or synced.
/// See DESIGN.md §6.3 for the invitation flow.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct InviteToken(String);

impl InviteToken {
    /// Generate a new token from 32 OS-random bytes.
    pub fn generate() -> Self {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        Self(bytes.iter().map(|b| format!("{b:02x}")).collect())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InviteToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Error returned when parsing an invalid invite token string.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidInviteToken;

impl fmt::Display for InvalidInviteToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid invite token: expected 64 lowercase hex characters"
        )
    }
}

impl FromStr for InviteToken {
    type Err = InvalidInviteToken;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            Ok(Self(s.to_owned()))
        } else {
            Err(InvalidInviteToken)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_is_64_hex_chars() {
        let tok = InviteToken::generate();
        let s = tok.to_string();
        assert_eq!(s.len(), 64);
        assert!(s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
    }

    #[test]
    fn test_generate_is_unique() {
        assert_ne!(InviteToken::generate(), InviteToken::generate());
    }

    #[test]
    fn test_round_trip_from_str() {
        let tok = InviteToken::generate();
        let parsed: InviteToken = tok.to_string().parse().unwrap();
        assert_eq!(tok, parsed);
    }

    #[test]
    fn test_rejects_wrong_length() {
        assert!("abc".parse::<InviteToken>().is_err());
    }

    #[test]
    fn test_rejects_uppercase_hex() {
        let upper = "A".repeat(64);
        assert!(upper.parse::<InviteToken>().is_err());
    }

    #[test]
    fn test_rejects_non_hex() {
        let bad = "z".repeat(64);
        assert!(bad.parse::<InviteToken>().is_err());
    }
}
